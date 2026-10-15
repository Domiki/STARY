use rayon::prelude::*;
use stary_core::{BayerPattern, COLORS};

const GREEN: usize = 1;
const ROUNDING: i32 = 8;
const SHIFT: u32 = 4;

fn mirrored(position: isize, length: usize) -> usize {
    let last = length as isize - 1;
    let wrapped = if position < 0 { -position } else { position };
    let folded = if wrapped > last {
        2 * last - wrapped
    } else {
        wrapped
    };
    folded.clamp(0, last) as usize
}

struct Mosaic<'a> {
    samples: &'a [i32],
    width: usize,
    height: usize,
    colors: [u8; 4],
}

impl Mosaic<'_> {
    fn color(&self, row: usize, column: usize) -> usize {
        usize::from(self.colors[(row % 2) * 2 + column % 2])
    }

    fn at(&self, row: isize, column: isize) -> i32 {
        self.samples[mirrored(row, self.height) * self.width + mirrored(column, self.width)]
    }

    /// Malvar-He-Cutler gradient-corrected interpolation of one missing color.
    fn interpolate(&self, row: usize, column: usize, target: usize) -> i32 {
        let (r, c) = (row as isize, column as isize);
        let center = self.at(r, c);
        let own = self.color(row, column);
        let cross = |distance: isize| {
            self.at(r - distance, c)
                + self.at(r + distance, c)
                + self.at(r, c - distance)
                + self.at(r, c + distance)
        };
        let diagonal = || {
            self.at(r - 1, c - 1)
                + self.at(r - 1, c + 1)
                + self.at(r + 1, c - 1)
                + self.at(r + 1, c + 1)
        };
        let sixteenths = if target == GREEN {
            8 * center + 4 * cross(1) - 2 * cross(2)
        } else if own == GREEN {
            let horizontal = self.color(row, column + 1) == target;
            let (near, far, across) = if horizontal {
                (
                    self.at(r, c - 1) + self.at(r, c + 1),
                    self.at(r, c - 2) + self.at(r, c + 2),
                    self.at(r - 2, c) + self.at(r + 2, c),
                )
            } else {
                (
                    self.at(r - 1, c) + self.at(r + 1, c),
                    self.at(r - 2, c) + self.at(r + 2, c),
                    self.at(r, c - 2) + self.at(r, c + 2),
                )
            };
            10 * center + 8 * near - 2 * far - 2 * diagonal() + across
        } else {
            12 * center + 4 * diagonal() - 3 * cross(2)
        };
        (sixteenths + ROUNDING) >> SHIFT
    }
}

/// Demosaics Bayer data whose samples are already scaled to the 16-bit range.
pub fn demosaic_bayer(
    samples: &[u16],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) -> Vec<u16> {
    let widened: Vec<i32> = samples.iter().map(|value| i32::from(*value)).collect();
    let mosaic = Mosaic {
        samples: &widened,
        width,
        height,
        colors: pattern.colors(),
    };
    let mut out = vec![0u16; width * height * COLORS];
    out.par_chunks_mut(width * COLORS)
        .enumerate()
        .for_each(|(row, line)| {
            for column in 0..width {
                let own = mosaic.color(row, column);
                for channel in 0..COLORS {
                    let value = if channel == own {
                        mosaic.at(row as isize, column as isize)
                    } else {
                        mosaic.interpolate(row, column, channel)
                    };
                    line[column * COLORS + channel] = value.clamp(0, i32::from(u16::MAX)) as u16;
                }
            }
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mosaic_of(width: usize, height: usize, pattern: BayerPattern, level: [u16; 3]) -> Vec<u16> {
        let colors = pattern.colors();
        (0..width * height)
            .map(|index| {
                let (row, column) = (index / width, index % width);
                level[usize::from(colors[(row % 2) * 2 + column % 2])]
            })
            .collect()
    }

    #[test]
    fn a_uniform_color_is_reproduced_everywhere_including_the_borders() {
        for pattern in [
            BayerPattern::Rggb,
            BayerPattern::Bggr,
            BayerPattern::Grbg,
            BayerPattern::Gbrg,
        ] {
            let level = [4000, 9000, 20000];
            let out = demosaic_bayer(&mosaic_of(12, 10, pattern, level), 12, 10, pattern);
            for pixel in out.as_chunks::<3>().0 {
                assert_eq!(*pixel, level, "{pattern:?}");
            }
        }
    }

    #[test]
    fn a_horizontal_ramp_is_interpolated_exactly_in_the_interior() {
        let (width, height) = (16, 8);
        let pattern = BayerPattern::Rggb;
        let ramp = |column: usize| 1000 + 100 * column as u16;
        let samples: Vec<u16> = (0..width * height)
            .map(|index| ramp(index % width))
            .collect();
        let out = demosaic_bayer(&samples, width, height, pattern);
        for row in 3..height - 3 {
            for column in 3..width - 3 {
                for channel in 0..3 {
                    let value = i32::from(out[(row * width + column) * 3 + channel]);
                    assert!(
                        (value - i32::from(ramp(column))).abs() <= 1,
                        "{row},{column}"
                    );
                }
            }
        }
    }

    #[test]
    fn mirrored_positions_stay_inside() {
        assert_eq!(mirrored(-2, 10), 2);
        assert_eq!(mirrored(-1, 10), 1);
        assert_eq!(mirrored(10, 10), 8);
        assert_eq!(mirrored(11, 10), 7);
        assert_eq!(mirrored(4, 10), 4);
    }
}
