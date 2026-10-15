use rayon::prelude::*;

use stary_core::COLORS;

use crate::{
    geometry::{GeometryError, SensorGeometry},
    master::Master,
};

pub const GAIN_ONE: u16 = 1 << 13;
const MIN_GAIN: f32 = 0.25;
const MAX_GAIN: f32 = 3.99;
const DEAD_PIXEL_FRACTION: f32 = 0.05;
const SUBTRACTED_ALREADY_FRACTION: f32 = 0.01;

type ColorSums = [(f64, f64); COLORS];

fn row_sums(geometry: &SensorGeometry, signal: &[f32], row: usize) -> ColorSums {
    let mut sums = [(0.0, 0.0); COLORS];
    let row_len = geometry.width * geometry.layout.channels();
    let start = row * row_len;
    for (offset, value) in signal[start..start + row_len].iter().enumerate() {
        let color = geometry.color_of(start + offset);
        sums[color].0 += f64::from(*value);
        sums[color].1 += 1.0;
    }
    sums
}

fn color_means(geometry: &SensorGeometry, signal: &[f32]) -> [f32; COLORS] {
    let totals = (0..geometry.height)
        .into_par_iter()
        .map(|row| row_sums(geometry, signal, row))
        .reduce(
            || [(0.0, 0.0); COLORS],
            |mut a, b| {
                a.iter_mut().zip(b).for_each(|(x, y)| {
                    x.0 += y.0;
                    x.1 += y.1;
                });
                a
            },
        );
    totals.map(|(sum, count)| {
        if count > 0.0 {
            (sum / count) as f32
        } else {
            0.0
        }
    })
}

fn gain_for(signal: f32, mean: f32) -> u16 {
    if mean <= 0.0 || signal < mean * DEAD_PIXEL_FRACTION {
        return GAIN_ONE;
    }
    let gain = (mean / signal).clamp(MIN_GAIN, MAX_GAIN);
    (gain * f32::from(GAIN_ONE)).round() as u16
}

/// A raw flat never lies below its own offset, so a flat that does so over a good part of the
/// frame (its dark corners) has had the offset taken out already.
fn lost_its_offset(flat: &Master, bias: &Master) -> bool {
    let below = flat
        .samples
        .iter()
        .zip(&bias.samples)
        .filter(|(f, b)| f < b)
        .count();
    below as f32 > SUBTRACTED_ALREADY_FRACTION * flat.samples.len() as f32
}

