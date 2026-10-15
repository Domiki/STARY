use std::ops::Range;

use rayon::prelude::*;
use stary_core::{COLORS, LinearRgbF32};
use stary_mask::Mask;
use stary_register::Transform;
use stary_stack::{Kernel, Matching, Rejection, SourcePlane, combine, warp_rows};

use super::prepare::{PrepareContext, demosaiced_planes};
use crate::{
    control::{Cancel, Progress, Stage},
    decode_stream::for_each_processed,
    error::PipelineError,
    masking::{Rows, blend_rows, mix_f32},
    planes::Planes,
};

const FULL_SCALE: f32 = 65535.0;

/// A frame that takes part in the result.
pub struct Member {
    pub index: usize,
    /// Maps coordinates of the result onto coordinates of this frame.
    pub to_source: Transform,
    pub matching: [Matching; COLORS],
    pub planes: Option<Planes>,
}

pub struct Integration<'a> {
    pub width: usize,
    pub height: usize,
    pub kernel: Kernel,
    pub rejection: Rejection,
    pub strip_rows: usize,
    pub workers: usize,
    pub context: &'a PrepareContext<'a>,
    /// When set, the frames are also combined without being moved, and that picture is put under
    /// the result where this mask says ground.
    pub ground: Option<&'a Mask>,
}

struct Strip {
    rows: Range<usize>,
    members: usize,
    width: usize,
    samples: Vec<f32>,
}

impl Strip {
    fn new(rows: Range<usize>, members: usize, width: usize) -> Self {
        let len = COLORS * members * rows.len() * width;
        Self {
            rows,
            members,
            width,
            samples: vec![f32::NAN; len],
        }
    }

    fn region(&mut self, channel: usize, member: usize) -> &mut [f32] {
        let len = self.rows.len() * self.width;
        let start = (channel * self.members + member) * len;
        &mut self.samples[start..start + len]
    }

    fn fill(&mut self, member: usize, frame: &Member, planes: &Planes, kernel: Kernel) {
        for channel in 0..COLORS {
            let source = SourcePlane {
                width: planes.width,
                height: planes.height,
                data: &planes.data[channel],
            };
            let (rows, width) = (self.rows.clone(), self.width);
            let region = self.region(channel, member);
            warp_rows(&source, &frame.to_source, kernel, rows, width, region);
            let matching = frame.matching[channel];
            region
                .par_iter_mut()
                .filter(|value| value.is_finite())
                .for_each(|value| *value = matching.apply(*value));
        }
    }

    /// Puts the rows of a frame in as they are, without moving them or matching their levels.
    fn fill_unmoved(&mut self, member: usize, planes: &Planes) {
        let (rows, width) = (self.rows.clone(), self.width);
        if planes.width != width || planes.height < rows.end {
            return;
        }
        for channel in 0..COLORS {
            let region = self.region(channel, member);
            for (line, row) in region.chunks_exact_mut(width).zip(rows.clone()) {
                let source = &planes.data[channel][row * width..(row + 1) * width];
                for (value, sample) in line.iter_mut().zip(source) {
                    *value = f32::from(*sample);
                }
            }
        }
    }

    fn combine_into(&self, rejection: Rejection, out: &mut [f32]) {
        let (rows, width, members) = (self.rows.len(), self.width, self.members);
        let value_at = |channel: usize, member: usize, row: usize, column: usize| {
            self.samples[((channel * members + member) * rows + row) * width + column]
        };
        out.par_chunks_mut(width * COLORS)
            .enumerate()
            .for_each_init(
                || (Vec::with_capacity(members), Vec::new()),
                |(values, scratch), (row, line)| {
                    for column in 0..width {
                        for channel in 0..COLORS {
                            values.clear();
                            values.extend((0..members).map(|m| value_at(channel, m, row, column)));
                            line[column * COLORS + channel] = combine(values, rejection, scratch);
                        }
                    }
                },
            );
    }
}

fn strips(height: usize, rows: usize) -> Vec<Range<usize>> {
    (0..height)
        .step_by(rows.max(1))
        .map(|start| start..(start + rows).min(height))
        .collect()
}

/// A strip of the aligned frames and, when the ground is combined apart, a strip of the same rows
/// of the frames as they are.
struct Layers {
    sky: Strip,
    ground: Option<Strip>,
}

