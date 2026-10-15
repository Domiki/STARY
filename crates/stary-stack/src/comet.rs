use rayon::prelude::*;

use crate::lighten::LengthMismatch;

const CHUNK: usize = 1 << 16;
/// How much of a frame is left at the end of the tail.
const TAIL_END_WEIGHT: f32 = 0.05;

/// The weight of frame `index` of `count` when the tail fades to nothing over `tail_frames`
/// frames. Writing the comet rule `acc = max(acc * d, frame)` out gives
/// `max over frames of frame * d^(frames after it)`, so frames can come in any order.
pub fn tail_weight(index: usize, count: usize, tail_frames: u32) -> f32 {
    let decay = TAIL_END_WEIGHT.powf(1.0 / tail_frames.max(1) as f32);
    decay.powi(count.saturating_sub(index + 1) as i32)
}

/// Keeps the brightest weighted value above the black level at every position.
#[derive(Debug, Clone)]
pub struct CometAccumulator {
    signal: Vec<f32>,
}

impl CometAccumulator {
    pub fn new(len: usize) -> Self {
        Self {
            signal: vec![0.0; len],
        }
    }

    pub fn add(&mut self, frame: &[u16], black: u16, weight: f32) -> Result<(), LengthMismatch> {
        if frame.len() != self.signal.len() {
            return Err(LengthMismatch {
                expected: self.signal.len(),
                actual: frame.len(),
            });
        }
        let black = f32::from(black);
        self.signal
            .par_chunks_mut(CHUNK)
            .zip(frame.par_chunks(CHUNK))
            .for_each(|(kept, incoming)| {
                for (kept, incoming) in kept.iter_mut().zip(incoming) {
                    let weighted = (f32::from(*incoming) - black).max(0.0) * weight;
                    *kept = kept.max(weighted);
                }
            });
        Ok(())
    }

    pub fn into_samples(self, black: u16) -> Vec<u16> {
        let black = f32::from(black);
        self.signal
            .into_par_iter()
            .map(|signal| (black + signal).round().min(f32::from(u16::MAX)) as u16)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_frame_counts_in_full_and_older_ones_fade() {
        assert_eq!(tail_weight(9, 10, 5), 1.0);
        let older = tail_weight(4, 10, 5);
        assert!((older - TAIL_END_WEIGHT).abs() < 1e-5, "{older}");
        assert!(tail_weight(8, 10, 5) > tail_weight(7, 10, 5));
    }

    #[test]
    fn a_longer_tail_fades_more_slowly() {
        assert!(tail_weight(0, 10, 20) > tail_weight(0, 10, 3));
    }

    #[test]
    fn it_matches_the_step_by_step_rule() {
        let (count, tail) = (6usize, 3u32);
        let frames: Vec<Vec<u16>> = (0..count)
            .map(|i| {
                (0..8)
                    .map(|p| 100 + ((p * 37 + i * 91) % 300) as u16)
                    .collect()
            })
            .collect();
        let black = 100u16;
        let decay = TAIL_END_WEIGHT.powf(1.0 / tail as f32);
        let mut stepwise = vec![0.0f32; 8];
        for frame in &frames {
            for (kept, value) in stepwise.iter_mut().zip(frame) {
                *kept = (*kept * decay).max(f32::from(*value) - f32::from(black));
            }
        }
        let mut acc = CometAccumulator::new(8);
        // any order gives the same result
        for index in [3usize, 0, 5, 1, 4, 2] {
            acc.add(&frames[index], black, tail_weight(index, count, tail))
                .unwrap();
        }
        let got = acc.into_samples(black);
        for (g, s) in got.iter().zip(&stepwise) {
            assert!(
                (f32::from(*g) - (f32::from(black) + s)).abs() <= 1.0,
                "{g} vs {s}"
            );
        }
    }

    #[test]
    fn values_below_the_black_level_add_nothing() {
        let mut acc = CometAccumulator::new(2);
        acc.add(&[50, 150], 100, 1.0).unwrap();
        assert_eq!(acc.into_samples(100), vec![100, 150]);
    }

    #[test]
    fn a_frame_of_the_wrong_size_is_refused() {
        let mut acc = CometAccumulator::new(3);
        assert!(acc.add(&[1, 2], 0, 1.0).is_err());
    }
}
