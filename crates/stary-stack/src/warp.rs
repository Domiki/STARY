use std::{ops::Range, sync::OnceLock};

use rayon::prelude::*;
use stary_register::Transform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kernel {
    Bilinear,
    Bicubic,
    Lanczos3,
}

const LANCZOS_LOBES: f32 = 3.0;
const LANCZOS_STEPS: usize = 1024;
const MAX_TAPS: usize = 6;

impl Kernel {
    const fn taps(self) -> usize {
        match self {
            Self::Bilinear => 2,
            Self::Bicubic => 4,
            Self::Lanczos3 => 6,
        }
    }
}

fn lanczos_table() -> &'static [f32] {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let sinc = |x: f32| {
            if x.abs() < 1e-6 {
                1.0
            } else {
                (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x)
            }
        };
        (0..=LANCZOS_STEPS * LANCZOS_LOBES as usize)
            .map(|index| {
                let x = index as f32 / LANCZOS_STEPS as f32;
                sinc(x) * sinc(x / LANCZOS_LOBES)
            })
            .collect()
    })
}

fn lanczos(distance: f32) -> f32 {
    let position = distance.abs() * LANCZOS_STEPS as f32;
    let table = lanczos_table();
    let index = position as usize;
    if index + 1 >= table.len() {
        return 0.0;
    }
    let fraction = position - index as f32;
    table[index] * (1.0 - fraction) + table[index + 1] * fraction
}

fn catmull_rom(distance: f32) -> f32 {
    let x = distance.abs();
    if x < 1.0 {
        1.5 * x * x * x - 2.5 * x * x + 1.0
    } else if x < 2.0 {
        -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0
    } else {
        0.0
    }
}

/// The weights of the taps around a position whose fractional part is `fraction`, for taps at
/// offsets `-(taps/2 - 1) ..= taps/2`.
fn weights(kernel: Kernel, fraction: f32) -> [f32; MAX_TAPS] {
    let mut out = [0.0; MAX_TAPS];
    let first = 1 - (kernel.taps() / 2) as i32;
    for (index, weight) in out.iter_mut().take(kernel.taps()).enumerate() {
        let distance = (first + index as i32) as f32 - fraction;
        *weight = match kernel {
            Kernel::Bilinear => 1.0 - distance.abs(),
            Kernel::Bicubic => catmull_rom(distance),
            Kernel::Lanczos3 => lanczos(distance),
        };
    }
    if kernel == Kernel::Lanczos3 {
        let total: f32 = out.iter().sum();
        out.iter_mut().for_each(|weight| *weight /= total);
    }
    out
}

/// A plane of 16-bit samples to resample.
#[derive(Debug, Clone, Copy)]
pub struct SourcePlane<'a> {
    pub width: usize,
    pub height: usize,
    pub data: &'a [u16],
}

impl SourcePlane<'_> {
    fn at(&self, x: isize, y: isize) -> f32 {
        let column = x.clamp(0, self.width as isize - 1) as usize;
        let row = y.clamp(0, self.height as isize - 1) as usize;
        f32::from(self.data[row * self.width + column])
    }

    fn sample(&self, kernel: Kernel, x: f64, y: f64) -> f32 {
        let (floor_x, floor_y) = (x.floor(), y.floor());
        let (wx, wy) = (
            weights(kernel, (x - floor_x) as f32),
            weights(kernel, (y - floor_y) as f32),
        );
        let (ix, iy) = (floor_x as isize, floor_y as isize);
        let first = 1 - (kernel.taps() / 2) as isize;
        let mut total = 0.0;
        for (row, weight_y) in wy.iter().take(kernel.taps()).enumerate() {
            let line: f32 = wx
                .iter()
                .take(kernel.taps())
                .enumerate()
                .map(|(column, weight_x)| {
                    weight_x * self.at(ix + first + column as isize, iy + first + row as isize)
                })
                .sum();
            total += weight_y * line;
        }
        if kernel == Kernel::Bilinear {
            return total;
        }
        let corners = [
            self.at(ix, iy),
            self.at(ix + 1, iy),
            self.at(ix, iy + 1),
            self.at(ix + 1, iy + 1),
        ];
        let low = corners.iter().copied().fold(f32::MAX, f32::min);
        let high = corners.iter().copied().fold(f32::MIN, f32::max);
        total.clamp(low, high)
    }
}

