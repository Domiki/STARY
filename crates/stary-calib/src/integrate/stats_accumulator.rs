use rayon::prelude::*;

use crate::geometry::GeometryError;

const CHUNK: usize = 1 << 16;

/// First pass of an integration: per-pixel sum and sum of squares, kept as exact integers.
#[derive(Debug)]
pub struct StatsAccumulator {
    sum: Vec<u32>,
    sum_sq: Vec<u64>,
    count: u32,
}

impl StatsAccumulator {
    pub fn new(len: usize) -> Self {
        Self {
            sum: vec![0; len],
            sum_sq: vec![0; len],
            count: 0,
        }
    }

    pub const fn count(&self) -> u32 {
        self.count
    }

    pub fn len(&self) -> usize {
        self.sum.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sum.is_empty()
    }

    pub fn add(&mut self, samples: &[u16]) -> Result<(), GeometryError> {
        if samples.len() != self.sum.len() {
            return Err(GeometryError::BadSampleCount);
        }
        self.sum
            .par_chunks_mut(CHUNK)
            .zip(self.sum_sq.par_chunks_mut(CHUNK))
            .zip(samples.par_chunks(CHUNK))
            .for_each(|((sums, squares), values)| {
                for ((sum, square), value) in sums.iter_mut().zip(squares).zip(values) {
                    let value = u32::from(*value);
                    *sum += value;
                    *square += u64::from(value * value);
                }
            });
        self.count += 1;
        Ok(())
    }

    pub fn mean_at(&self, index: usize) -> f32 {
        self.sum[index] as f32 / self.count.max(1) as f32
    }

    pub fn sigma_at(&self, index: usize) -> f32 {
        let n = f64::from(self.count.max(1));
        let sum = f64::from(self.sum[index]);
        let variance = (self.sum_sq[index] as f64 - sum * sum / n) / n;
        variance.max(0.0).sqrt() as f32
    }

    pub fn mean(&self) -> Vec<f32> {
        let n = self.count.max(1) as f32;
        self.sum.par_iter().map(|sum| *sum as f32 / n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_sigma_are_exact_for_small_inputs() {
        let mut stats = StatsAccumulator::new(2);
        for frame in [[2u16, 10], [4, 10], [6, 10], [8, 10]] {
            stats.add(&frame).unwrap();
        }
        assert_eq!(stats.count(), 4);
        assert_eq!(stats.mean(), vec![5.0, 10.0]);
        assert!((stats.sigma_at(0) - 5.0_f32.sqrt()).abs() < 1e-6);
        assert_eq!(stats.sigma_at(1), 0.0);
    }

    #[test]
    fn rejects_frames_of_the_wrong_size() {
        let mut stats = StatsAccumulator::new(3);
        assert_eq!(stats.add(&[1, 2]), Err(GeometryError::BadSampleCount));
        assert_eq!(stats.count(), 0);
    }
}
