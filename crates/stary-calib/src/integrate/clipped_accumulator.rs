use rayon::prelude::*;

use super::stats_accumulator::StatsAccumulator;
use crate::geometry::GeometryError;

const CHUNK: usize = 1 << 16;

/// Second pass of an integration: sums only the values within `kappa` sigma of the mean.
#[derive(Debug)]
pub struct ClippedAccumulator {
    sum: Vec<u32>,
    kept: Vec<u16>,
}

impl ClippedAccumulator {
    pub fn new(len: usize) -> Self {
        Self {
            sum: vec![0; len],
            kept: vec![0; len],
        }
    }

    pub fn add(
        &mut self,
        stats: &StatsAccumulator,
        samples: &[u16],
        kappa: f32,
    ) -> Result<(), GeometryError> {
        if samples.len() != self.sum.len() || stats.len() != self.sum.len() {
            return Err(GeometryError::BadSampleCount);
        }
        self.sum
            .par_chunks_mut(CHUNK)
            .zip(self.kept.par_chunks_mut(CHUNK))
            .zip(samples.par_chunks(CHUNK))
            .enumerate()
            .for_each(|(chunk, ((sums, kept), values))| {
                for (offset, ((sum, kept), value)) in
                    sums.iter_mut().zip(kept).zip(values).enumerate()
                {
                    let index = chunk * CHUNK + offset;
                    let limit = kappa * stats.sigma_at(index);
                    if (f32::from(*value) - stats.mean_at(index)).abs() <= limit {
                        *sum += u32::from(*value);
                        *kept += 1;
                    }
                }
            });
        Ok(())
    }

    /// Mean of the kept values; falls back to the plain mean where everything was rejected.
    pub fn finish(&self, stats: &StatsAccumulator) -> Vec<f32> {
        (0..self.sum.len())
            .into_par_iter()
            .map(|index| match self.kept[index] {
                0 => stats.mean_at(index),
                kept => self.sum[index] as f32 / f32::from(kept),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integrate(frames: &[Vec<u16>], kappa: f32) -> Vec<f32> {
        let len = frames[0].len();
        let mut stats = StatsAccumulator::new(len);
        frames.iter().for_each(|frame| stats.add(frame).unwrap());
        let mut clipped = ClippedAccumulator::new(len);
        frames
            .iter()
            .for_each(|frame| clipped.add(&stats, frame, kappa).unwrap());
        clipped.finish(&stats)
    }

    #[test]
    fn removes_a_single_outlier() {
        let mut frames: Vec<Vec<u16>> = (0..9).map(|_| vec![100, 50]).collect();
        frames.push(vec![1000, 50]);
        assert_eq!(integrate(&frames, 2.0), vec![100.0, 50.0]);
    }

    #[test]
    fn keeps_everything_when_there_is_no_outlier() {
        let frames: Vec<Vec<u16>> = vec![vec![10], vec![12], vec![14], vec![16]];
        assert_eq!(integrate(&frames, 3.0), vec![13.0]);
    }

    #[test]
    fn identical_frames_pass_through() {
        let frames: Vec<Vec<u16>> = (0..5).map(|_| vec![7, 8, 9]).collect();
        assert_eq!(integrate(&frames, 3.0), vec![7.0, 8.0, 9.0]);
    }
}