/// Resamples `rows` of the output grid from `source`. `to_source` maps output coordinates to
/// source coordinates. Output samples that fall outside the source are NaN.
pub fn warp_rows(
    source: &SourcePlane,
    to_source: &Transform,
    kernel: Kernel,
    rows: Range<usize>,
    width: usize,
    out: &mut [f32],
) {
    debug_assert_eq!(out.len(), rows.len() * width);
    let (max_x, max_y) = (source.width as f64 - 1.0, source.height as f64 - 1.0);
    out.par_chunks_mut(width).zip(rows).for_each(|(line, row)| {
        for (column, value) in line.iter_mut().enumerate() {
            let (x, y) = to_source.apply((column as f64, row as f64));
            *value = if (0.0..=max_x).contains(&x) && (0.0..=max_y).contains(&y) {
                source.sample(kernel, x, y)
            } else {
                f32::NAN
            };
        }
    });
}

/// Resamples `rows` of the output grid from `source` like `warp_rows` and keeps the larger of the
/// result and what `kept` holds. Positions outside the source change nothing, and neither do
/// values that do not exceed `floor`: stacking many shifted copies of the sky noise would raise the
/// background otherwise.
pub fn warp_max_rows(
    source: &SourcePlane,
    to_source: &Transform,
    kernel: Kernel,
    rows: Range<usize>,
    width: usize,
    kept: &mut [f32],
    floor: f32,
) {
    debug_assert_eq!(kept.len(), rows.len() * width);
    let (max_x, max_y) = (source.width as f64 - 1.0, source.height as f64 - 1.0);
    kept.par_chunks_mut(width)
        .zip(rows)
        .for_each(|(line, row)| {
            for (column, value) in line.iter_mut().enumerate() {
                let (x, y) = to_source.apply((column as f64, row as f64));
                if (0.0..=max_x).contains(&x) && (0.0..=max_y).contains(&y) {
                    let sampled = source.sample(kernel, x, y);
                    if sampled > floor {
                        *value = value.max(sampled);
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(width: usize, height: usize) -> Vec<u16> {
        (0..width * height)
            .map(|index| (1000 + 10 * (index % width) + 7 * (index / width)) as u16)
            .collect()
    }

    fn warp_all(
        data: &[u16],
        width: usize,
        height: usize,
        to_source: &Transform,
        kernel: Kernel,
    ) -> Vec<f32> {
        let source = SourcePlane {
            width,
            height,
            data,
        };
        let mut out = vec![0.0; width * height];
        warp_rows(&source, to_source, kernel, 0..height, width, &mut out);
        out
    }

    #[test]
    fn the_identity_gives_the_samples_back_for_every_kernel() {
        let data = ramp(16, 12);
        for kernel in [Kernel::Bilinear, Kernel::Bicubic, Kernel::Lanczos3] {
            let out = warp_all(&data, 16, 12, &Transform::IDENTITY, kernel);
            for (a, b) in out.iter().zip(&data) {
                assert!((a - f32::from(*b)).abs() < 0.01, "{kernel:?}");
            }
        }
    }

    #[test]
    fn a_half_pixel_shift_of_a_ramp_is_exact_inside() {
        let data = ramp(24, 16);
        let to_source = Transform::from_affine(1.0, 0.0, 0.5, 0.0, 1.0, 0.25);
        for kernel in [Kernel::Bilinear, Kernel::Bicubic, Kernel::Lanczos3] {
            let out = warp_all(&data, 24, 16, &to_source, kernel);
            for row in 4..10 {
                for column in 4..18 {
                    let expected =
                        1000.0 + 10.0 * (column as f32 + 0.5) + 7.0 * (row as f32 + 0.25);
                    let got = out[row * 24 + column];
                    assert!(
                        (got - expected).abs() < 0.5,
                        "{kernel:?} {row},{column}: {got} vs {expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn positions_outside_the_source_are_not_a_number() {
        let data = ramp(8, 8);
        let to_source = Transform::from_affine(1.0, 0.0, 3.0, 0.0, 1.0, 0.0);
        let out = warp_all(&data, 8, 8, &to_source, Kernel::Bicubic);
        assert!(out[0].is_finite());
        assert!(out[4].is_finite());
        assert!(out[5].is_nan());
        assert!(out[7].is_nan());
    }

    #[test]
    fn a_bright_point_does_not_ring_below_the_background() {
        let mut data = vec![1000u16; 32 * 32];
        data[16 * 32 + 16] = 30000;
        let to_source = Transform::from_affine(1.0, 0.0, 0.4, 0.0, 1.0, 0.4);
        for kernel in [Kernel::Bicubic, Kernel::Lanczos3] {
            let out = warp_all(&data, 32, 32, &to_source, kernel);
            assert!(
                out.iter()
                    .filter(|value| value.is_finite())
                    .all(|value| *value >= 1000.0 - 1e-3 && *value <= 30000.0 + 1e-3),
                "{kernel:?}"
            );
        }
    }

    #[test]
    fn a_strip_matches_the_same_rows_of_the_whole_image() {
        let data = ramp(20, 20);
        let to_source = Transform::from_affine(0.98, -0.05, 1.5, 0.05, 0.98, -0.7);
        let whole = warp_all(&data, 20, 20, &to_source, Kernel::Lanczos3);
        let source = SourcePlane {
            width: 20,
            height: 20,
            data: &data,
        };
        let mut strip = vec![0.0; 6 * 20];
        warp_rows(&source, &to_source, Kernel::Lanczos3, 7..13, 20, &mut strip);
        for (a, b) in strip.iter().zip(&whole[7 * 20..13 * 20]) {
            assert!(a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()));
        }
    }

    #[test]
    fn the_larger_of_the_old_and_the_new_value_is_kept() {
        let data = ramp(16, 8);
        let source = SourcePlane {
            width: 16,
            height: 8,
            data: &data,
        };
        let mut kept = vec![1050.0; 16 * 8];
        warp_max_rows(
            &source,
            &Transform::IDENTITY,
            Kernel::Bilinear,
            0..8,
            16,
            &mut kept,
            0.0,
        );
        for (index, value) in kept.iter().enumerate() {
            assert_eq!(*value, f32::from(data[index]).max(1050.0));
        }
    }

    #[test]
    fn values_that_do_not_exceed_the_floor_change_nothing() {
        let mut data = vec![1000u16; 8 * 8];
        data[4 * 8 + 4] = 5000;
        let source = SourcePlane {
            width: 8,
            height: 8,
            data: &data,
        };
        let mut kept = vec![0.0; 8 * 8];
        warp_max_rows(
            &source,
            &Transform::IDENTITY,
            Kernel::Bilinear,
            0..8,
            8,
            &mut kept,
            2000.0,
        );
        assert_eq!(kept[4 * 8 + 4], 5000.0);
        assert_eq!(kept.iter().filter(|value| **value > 0.0).count(), 1);
    }

    #[test]
    fn positions_outside_the_source_keep_what_was_there() {
        let data = ramp(8, 4);
        let source = SourcePlane {
            width: 8,
            height: 4,
            data: &data,
        };
        let mut kept = vec![5.0; 8 * 4];
        let away = Transform::from_affine(1.0, 0.0, 100.0, 0.0, 1.0, 0.0);
        warp_max_rows(&source, &away, Kernel::Bilinear, 0..4, 8, &mut kept, 0.0);
        assert!(kept.iter().all(|value| *value == 5.0));
    }

    #[test]
    fn kernel_weights_add_up_to_one() {
        for kernel in [Kernel::Bilinear, Kernel::Bicubic, Kernel::Lanczos3] {
            for step in 0..10 {
                let total: f32 = weights(kernel, step as f32 / 10.0).iter().sum();
                assert!((total - 1.0).abs() < 1e-3, "{kernel:?} {step}: {total}");
            }
        }
    }
}
