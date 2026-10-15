use rayon::prelude::*;
use stary_core::COLORS;
use stary_stack::{SourcePlane, warp_rows};

use super::{Clipping, Combined, Lay, Layers, StreamSetup};
use crate::{error::PipelineError, planes::Planes, stacking::integrate::Member};

const FULL_SCALE: f32 = 65535.0;
const CHUNK: usize = 1 << 16;

/// What is known of every sample of the picture of one kind of lay (moved or not).
struct Layer {
    sum: [Vec<f64>; COLORS],
    sum_squares: [Vec<f64>; COLORS],
    count: [Vec<u16>; COLORS],
    kept_sum: [Vec<f64>; COLORS],
    kept: [Vec<u16>; COLORS],
}

impl Layer {
    fn new(len: usize, clipping: bool) -> Self {
        let extra = if clipping { len } else { 0 };
        Self {
            sum: std::array::from_fn(|_| vec![0.0; len]),
            sum_squares: std::array::from_fn(|_| vec![0.0; len]),
            count: std::array::from_fn(|_| vec![0; len]),
            kept_sum: std::array::from_fn(|_| vec![0.0; extra]),
            kept: std::array::from_fn(|_| vec![0; extra]),
        }
    }

    /// First lay: counts, sums and sums of squares. Samples a frame does not cover are NaN.
    fn add_statistics(&mut self, values: &[Vec<f32>; COLORS]) {
        for channel in 0..COLORS {
            self.sum[channel]
                .par_chunks_mut(CHUNK)
                .zip(self.sum_squares[channel].par_chunks_mut(CHUNK))
                .zip(self.count[channel].par_chunks_mut(CHUNK))
                .zip(values[channel].par_chunks(CHUNK))
                .for_each(|(((sum, squares), count), values)| {
                    for (((sum, square), count), &value) in
                        sum.iter_mut().zip(squares).zip(count).zip(values)
                    {
                        if value.is_finite() {
                            let value = f64::from(value);
                            *sum += value;
                            *square += value * value;
                            *count += 1;
                        }
                    }
                });
        }
    }

    /// Second lay: adds up the values within `clipping` spreads of the mean.
    fn add_clipped(&mut self, values: &[Vec<f32>; COLORS], clipping: Clipping) {
        let (low, high) = (f64::from(clipping.low), f64::from(clipping.high));
        for channel in 0..COLORS {
            self.kept_sum[channel]
                .par_chunks_mut(CHUNK)
                .zip(self.kept[channel].par_chunks_mut(CHUNK))
                .zip(self.sum[channel].par_chunks(CHUNK))
                .zip(self.sum_squares[channel].par_chunks(CHUNK))
                .zip(self.count[channel].par_chunks(CHUNK))
                .zip(values[channel].par_chunks(CHUNK))
                .for_each(|(((((kept_sum, kept), sum), squares), count), values)| {
                    for index in 0..values.len() {
                        let value = values[index];
                        if !value.is_finite() || count[index] == 0 {
                            continue;
                        }
                        let n = f64::from(count[index]);
                        let mean = sum[index] / n;
                        let spread = (squares[index] / n - mean * mean).max(0.0).sqrt();
                        let off = f64::from(value) - mean;
                        if off >= -low * spread && off <= high * spread {
                            kept_sum[index] += f64::from(value);
                            kept[index] += 1;
                        }
                    }
                });
        }
    }

    /// The combined value of every sample, interleaved as colors of pixels and scaled to 1.0.
    fn into_samples(self, len: usize, clipped: bool) -> Vec<f32> {
        let mut samples = vec![0.0f32; len * COLORS];
        samples
            .par_chunks_exact_mut(COLORS)
            .enumerate()
            .for_each(|(index, pixel)| {
                for (channel, out) in pixel.iter_mut().enumerate() {
                    let n = self.count[channel][index];
                    let kept = if clipped {
                        self.kept[channel][index]
                    } else {
                        0
                    };
                    let value = if kept > 0 {
                        self.kept_sum[channel][index] / f64::from(kept)
                    } else if n > 0 {
                        self.sum[channel][index] / f64::from(n)
                    } else {
                        0.0
                    };
                    *out = (value as f32).max(0.0) / FULL_SCALE;
                }
            });
        samples
    }
}

/// The planes of a frame moved onto the picture and matched to the reference, as samples that are
/// NaN where the frame does not cover the picture.
fn lay_moved(
    scratch: &mut [Vec<f32>; COLORS],
    planes: &Planes,
    member: &Member,
    setup: &StreamSetup,
) {
    for (channel, out) in scratch.iter_mut().enumerate() {
        let source = SourcePlane {
            width: planes.width,
            height: planes.height,
            data: &planes.data[channel],
        };
        warp_rows(
            &source,
            &member.to_source,
            setup.kernel,
            0..setup.height,
            setup.width,
            out,
        );
        let matching = member.matching[channel];
        out.par_iter_mut()
            .filter(|value| value.is_finite())
            .for_each(|value| *value = matching.apply(*value));
    }
}

/// The planes of a frame as they are.
fn lay_unmoved(scratch: &mut [Vec<f32>; COLORS], planes: &Planes) {
    for (out, plane) in scratch.iter_mut().zip(&planes.data) {
        if out.len() == plane.len() {
            out.par_iter_mut()
                .zip(plane.par_iter())
                .for_each(|(out, value)| *out = f32::from(*value));
        } else {
            out.fill(f32::NAN);
        }
    }
}

/// The picture kept in the memory of the computer, laid on by the processor.
pub struct CpuLayers<'a> {
    setup: &'a StreamSetup<'a>,
    scratch: [Vec<f32>; COLORS],
    sky: Layer,
    ground: Option<Layer>,
}

impl<'a> CpuLayers<'a> {
    pub fn new(setup: &'a StreamSetup<'a>) -> Self {
        let len = setup.width * setup.height;
        let clipping = setup.clipping.is_some();
        Self {
            setup,
            scratch: std::array::from_fn(|_| vec![0.0; len]),
            sky: Layer::new(len, clipping),
            ground: setup.ground.map(|_| Layer::new(len, clipping)),
        }
    }

    fn add(&mut self, layer: LayerKind, lay: Lay) {
        let target = match layer {
            LayerKind::Sky => Some(&mut self.sky),
            LayerKind::Ground => self.ground.as_mut(),
        };
        let Some(target) = target else { return };
        match lay {
            Lay::Statistics => target.add_statistics(&self.scratch),
            Lay::Clipped(clipping) => target.add_clipped(&self.scratch, clipping),
        }
    }
}

#[derive(Clone, Copy)]
enum LayerKind {
    Sky,
    Ground,
}

impl Layers for CpuLayers<'_> {
    fn lay(&mut self, member: &Member, planes: &Planes, lay: Lay) -> Result<(), PipelineError> {
        lay_moved(&mut self.scratch, planes, member, self.setup);
        self.add(LayerKind::Sky, lay);
        if self.ground.is_some() {
            lay_unmoved(&mut self.scratch, planes);
            self.add(LayerKind::Ground, lay);
        }
        Ok(())
    }

    fn combined(self, clipped: bool) -> Result<Combined, PipelineError> {
        let len = self.setup.width * self.setup.height;
        drop(self.scratch);
        Ok(Combined {
            sky: self.sky.into_samples(len, clipped),
            ground: self.ground.map(|ground| ground.into_samples(len, clipped)),
        })
    }
}
