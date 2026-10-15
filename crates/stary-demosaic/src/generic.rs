use rayon::prelude::*;
use stary_core::{COLORS, CfaPattern};

const REACH: isize = 2;

/// Fills each missing color with the average of the nearest samples of that color, weighted by
/// distance. It works for any pattern, which is what X-Trans needs here.
pub fn demosaic_generic(
    samples: &[u16],
    width: usize,
    height: usize,
    pattern: &CfaPattern,
) -> Vec<u16> {
    let mut out = vec![0u16; width * height * COLORS];
    out.par_chunks_mut(width * COLORS)
        .enumerate()
        .for_each(|(row, line)| {
            for column in 0..width {
                let mut sums = [0.0f32; COLORS];
                let mut weights = [0.0f32; COLORS];
                for dy in -REACH..=REACH {
                    for dx in -REACH..=REACH {
                        let (y, x) = (row as isize + dy, column as isize + dx);
                        if y < 0 || x < 0 || y >= height as isize || x >= width as isize {
                            continue;
                        }
                        let color = pattern.color_at(y as usize, x as usize);
                        let weight = 1.0 / (1 + dx * dx + dy * dy) as f32;
                        sums[color] += weight * f32::from(samples[y as usize * width + x as usize]);
                        weights[color] += weight;
                    }
                }
                let own = pattern.color_at(row, column);
                for channel in 0..COLORS {
                    line[column * COLORS + channel] = if channel == own {
                        samples[row * width + column]
                    } else if weights[channel] > 0.0 {
                        (sums[channel] / weights[channel]).round() as u16
                    } else {
                        samples[row * width + column]
                    };
                }
            }
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xtrans() -> CfaPattern {
        const ROW_A: [u8; 6] = [1, 2, 0, 1, 0, 2];
        const ROW_B: [u8; 6] = [0, 1, 1, 2, 1, 1];
        const ROW_C: [u8; 6] = [2, 1, 1, 0, 1, 1];
        CfaPattern::XTrans([ROW_A, ROW_B, ROW_C, ROW_A, [0, 1, 1, 2, 1, 1], ROW_C])
    }

    #[test]
    fn a_uniform_color_is_reproduced_for_a_six_by_six_pattern() {
        let pattern = xtrans();
        let level = [3000u16, 8000, 15000];
        let (width, height) = (18, 12);
        let samples: Vec<u16> = (0..width * height)
            .map(|index| level[pattern.color_at(index / width, index % width)])
            .collect();
        let out = demosaic_generic(&samples, width, height, &pattern);
        for pixel in out.as_chunks::<3>().0 {
            assert_eq!(*pixel, level);
        }
    }
}
