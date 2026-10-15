use rayon::prelude::*;

use crate::geometry::GeometryError;

const CHUNK: usize = 1 << 16;

/// How many of the largest and of the smallest values of a pixel are left out at most; more would
/// take too much memory and, with that many frames, reading twice costs less than it saves.
pub const MAX_TRIM: usize = 4;
/// Of every frame, this share is taken to be spoiled at a pixel (a hot pixel, a cosmic ray), half of
/// them on each side, on top of the values that lie out in the tails of a normal spread.
const SPOILED_SHARE: f64 = 0.002;
/// With fewer frames than this, leaving out the largest and the smallest value of every pixel costs
/// a fifth or more of the data, and reading them twice costs little anyway.
pub const MIN_FRAMES_FOR_TRIMMING: u32 = 8;
/// The chance allowed that a side has more values to leave out than there is room for.
const MISS_CHANCE: f64 = 0.01;

/// The chance that a normal value lies more than `kappa` spreads above its mean.
fn upper_tail(kappa: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * kappa);
    let poly = t
        * (0.319381530
            + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    (-kappa * kappa / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt() * poly
}

/// How many values to leave out on each side of a pixel when `frames` frames are read once: the
/// fewest that make it unlikely (`MISS_CHANCE`) that more values than that lie beyond `kappa`
/// spreads. `None` when that takes more than `MAX_TRIM`, or when there are too few frames to leave
/// any out.
pub fn trim_count(frames: u32, kappa: f32) -> Option<usize> {
    if frames < MIN_FRAMES_FOR_TRIMMING {
        return None;
    }
    let per_side = f64::from(frames) * (upper_tail(f64::from(kappa)) + SPOILED_SHARE / 2.0);
    let mut chance_of_exactly = (-per_side).exp();
    let mut chance_of_more = 1.0 - chance_of_exactly;
    let mut count = 0;
    while chance_of_more > MISS_CHANCE {
        count += 1;
        chance_of_exactly *= per_side / count as f64;
        chance_of_more -= chance_of_exactly;
        if count > MAX_TRIM {
            return None;
        }
    }
    let count = count.max(1);
    (2 * count + 2 <= frames as usize).then_some(count)
}

/// An integration made in one read of the frames: per pixel the sum, the sum of squares and the
/// `keep` largest and smallest values, which are taken out of the mean at the end.
#[derive(Debug)]
pub struct TrimmedAccumulator {
    sum: Vec<u32>,
    sum_sq: Vec<u64>,
    /// `keep` values per pixel, the smallest of the largest first.
    high: Vec<u16>,
    /// `keep` values per pixel, the largest of the smallest first.
    low: Vec<u16>,
    keep: usize,
    count: u32,
}

/// What an integration in one read gave.
#[derive(Debug, Clone, PartialEq)]
pub struct Trimmed {
    pub samples: Vec<f32>,
    /// Pixels where every place for a value to leave out on one side went to a value far outside
    /// the rest, so that more values than there was room for may be spoiled.
    pub full: usize,
}

fn insert_high(slots: &mut [u16], value: u16) {
    if value <= slots[0] {
        return;
    }
    slots[0] = value;
    let mut at = 0;
    while at + 1 < slots.len() && slots[at] > slots[at + 1] {
        slots.swap(at, at + 1);
        at += 1;
    }
}

fn insert_low(slots: &mut [u16], value: u16) {
    if value >= slots[0] {
        return;
    }
    slots[0] = value;
    let mut at = 0;
    while at + 1 < slots.len() && slots[at] < slots[at + 1] {
        slots.swap(at, at + 1);
        at += 1;
    }
}

impl TrimmedAccumulator {
    pub fn new(len: usize, keep: usize) -> Self {
        Self {
            sum: vec![0; len],
            sum_sq: vec![0; len],
            high: vec![0; len * keep],
            low: vec![u16::MAX; len * keep],
            keep,
            count: 0,
        }
    }

    pub fn add(&mut self, samples: &[u16]) -> Result<(), GeometryError> {
        if samples.len() != self.sum.len() {
            return Err(GeometryError::BadSampleCount);
        }
        let keep = self.keep;
        self.sum
            .par_chunks_mut(CHUNK)
            .zip(self.sum_sq.par_chunks_mut(CHUNK))
            .zip(self.high.par_chunks_mut(CHUNK * keep))
            .zip(self.low.par_chunks_mut(CHUNK * keep))
            .zip(samples.par_chunks(CHUNK))
            .for_each(|((((sums, squares), highs), lows), values)| {
                for (at, &value) in values.iter().enumerate() {
                    let wide = u32::from(value);
                    sums[at] += wide;
                    squares[at] += u64::from(wide * wide);
                    insert_high(&mut highs[at * keep..(at + 1) * keep], value);
                    insert_low(&mut lows[at * keep..(at + 1) * keep], value);
                }
            });
        self.count += 1;
        Ok(())
    }

    fn pixel(&self, index: usize, kappa: f32) -> (f32, bool) {
        let keep = self.keep;
        let (highs, lows) = (
            &self.high[index * keep..(index + 1) * keep],
            &self.low[index * keep..(index + 1) * keep],
        );
        let dropped = highs.iter().chain(lows).map(|&value| u64::from(value));
        let dropped_sum: u64 = dropped.clone().sum();
        let dropped_squares: u64 = dropped.map(|value| value * value).sum();
        let kept = u64::from(self.count) - 2 * keep as u64;
        let sum = u64::from(self.sum[index]) - dropped_sum;
        let mean = sum as f64 / kept as f64;
        let variance =
            ((self.sum_sq[index] - dropped_squares) as f64 / kept as f64 - mean * mean).max(0.0);
        let reach = 2.0 * f64::from(kappa) * variance.sqrt() + 0.5;
        let full = f64::from(highs[0]) - mean > reach || mean - f64::from(lows[0]) > reach;
        (mean as f32, full)
    }

    /// The mean of what is left after the largest and the smallest values are taken out.
    pub fn finish(&self, kappa: f32) -> Trimmed {
        let pixels: Vec<(f32, bool)> = (0..self.sum.len())
            .into_par_iter()
            .map(|index| self.pixel(index, kappa))
            .collect();
        Trimmed {
            full: pixels.iter().filter(|(_, full)| *full).count(),
            samples: pixels.into_iter().map(|(mean, _)| mean).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integrate(frames: &[Vec<u16>], keep: usize) -> Trimmed {
        let mut accumulator = TrimmedAccumulator::new(frames[0].len(), keep);
        frames
            .iter()
            .for_each(|frame| accumulator.add(frame).unwrap());
        accumulator.finish(3.0)
    }

    #[test]
    fn the_largest_and_the_smallest_value_are_left_out() {
        let frames: Vec<Vec<u16>> = [10u16, 12, 14, 16, 1000, 2]
            .iter()
            .map(|&v| vec![v])
            .collect();
        assert_eq!(integrate(&frames, 1).samples, vec![13.0]);
    }

    #[test]
    fn two_spoiled_values_on_one_side_are_left_out_with_room_for_two() {
        let frames: Vec<Vec<u16>> = [100u16, 101, 99, 100, 4000, 3000, 100, 98]
            .iter()
            .map(|&v| vec![v])
            .collect();
        let kept = integrate(&frames, 2).samples[0];
        assert!((kept - 99.5).abs() < 1.0, "{kept}");
    }

    #[test]
    fn what_is_left_when_there_are_fewer_values_than_room_is_still_exact() {
        let frames: Vec<Vec<u16>> = [0u16, 0, 65535, 65535, 5, 7]
            .iter()
            .map(|&v| vec![v])
            .collect();
        assert_eq!(integrate(&frames, 2).samples, vec![6.0]);
    }

    #[test]
    fn identical_frames_pass_through_and_are_not_doubtful() {
        let frames: Vec<Vec<u16>> = (0..6).map(|_| vec![7, 8, 9]).collect();
        let result = integrate(&frames, 1);
        assert_eq!(result.samples, vec![7.0, 8.0, 9.0]);
        assert_eq!(result.full, 0);
    }

    fn spoiled(values: &[(usize, u16)]) -> Vec<Vec<u16>> {
        let mut frames: Vec<Vec<u16>> = (0..10)
            .map(|index| vec![100 + (index % 3) as u16, 100])
            .collect();
        for &(frame, value) in values {
            frames[frame][0] = value;
        }
        frames
    }

    #[test]
    fn a_pixel_is_full_when_every_place_went_to_a_spoiled_value() {
        assert_eq!(integrate(&spoiled(&[(2, 3000)]), 1).full, 1);
        assert_eq!(integrate(&spoiled(&[(2, 3000), (5, 3500)]), 2).full, 1);
    }

    #[test]
    fn a_pixel_with_a_place_to_spare_is_not_full() {
        assert_eq!(integrate(&spoiled(&[(2, 3000)]), 2).full, 0);
        assert_eq!(integrate(&spoiled(&[]), 1).full, 0);
    }

    #[test]
    fn frames_of_the_wrong_size_are_refused() {
        let mut accumulator = TrimmedAccumulator::new(3, 1);
        assert_eq!(accumulator.add(&[1, 2]), Err(GeometryError::BadSampleCount));
    }
}

#[cfg(test)]
mod count_tests {
    use super::*;

    #[test]
    fn the_count_grows_slowly_with_the_frames() {
        let counts: Vec<Option<usize>> = [20, 60, 100, 300, 1172]
            .iter()
            .map(|&frames| trim_count(frames, 3.0))
            .collect();
        assert_eq!(counts[0], Some(1));
        assert!(
            counts[1].unwrap() <= 2 && counts[2].unwrap() <= 2,
            "{counts:?}"
        );
        assert!(counts[3].unwrap() >= 3);
        assert_eq!(counts[4], None);
    }

    #[test]
    fn too_few_frames_leave_nothing_out() {
        assert_eq!(trim_count(3, 3.0), None);
        assert_eq!(trim_count(MIN_FRAMES_FOR_TRIMMING - 1, 3.0), None);
        assert_eq!(trim_count(MIN_FRAMES_FOR_TRIMMING, 3.0), Some(1));
    }

    #[test]
    fn the_tail_chance_of_a_normal_value_is_known() {
        assert!((upper_tail(3.0) - 0.00135).abs() < 1e-5);
        assert!((upper_tail(2.0) - 0.02275).abs() < 1e-4);
    }
}