/// Per-pixel multiplier (fixed point, 1.0 = `GAIN_ONE`) that flattens the master flat.
/// Each color is normalised on its own, so the flat's light color does not tint the result.
pub fn flat_gain(flat: &Master, bias: Option<&Master>) -> Result<Vec<u16>, GeometryError> {
    let geometry = flat.geometry;
    if let Some(bias) = bias {
        geometry.check_matches(&bias.geometry)?;
    }
    let bias = bias.filter(|bias| !flat.offset_removed && !lost_its_offset(flat, bias));
    let signal: Vec<f32> = match bias {
        Some(bias) => flat
            .samples
            .iter()
            .zip(&bias.samples)
            .map(|(f, b)| f - b)
            .collect(),
        None if flat.offset_removed => flat.samples.clone(),
        None => flat
            .samples
            .iter()
            .map(|f| f - f32::from(flat.black))
            .collect(),
    };
    let means = color_means(&geometry, &signal);
    Ok(signal
        .par_iter()
        .enumerate()
        .map(|(index, value)| gain_for(*value, means[geometry.color_of(index)]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stary_core::{BayerPattern, CfaPattern, PixelLayout};

    fn geometry(width: usize, height: usize) -> SensorGeometry {
        SensorGeometry {
            width,
            height,
            layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        }
    }

    /// A flat with vignetting (darker towards the right) and a different level per color.
    fn vignetted_flat(width: usize, height: usize, black: f32) -> Master {
        let geometry = geometry(width, height);
        let samples = (0..width * height)
            .map(|index| {
                let column = index % width;
                let color_level = [3000.0, 5000.0, 4000.0][geometry.color_of(index)];
                let vignette = 1.0 - 0.4 * column as f32 / width as f32;
                black + color_level * vignette
            })
            .collect();
        Master::new(geometry, black as u16, 10, samples).unwrap()
    }

    #[test]
    fn a_flat_with_dark_corners_below_the_bias_is_taken_to_be_reduced_already() {
        let (width, height) = (64, 8);
        let bias = Master::new(
            geometry(width, height),
            1000,
            10,
            vec![1000.0; width * height],
        )
        .unwrap();
        // a flat of 1100..400 counts: its right part lies below the bias level of 1000
        let samples = (0..width * height)
            .map(|index| 1100.0 - 700.0 * (index % width) as f32 / width as f32)
            .collect();
        let flat = Master::new(geometry(width, height), 0, 10, samples).unwrap();
        assert!(lost_its_offset(&flat, &bias));
        assert_eq!(
            flat_gain(&flat, Some(&bias)).unwrap(),
            flat_gain(&flat, None).unwrap()
        );
    }

    #[test]
    fn a_raw_flat_is_reduced_by_the_bias() {
        let raw = vignetted_flat(64, 8, 1000.0);
        let bias = Master::new(geometry(64, 8), 1000, 10, vec![1000.0; 64 * 8]).unwrap();
        assert!(!lost_its_offset(&raw, &bias));
        // the bias is subtracted: the gain flattens the signal above the offset
        let gain = flat_gain(&raw, Some(&bias)).unwrap();
        let flattened = raw.samples[0] - 1000.0;
        let corner = raw.samples[63 / 2 * 2] - 1000.0;
        let ratio = (flattened * f32::from(gain[0])) / (corner * f32::from(gain[63 / 2 * 2]));
        assert!((ratio - 1.0).abs() < 0.05, "{ratio}");
    }

    #[test]
    fn a_flat_that_lost_its_offset_is_not_reduced_by_the_bias_again() {
        let flat = vignetted_flat(64, 8, 0.0).with_offset_removed(true);
        let bias = Master::new(geometry(64, 8), 1000, 10, vec![1000.0; 64 * 8]).unwrap();
        assert_eq!(
            flat_gain(&flat, Some(&bias)).unwrap(),
            flat_gain(&flat, None).unwrap()
        );
    }

    #[test]
    fn applying_the_gain_makes_each_color_uniform() {
        let flat = vignetted_flat(64, 8, 1000.0);
        let gain = flat_gain(&flat, None).unwrap();
        let geometry = flat.geometry;
        let mut per_color: [Vec<f32>; 3] = Default::default();
        for (index, g) in gain.iter().enumerate() {
            let color = geometry.color_of(index);
            per_color[color]
                .push((flat.samples[index] - 1000.0) * f32::from(*g) / f32::from(GAIN_ONE));
        }
        for values in per_color {
            let min = values.iter().cloned().fold(f32::MAX, f32::min);
            let max = values.iter().cloned().fold(f32::MIN, f32::max);
            assert!(
                (max - min) / max < 0.01,
                "color not flattened: {min}..{max}"
            );
        }
    }

    #[test]
    fn a_uniform_flat_gives_unity_gain() {
        let geometry = geometry(8, 4);
        let flat = Master::new(geometry, 100, 3, vec![600.0; 32]).unwrap();
        assert!(
            flat_gain(&flat, None)
                .unwrap()
                .iter()
                .all(|g| *g == GAIN_ONE)
        );
    }

    #[test]
    fn dead_pixels_are_left_alone() {
        let geometry = geometry(4, 2);
        let mut samples = vec![600.0; 8];
        samples[3] = 100.0;
        let flat = Master::new(geometry, 100, 3, samples).unwrap();
        assert_eq!(flat_gain(&flat, None).unwrap()[3], GAIN_ONE);
    }

    #[test]
    fn a_bias_master_replaces_the_flat_black_level() {
        let geometry = geometry(4, 2);
        let flat = Master::new(geometry, 0, 3, vec![1100.0; 8]).unwrap();
        let bias = Master::new(geometry, 0, 3, vec![1000.0; 8]).unwrap();
        assert!(
            flat_gain(&flat, Some(&bias))
                .unwrap()
                .iter()
                .all(|g| *g == GAIN_ONE)
        );
    }

    #[test]
    fn mismatched_geometry_is_an_error() {
        let flat = Master::new(geometry(4, 2), 0, 1, vec![1.0; 8]).unwrap();
        let bias = Master::new(geometry(2, 2), 0, 1, vec![1.0; 4]).unwrap();
        assert_eq!(
            flat_gain(&flat, Some(&bias)),
            Err(GeometryError::SizeMismatch)
        );
    }
}
