mod fill;
mod motion;
mod segments;
mod stars;

use std::path::Path;

use rayon::prelude::*;
use stary_core::{Frame, LinearRgb16};
use stary_register::{AlignParams, DetectParams, Model, Star, Transform};
use stary_stack::LightenAccumulator;

pub use self::stars::frame_stars;
use self::{fill::Canvas, segments::Segments};
use crate::{
    control::{Cancel, Progress, Stage},
    demosaicer::Demosaicer,
    error::PipelineError,
    masking::Masking,
    planes::split_planes,
};

/// What the finding of stars, the matching of their motion and the filling can be told.
#[derive(Debug, Clone, Copy)]
pub struct GapFillTuning {
    pub detect: DetectParams,
    pub align: AlignParams,
    /// How many lights are merged into one picture before it is filled (the motion of the stars
    /// is taken to be the same within one).
    pub stretch_frames: usize,
    /// The most stretches the lights are cut into, however short `stretch_frames` is (each holds
    /// a picture in memory).
    pub max_stretches: usize,
    /// How far apart, in pixels, the images of a star are laid between two lights. A star is
    /// seen as a line when the images are well within its width, and finer than a pixel costs
    /// little because only one picture per stretch is moved.
    pub spacing: f64,
    /// How many pairs of consecutive lights of a stretch have their stars matched to tell its
    /// motion. Only the lights of those pairs are searched for stars.
    pub pairs_per_stretch: usize,
}

const STRETCH_FRAMES: usize = 100;
const MAX_STRETCHES: usize = 16;
const PAIRS_PER_STRETCH: usize = 8;
const SPACING: f64 = 0.8;

impl Default for GapFillTuning {
    fn default() -> Self {
        Self {
            detect: DetectParams::default(),
            align: AlignParams {
                model: Model::Homography,
                ..AlignParams::default()
            },
            stretch_frames: STRETCH_FRAMES,
            max_stretches: MAX_STRETCHES,
            spacing: SPACING,
            pairs_per_stretch: PAIRS_PER_STRETCH,
        }
    }
}

/// Which lights have their stars searched for: those of the pairs whose motion is measured.
pub fn light_needs_stars(frames: usize, tuning: &GapFillTuning) -> Vec<bool> {
    let segments = Segments::new(frames, tuning.stretch_frames, tuning.max_stretches);
    let mut needed = vec![false; frames];
    for stretch in 0..segments.count() {
        for pair in segments.sampled_pairs(stretch, tuning.pairs_per_stretch) {
            needed[pair] = true;
            needed[pair + 1] = true;
        }
    }
    needed
}

/// The lights of a run of gap filling, merged by the stretch with the stars of some of them kept.
pub struct GapFiller {
    tuning: GapFillTuning,
    segments: Segments,
    accumulators: Vec<Option<LightenAccumulator>>,
    stars: Vec<Vec<Star>>,
}

/// What the end of the run needs to know about the frames and where to put things.
pub struct GapContext<'a> {
    pub shape: &'a Frame,
    pub demosaicer: &'a Demosaicer,
    pub first_light: &'a Path,
    pub masking: Option<&'a Masking>,
    pub cancel: &'a Cancel,
    pub progress: &'a (dyn Fn(Progress) + Sync),
}

pub struct GapPicture {
    pub image: LinearRgb16,
    /// The lights merged without filling, for the ground when it is not made apart.
    pub plain: Option<LinearRgb16>,
    pub gaps_filled: u32,
    pub gaps_skipped: u32,
}

impl GapFiller {
    pub fn new(frames: usize, tuning: &GapFillTuning) -> Self {
        let segments = Segments::new(frames, tuning.stretch_frames, tuning.max_stretches);
        Self {
            tuning: *tuning,
            accumulators: (0..segments.count()).map(|_| None).collect(),
            segments,
            stars: vec![Vec::new(); frames],
        }
    }

    pub fn add(
        &mut self,
        index: usize,
        samples: &[u16],
        stars: Vec<Star>,
    ) -> Result<(), PipelineError> {
        let stretch = self.segments.of(index);
        self.accumulators[stretch]
            .get_or_insert_with(|| LightenAccumulator::new(samples.len()))
            .add(samples)
            .map_err(|_| PipelineError::NoInput)?;
        self.stars[index] = stars;
        Ok(())
    }