impl Layers {
    fn add(&mut self, position: usize, member: &Member, planes: &Planes, kernel: Kernel) {
        self.sky.fill(position, member, planes, kernel);
        if let Some(ground) = &mut self.ground {
            ground.fill_unmoved(position, planes);
        }
    }
}

/// Combines the aligned frames strip by strip. Frames kept in memory are used directly; the
/// others are made again for every strip.
pub fn integrate(
    members: &[Member],
    setup: &Integration,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<LinearRgbF32, PipelineError> {
    let (width, height) = (setup.width, setup.height);
    let mut samples = vec![0.0f32; width * height * COLORS];
    let ranges = strips(height, setup.strip_rows);
    for (done, rows) in ranges.iter().enumerate() {
        if cancel.is_requested() {
            return Err(PipelineError::Cancelled);
        }
        let needs_ground = setup
            .ground
            .is_some_and(|mask| mask.has_ground_in(rows.clone(), height));
        let mut layers = Layers {
            sky: Strip::new(rows.clone(), members.len(), width),
            ground: needs_ground.then(|| Strip::new(rows.clone(), members.len(), width)),
        };
        for (position, member) in members.iter().enumerate() {
            if let Some(planes) = &member.planes {
                layers.add(position, member, planes, setup.kernel);
            }
        }
        let missing: Vec<usize> = (0..members.len())
            .filter(|&position| members[position].planes.is_none())
            .collect();
        for_each_processed(
            missing.len(),
            setup.workers,
            |slot| demosaiced_planes(setup.context, members[missing[slot]].index, false),
            cancel,
            |slot, planes| {
                let position = missing[slot];
                layers.add(position, &members[position], &planes, setup.kernel);
                Ok(())
            },
        )?;
        let out = &mut samples[rows.start * width * COLORS..rows.end * width * COLORS];
        layers.sky.combine_into(setup.rejection, out);
        if let (Some(ground), Some(mask)) = (&layers.ground, setup.ground) {
            let mut under = vec![0.0f32; out.len()];
            ground.combine_into(setup.rejection, &mut under);
            let placed = Rows {
                width,
                height,
                channels: COLORS,
                first: rows.start,
            };
            blend_rows(out, &under, placed, mask, mix_f32);
        }
        progress(Progress {
            stage: Stage::Stack,
            done: done + 1,
            total: ranges.len(),
        });
    }
    samples
        .par_iter_mut()
        .for_each(|value| *value = value.max(0.0) / FULL_SCALE);
    Ok(LinearRgbF32 {
        width,
        height,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_cover_the_image_without_overlap() {
        assert_eq!(strips(10, 4), vec![0..4, 4..8, 8..10]);
        assert_eq!(strips(8, 4), vec![0..4, 4..8]);
        assert_eq!(strips(3, 100), vec![0..3]);
    }

    #[test]
    fn unmoved_frames_are_copied_row_by_row_without_changes() {
        let planes = Planes {
            width: 2,
            height: 3,
            data: [vec![1, 2, 3, 4, 5, 6], vec![7; 6], vec![9; 6]],
        };
        let mut strip = Strip::new(1..3, 1, 2);
        strip.fill_unmoved(0, &planes);
        assert_eq!(strip.region(0, 0), &[3.0, 4.0, 5.0, 6.0]);
        assert_eq!(strip.region(2, 0), &[9.0; 4]);
    }

    #[test]
    fn a_strip_combines_each_pixel_across_its_members() {
        let mut strip = Strip::new(0..2, 3, 2);
        for channel in 0..COLORS {
            for member in 0..3 {
                let region = strip.region(channel, member);
                for (index, value) in region.iter_mut().enumerate() {
                    *value = (member * 10 + channel + index) as f32;
                }
            }
        }
        let mut out = vec![0.0; 2 * 2 * COLORS];
        strip.combine_into(Rejection::None, &mut out);
        // pixel (row 0, column 0), channel 0: members give 0, 10, 20
        assert_eq!(out[0], 10.0);
        // pixel (row 1, column 1), channel 2: index 3, members give 5, 15, 25
        assert_eq!(out[(2 + 1) * COLORS + 2], 15.0);
    }
}
