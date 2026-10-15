use std::path::{Path, PathBuf};

use rayon::prelude::*;
use stary_calib::CalibrationSet;
use stary_core::COLORS;
use stary_register::{DetectParams, Star, find_stars};
use stary_stack::{SkyLevel, estimate_sky};

use crate::{
    decode_stream::DecodeFn,
    demosaicer::Demosaicer,
    error::PipelineError,
    master_build::geometry_of,
    planes::{Planes, split_planes},
};

const SATURATION: f32 = 0.9 * 65535.0;

/// What is known about a frame after it has been decoded, calibrated and demosaiced.
pub struct Prepared {
    pub stars: Vec<Star>,
    pub sky: [SkyLevel; COLORS],
    pub planes: Option<Planes>,
}

pub use crate::calibrate::Observer;

pub struct PrepareContext<'a> {
    pub lights: &'a [PathBuf],
    pub calibration: Option<&'a CalibrationSet>,
    pub demosaicer: &'a Demosaicer,
    pub decode: DecodeFn,
    pub detect: DetectParams,
    pub observer: Observer<'a>,
}

/// The average of the three colors over blocks of 2x2 pixels, one fourth the size of the frame.
fn luminance_binned(planes: &Planes) -> (Vec<f32>, usize, usize) {
    let (width, height) = (planes.width / 2, planes.height / 2);
    let [red, green, blue] = &planes.data;
    let mut out = vec![0.0f32; width * height];
    out.par_chunks_mut(width)
        .enumerate()
        .for_each(|(row, line)| {
            let (top, bottom) = (2 * row * planes.width, (2 * row + 1) * planes.width);
            for (column, value) in line.iter_mut().enumerate() {
                let at = |offset: usize| {
                    let index = offset + 2 * column;
                    [index, index + 1]
                        .iter()
                        .map(|&i| f32::from(red[i]) + f32::from(green[i]) + f32::from(blue[i]))
                        .sum::<f32>()
                };
                *value = (at(top) + at(bottom)) / 12.0;
            }
        });
    (out, width, height)
}

/// Stars found in the half-size luminance, placed back on the pixels of the frame.
fn find_frame_stars(planes: &Planes, detect: &DetectParams) -> Vec<Star> {
    let (luma, width, height) = luminance_binned(planes);
    let mut stars = find_stars(width, height, luma, detect);
    for star in &mut stars {
        star.x = 2.0 * star.x + 0.5;
        star.y = 2.0 * star.y + 0.5;
        star.fwhm *= 2.0;
    }
    stars
}

/// Decodes, calibrates and demosaics one light. `observe` lets the caller see the calibrated
/// frame (to save it); it is off when a frame is made again for a later strip.
pub fn demosaiced_planes(
    context: &PrepareContext,
    index: usize,
    observe: bool,
) -> Result<Planes, PipelineError> {
    let path: &Path = &context.lights[index];
    let mut frame = (context.decode)(path)?;
    if let Some(set) = context.calibration {
        set.apply(&mut frame.samples, frame.black, frame.maximum)
            .map_err(|source| PipelineError::Geometry {
                path: path.to_path_buf(),
                source,
            })?;
    }
    debug_assert_eq!(geometry_of(&frame).len(), frame.samples.len());
    if observe {
        (context.observer)(index, &frame)?;
    }
    let image = context.demosaicer.run(&frame, path)?;
    drop(frame);
    Ok(split_planes(&image.samples, image.width, image.height))
}

pub fn prepare_frame(
    context: &PrepareContext,
    index: usize,
    keep_planes: bool,
) -> Result<Prepared, PipelineError> {
    let planes = demosaiced_planes(context, index, true)?;
    let sky = std::array::from_fn(|channel| estimate_sky(&planes.data[channel], planes.width));
    let stars = find_frame_stars(
        &planes,
        &DetectParams {
            saturation: SATURATION,
            ..context.detect
        },
    );
    Ok(Prepared {
        stars,
        sky,
        planes: keep_planes.then_some(planes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_of_a_small_frame_are_found_and_placed_on_the_frame_pixels() {
        let (width, height) = (320usize, 240usize);
        let truth = [
            (60.0f64, 50.0f64),
            (150.5, 120.5),
            (250.0, 80.25),
            (100.0, 190.0),
        ];
        let mut gray = vec![3000.0f32; width * height];
        for &(sx, sy) in &truth {
            for y in 0..height {
                for x in 0..width {
                    let d2 = (x as f64 - sx).powi(2) + (y as f64 - sy).powi(2);
                    gray[y * width + x] += 30000.0 * (-d2 / (2.0 * 1.7 * 1.7)).exp() as f32;
                }
            }
        }
        let mut rng = 12345u64;
        let mut interleaved = Vec::with_capacity(width * height * 3);
        for value in &gray {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let noise = ((rng >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 12.0;
            let sample = (value + noise).clamp(0.0, 65535.0) as u16;
            interleaved.extend_from_slice(&[sample; 3]);
        }
        let planes = split_planes(&interleaved, width, height);
        let stars = find_frame_stars(
            &planes,
            &DetectParams {
                threshold_sigma: 8.0,
                ..DetectParams::default()
            },
        );
        assert!(
            stars.len() >= truth.len(),
            "{} stars: {stars:?}",
            stars.len()
        );
        for (sx, sy) in truth {
            let nearest = stars
                .iter()
                .map(|star| (star.x - sx).hypot(star.y - sy))
                .fold(f64::MAX, f64::min);
            assert!(
                nearest < 0.5,
                "star at {sx},{sy} is {nearest} px from the nearest detection"
            );
        }
    }

    #[test]
    fn the_binned_luminance_averages_blocks_and_channels() {
        // 2x2 pixels, every channel of the first row is 3, 6 or 9 and of the second row 0, 0, 12
        let interleaved = [3, 3, 3, 6, 6, 6, 0, 0, 0, 12, 12, 12];
        let planes = split_planes(&interleaved, 2, 2);
        let (luma, width, height) = luminance_binned(&planes);
        assert_eq!((width, height), (1, 1));
        assert_eq!(luma, vec![5.25]);
    }
}
