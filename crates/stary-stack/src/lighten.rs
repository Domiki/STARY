use rayon::prelude::*;

const CHUNK: usize = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("frame has {actual} samples but the accumulator expects {expected}")]
pub struct LengthMismatch {
    pub expected: usize,
    pub actual: usize,
}

/// Keeps the brightest value seen at every position. Works on any per-position data, so it can
/// run on raw sensor samples: the maximum commutes with white balance and black-level scaling.
#[derive(Debug, Clone)]
pub struct LightenAccumulator {
    samples: Vec<u16>,
    frames: u32,
}

impl LightenAccumulator {
    pub fn new(len: usize) -> Self {
        Self {
            samples: vec![0; len],
            frames: 0,
        }
    }

    pub const fn frames(&self) -> u32 {
        self.frames
    }

    pub fn add(&mut self, frame: &[u16]) -> Result<(), LengthMismatch> {
        if frame.len() != self.samples.len() {
            return Err(LengthMismatch {
                expected: self.samples.len(),
                actual: frame.len(),
            });
        }
        self.samples
            .par_chunks_mut(CHUNK)
            .zip(frame.par_chunks(CHUNK))
            .for_each(|(kept, incoming)| {
                for (kept, incoming) in kept.iter_mut().zip(incoming) {
                    *kept = (*kept).max(*incoming);
                }
            });
        self.frames += 1;
        Ok(())
    }

    pub fn into_samples(self) -> Vec<u16> {
        self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_largest_value_per_position() {
        let mut acc = LightenAccumulator::new(4);
        acc.add(&[1, 9, 3, 0]).unwrap();
        acc.add(&[5, 2, 3, 7]).unwrap();
        acc.add(&[2, 2, 8, 1]).unwrap();
        assert_eq!(acc.frames(), 3);
        assert_eq!(acc.into_samples(), vec![5, 9, 8, 7]);
    }

    #[test]
    fn the_result_does_not_depend_on_the_order_of_frames() {
        let frames: Vec<Vec<u16>> = (0..6u16)
            .map(|i| (0..100).map(|p| (p * 7 + i * 13) % 251).collect())
            .collect();
        let run = |order: &[usize]| {
            let mut acc = LightenAccumulator::new(100);
            order.iter().for_each(|i| acc.add(&frames[*i]).unwrap());
            acc.into_samples()
        };
        assert_eq!(run(&[0, 1, 2, 3, 4, 5]), run(&[5, 3, 1, 4, 0, 2]));
    }

    #[test]
    fn works_across_the_parallel_chunk_boundary() {
        let len = CHUNK * 2 + 17;
        let mut acc = LightenAccumulator::new(len);
        acc.add(&vec![3; len]).unwrap();
        let mut second = vec![1; len];
        second[CHUNK] = 9;
        second[len - 1] = 7;
        acc.add(&second).unwrap();
        let out = acc.into_samples();
        assert_eq!((out[0], out[CHUNK], out[len - 1]), (3, 9, 7));
    }

    #[test]
    fn rejects_a_frame_of_the_wrong_size() {
        let mut acc = LightenAccumulator::new(3);
        assert_eq!(
            acc.add(&[1, 2]),
            Err(LengthMismatch {
                expected: 3,
                actual: 2
            })
        );
        assert_eq!(acc.frames(), 0);
    }
}