    fn stars_in_sky(&self, shape: &Frame, masking: Option<&Masking>) -> Vec<Vec<Star>> {
        let Some(masking) = masking else {
            return self.stars.clone();
        };
        self.stars
            .iter()
            .map(|frame| {
                frame
                    .iter()
                    .filter(|star| {
                        masking
                            .mask
                            .is_sky_at(star.x, star.y, shape.width, shape.height)
                    })
                    .copied()
                    .collect()
            })
            .collect()
    }

    /// The motion of the stars in every stretch, from the pairs that were measured, and how many of
    /// those pairs matched and did not. A stretch whose pairs all failed takes the usual motion of
    /// the others.
    fn stretch_motions(
        &self,
        stars: &[Vec<Star>],
        progress: &(dyn Fn(Progress) + Sync),
    ) -> (Vec<Option<Transform>>, u32, u32) {
        let stretches = self.segments.count();
        let pairs: Vec<Vec<usize>> = (0..stretches)
            .map(|stretch| {
                self.segments
                    .sampled_pairs(stretch, self.tuning.pairs_per_stretch)
            })
            .collect();
        let total: usize = pairs.iter().map(Vec::len).sum();
        progress(Progress {
            stage: Stage::Align,
            done: 0,
            total,
        });
        let measured: Vec<Vec<Option<Transform>>> = pairs
            .iter()
            .map(|pairs| motion::pair_motions(stars, pairs, &self.tuning.align))
            .collect();
        progress(Progress {
            stage: Stage::Align,
            done: total,
            total,
        });
        let filled = measured.iter().flatten().flatten().count() as u32;
        let own: Vec<Option<Transform>> = measured
            .iter()
            .map(|found| motion::typical_motion(found))
            .collect();
        let usual = motion::typical_motion(&own);
        let motions = own
            .into_iter()
            .enumerate()
            .map(|(stretch, motion)| {
                if self.segments.is_last(stretch) {
                    None
                } else {
                    motion.or(usual)
                }
            })
            .collect();
        (motions, filled, total as u32 - filled)
    }

    /// Fills the gaps in the picture of every stretch along the motion of its stars and lays the
    /// pictures on each other.
    pub fn finish(mut self, context: &GapContext) -> Result<GapPicture, PipelineError> {
        let sky_stars = self.stars_in_sky(context.shape, context.masking);
        let (motions, gaps_filled, gaps_skipped) =
            self.stretch_motions(&sky_stars, context.progress);
        let spacing = self.tuning.spacing;

        let (width, height) = (context.shape.width, context.shape.height);
        let mut canvas = Canvas::new(width, height);
        let mut plain = context
            .masking
            .filter(|masking| !masking.separates_ground())
            .map(|_| vec![0u16; width * height * stary_core::COLORS]);
        let stretches = self.segments.count();
        for (stretch, accumulator) in self.accumulators.iter_mut().enumerate() {
            if context.cancel.is_requested() {
                return Err(PipelineError::Cancelled);
            }
            let Some(accumulator) = accumulator.take() else {
                continue;
            };
            let merged = Frame {
                samples: accumulator.into_samples(),
                ..context.shape.clone()
            };
            let image = context.demosaicer.run(&merged, context.first_light)?;
            drop(merged);
            if let Some(plain) = &mut plain {
                plain
                    .par_iter_mut()
                    .zip(image.samples.par_iter())
                    .for_each(|(kept, value)| *kept = (*kept).max(*value));
            }
            let planes = split_planes(&image.samples, image.width, image.height);
            drop(image);
            match &motions[stretch] {
                Some(motion) => canvas.add_filled(&planes, motion, spacing),
                None => canvas.add_unmoved(&planes),
            }
            (context.progress)(Progress {
                stage: Stage::Demosaic,
                done: stretch + 1,
                total: stretches,
            });
        }
        Ok(GapPicture {
            image: canvas.into_image(),
            plain: plain.map(|samples| LinearRgb16 {
                width,
                height,
                samples,
            }),
            gaps_filled,
            gaps_skipped,
        })
    }
}
