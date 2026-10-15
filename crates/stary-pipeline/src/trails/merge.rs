use std::sync::Arc;

use rayon::prelude::*;
use stary_core::Frame;
use stary_mask::Mask;
use stary_register::Star;
use stary_stack::{CometAccumulator, LightenAccumulator, tail_weight};

use super::{TrailsMethod, TrailsSettings, gap_fill::GapFiller};
use crate::error::PipelineError;

/// How the lights are put together while they arrive.
pub enum Merge {
    Lighten(LightenAccumulator),
    Comet {
        accumulator: CometAccumulator,
        tail_frames: u32,
        reverse: bool,
    },
    /// Lighten by stretches of lights, with their stars kept, for filling the gaps afterwards.
    Gaps(GapFiller),
}

/// What the lights came to.
pub enum Merged {
    Sensor(Vec<u16>),
    Gaps(GapFiller),
}

impl Merge {
    pub fn new(settings: &TrailsSettings, samples: usize, frames: usize) -> Self {
        match settings.method {
            TrailsMethod::Lighten => Self::Lighten(LightenAccumulator::new(samples)),
            TrailsMethod::Comet {
                tail_frames,
                reverse,
            } => Self::Comet {
                accumulator: CometAccumulator::new(samples),
                tail_frames,
                reverse,
            },
            TrailsMethod::GapFilling => Self::Gaps(GapFiller::new(frames, &settings.gap_fill)),
        }
    }

    pub fn add(
        &mut self,
        frame: &Frame,
        index: usize,
        count: usize,
        stars: Vec<Star>,
    ) -> Result<(), PipelineError> {
        match self {
            Self::Lighten(accumulator) => accumulator
                .add(&frame.samples)
                .map_err(|_| PipelineError::NoInput),
            Self::Comet {
                accumulator,
                tail_frames,
                reverse,
            } => {
                let position = if *reverse { count - 1 - index } else { index };
                accumulator
                    .add(
                        &frame.samples,
                        frame.black,
                        tail_weight(position, count, *tail_frames),
                    )
                    .map_err(|_| PipelineError::NoInput)
            }
            Self::Gaps(filler) => filler.add(index, &frame.samples, stars),
        }
    }

    pub fn finish(self, black: u16) -> Merged {
        match self {
            Self::Lighten(accumulator) => Merged::Sensor(accumulator.into_samples()),
            Self::Comet { accumulator, .. } => Merged::Sensor(accumulator.into_samples(black)),
            Self::Gaps(filler) => Merged::Gaps(filler),
        }
    }
}

/// The sum of the frames as they are, for the ground.
pub struct GroundSum {
    pub mask: Arc<Mask>,
    sums: Vec<f32>,
}

impl GroundSum {
    pub fn new(mask: Arc<Mask>, len: usize) -> Self {
        Self {
            mask,
            sums: vec![0.0; len],
        }
    }

    pub fn add(&mut self, samples: &[u16]) {
        self.sums
            .par_iter_mut()
            .zip(samples.par_iter())
            .for_each(|(sum, &sample)| *sum += f32::from(sample));
    }

    pub fn mean(&self, frames: u32) -> Vec<u16> {
        let count = frames.max(1) as f32;
        self.sums
            .par_iter()
            .map(|&sum| (sum / count).round() as u16)
            .collect()
    }
}
