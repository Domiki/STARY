use rayon::prelude::*;

use crate::{
    background::{Background, median},
    bin::bin_by_two,
    star::{GrayImage, Star},
};

const SMOOTH_SIGMA: f32 = 1.2;
const SMOOTH_RADIUS: usize = 3;
const NOISE_SAMPLE_STRIDE: usize = 3;
const MAD_TO_SIGMA: f32 = 1.4826;
const FWHM_PER_SIGMA: f32 = 2.3548;
const MIN_WINDOW: f32 = 4.0;
const MAX_WINDOW: f32 = 30.0;
const WINDOW_PER_SIGMA: f32 = 3.0;
const WINDOW_ROUNDS: usize = 8;
/// At a shrunken level a star is taken to be located well when it is this wide (in the pixels of
/// that level); at the full size of the image sharper stars are taken too.
const LEVEL_FWHM: std::ops::RangeInclusive<f32> = 2.0..=14.0;
const FINEST_MIN_FWHM: f32 = 1.2;
const MAX_BINS: u32 = 16;
const MIN_LEVEL_EDGE: usize = 100;
const MIN_ROUNDNESS: f32 = 0.4;
const FOOTPRINT: f64 = 0.75;

#[derive(Debug, Clone, Copy)]
pub struct DetectParams {
    /// Detection threshold in units of the noise of the smoothed image.
    pub threshold_sigma: f32,
    pub max_stars: usize,
    /// Stars whose peak reaches this level are saturated and left out.
    pub saturation: f32,
}

impl Default for DetectParams {
    fn default() -> Self {
        Self {
            threshold_sigma: 6.0,
            max_stars: 500,
            saturation: f32::INFINITY,
        }
    }
}

fn kernel() -> [f32; 2 * SMOOTH_RADIUS + 1] {
    let mut weights = [0.0; 2 * SMOOTH_RADIUS + 1];
    for (index, weight) in weights.iter_mut().enumerate() {
        let offset = index as f32 - SMOOTH_RADIUS as f32;
        *weight = (-offset * offset / (2.0 * SMOOTH_SIGMA * SMOOTH_SIGMA)).exp();
    }
    let total: f32 = weights.iter().sum();
    weights.map(|weight| weight / total)
}

fn smooth(image: &[f32], width: usize, height: usize) -> Vec<f32> {
    let weights = kernel();
    let mut horizontal = vec![0.0f32; image.len()];
    horizontal
        .par_chunks_mut(width)
        .zip(image.par_chunks(width))
        .for_each(|(out, line)| {
            for (x, value) in out.iter_mut().enumerate() {
                *value = weights
                    .iter()
                    .enumerate()
                    .map(|(tap, weight)| {
                        let source = (x + tap).saturating_sub(SMOOTH_RADIUS).min(width - 1);
                        weight * line[source]
                    })
                    .sum();
            }
        });
    let mut out = vec![0.0f32; image.len()];
    out.par_chunks_mut(width).enumerate().for_each(|(y, line)| {
        for (tap, weight) in weights.iter().enumerate() {
            let source = (y + tap).saturating_sub(SMOOTH_RADIUS).min(height - 1);
            let row = &horizontal[source * width..(source + 1) * width];
            for (value, input) in line.iter_mut().zip(row) {
                *value += weight * input;
            }
        }
    });
    out
}

fn noise_of(image: &[f32], width: usize) -> f32 {
    let mut samples: Vec<f32> = image
        .chunks(width)
        .step_by(NOISE_SAMPLE_STRIDE)
        .flat_map(|line| line.iter().step_by(NOISE_SAMPLE_STRIDE).copied())
        .collect();
    let center = median(&mut samples);
    let mut deviations: Vec<f32> = samples.iter().map(|value| (value - center).abs()).collect();
    MAD_TO_SIGMA * median(&mut deviations)
}

fn is_local_maximum(smoothed: &[f32], width: usize, x: usize, y: usize) -> bool {
    let center = smoothed[y * width + x];
    for (dx, dy) in [
        (-1isize, -1isize),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        let neighbour = smoothed[(y as isize + dy) as usize * width + (x as isize + dx) as usize];
        let earlier = dy < 0 || (dy == 0 && dx < 0);
        if neighbour > center || (earlier && neighbour == center) {
            return false;
        }
    }
    true
}

/// A star measured in the image it was found in.
struct Measured {
    x: f64,
    y: f64,
    flux: f32,
    peak: f32,
    fwhm: f32,
    roundness: f32,
}

/// Locates a star by a flux-weighted centroid in a circle that grows with the star: the window
/// starts small and follows the width measured from the pixels inside it.
fn measure(
    sub: &[f32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    floor: f32,
) -> Option<Measured> {
    let (mut cx, mut cy, mut radius) = (x as f64, y as f64, MIN_WINDOW);
    let mut spread = (0.0f32, 0.0f32);
    for _ in 0..WINDOW_ROUNDS {
        let reach = radius.ceil() as isize;
        let (x0, y0) = (cx.round() as isize, cy.round() as isize);
        if x0 - reach < 0
            || y0 - reach < 0
            || x0 + reach >= width as isize
            || y0 + reach >= height as isize
        {
            return None;
        }
        let limit = f64::from(radius * radius) + 1.0;
        let (mut total, mut sx, mut sy) = (0.0f64, 0.0f64, 0.0f64);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                if (dx * dx + dy * dy) as f64 > limit {
                    continue;
                }
                let value = sub[(y0 + dy) as usize * width + (x0 + dx) as usize];
                let weight = f64::from((value - floor).max(0.0));
                total += weight;
                sx += weight * (x0 + dx) as f64;
                sy += weight * (y0 + dy) as f64;
            }
        }
        if total <= 0.0 {
            return None;
        }
        (cx, cy) = (sx / total, sy / total);
        let (mut vx, mut vy) = (0.0f64, 0.0f64);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                if (dx * dx + dy * dy) as f64 > limit {
                    continue;
                }
                let value = sub[(y0 + dy) as usize * width + (x0 + dx) as usize];
                let weight = f64::from((value - floor).max(0.0));
                vx += weight * ((x0 + dx) as f64 - cx).powi(2);
                vy += weight * ((y0 + dy) as f64 - cy).powi(2);
            }
        }
        spread = ((vx / total).sqrt() as f32, (vy / total).sqrt() as f32);
        let sigma = 0.5 * (spread.0 + spread.1);
        radius = (WINDOW_PER_SIGMA * sigma).clamp(MIN_WINDOW, MAX_WINDOW);
    }
    let reach = radius.ceil() as isize;
    let (x0, y0) = (cx.round() as isize, cy.round() as isize);
    if x0 - reach < 0
        || y0 - reach < 0
        || x0 + reach >= width as isize
        || y0 + reach >= height as isize
    {
        return None;
    }
    let limit = f64::from(radius * radius) + 1.0;
    let (mut flux, mut peak) = (0.0f32, f32::MIN);
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            if (dx * dx + dy * dy) as f64 > limit {
                continue;
            }
            let value = sub[(y0 + dy) as usize * width + (x0 + dx) as usize];
            flux += value.max(0.0);
            peak = peak.max(value);
        }
    }
    let (wide, narrow) = (spread.0.max(spread.1), spread.0.min(spread.1));
    (flux > 0.0 && peak > 0.0 && wide > 0.0).then_some(Measured {
        x: cx,
        y: cy,
        flux,
        peak,
        fwhm: FWHM_PER_SIGMA * 0.5 * (spread.0 + spread.1),
        roundness: narrow / wide,
    })
}

/// Stars of one image at one scale, with their sizes in the pixels of that image.
fn stars_at_scale(image: &GrayImage, params: &DetectParams) -> Vec<Measured> {
    let (width, height) = (image.width, image.height);
    let margin = MIN_WINDOW as usize + 2;
    if width <= 2 * margin || height <= 2 * margin {
        return Vec::new();
    }
    let background = Background::estimate(image);
    let mut sub = image.data.to_vec();
    sub.par_chunks_mut(width).enumerate().for_each(|(y, line)| {
        for (x, value) in line.iter_mut().enumerate() {
            *value -= background.at(x, y);
        }
    });
    let smoothed = smooth(&sub, width, height);
    let threshold = params.threshold_sigma * noise_of(&smoothed, width);
    let floor = background.sigma;
    (margin..height - margin)
        .into_par_iter()
        .flat_map_iter(|y| {
            let smoothed = &smoothed;
            (margin..width - margin)
                .filter(move |&x| {
                    smoothed[y * width + x] > threshold && is_local_maximum(smoothed, width, x, y)
                })
                .map(move |x| (x, y))
        })
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|(x, y)| {
            let measured = measure(&sub, width, height, x, y, floor)?;
            let raw_peak = measured.peak + background.at(x, y);
            let wanted = measured.roundness >= MIN_ROUNDNESS && raw_peak < params.saturation;
            wanted.then_some(measured)
        })
        .collect()
}

/// Merges neighbouring detections into the brightest of them. Large stars can show several maxima.
fn without_duplicates(mut stars: Vec<Measured>) -> Vec<Measured> {
    stars.sort_by(|a, b| b.flux.total_cmp(&a.flux));
    let mut kept: Vec<Measured> = Vec::with_capacity(stars.len());
    for star in stars {
        let near = kept.iter().any(|other| {
            (other.x - star.x).hypot(other.y - star.y)
                < f64::from(other.fwhm.max(star.fwhm).max(2.0))
        });
        if !near {
            kept.push(star);
        }
    }
    kept
}

struct Level {
    factor: u32,
    data: Vec<f32>,
    width: usize,
    height: usize,
}

impl Level {
    fn halved(&self) -> Self {
        let (data, width, height) = bin_by_two(&self.data, self.width, self.height);
        Self {
            factor: self.factor * 2,
            data,
            width,
            height,
        }
    }
}

fn pyramid(width: usize, height: usize, data: Vec<f32>) -> Vec<Level> {
    let mut levels = vec![Level {
        factor: 1,
        data,
        width,
        height,
    }];
    while levels.last().is_some_and(|last| {
        last.factor < MAX_BINS && last.width.min(last.height) / 2 >= MIN_LEVEL_EDGE
    }) {
        let next = levels.last().expect("a level").halved();
        levels.push(next);
    }
    levels
}

/// A star placed on the pixels of the image given to `find_stars`.
fn placed(star: &Measured, factor: u32) -> Star {
    let scale = f64::from(factor);
    Star {
        x: (star.x + 0.5) * scale - 0.5,
        y: (star.y + 0.5) * scale - 0.5,
        flux: star.flux * (factor * factor) as f32,
        peak: star.peak,
        fwhm: star.fwhm * factor as f32,
        roundness: star.roundness,
    }
}

/// Finds the stars of an image, brightest first. The image is searched at several sizes (halved
/// again and again) so that wide stars are located as well as sharp ones: a star is taken at the
/// coarsest size where it is a few pixels wide, and nothing finer inside its footprint counts as
/// another star. Positions are in the pixels of the given image.
pub fn find_stars(width: usize, height: usize, data: Vec<f32>, params: &DetectParams) -> Vec<Star> {
    if data.len() != width * height {
        return Vec::new();
    }
    let levels = pyramid(width, height, data);
    let mut accepted: Vec<Star> = Vec::new();
    for level in levels.iter().rev() {
        let image = GrayImage {
            width: level.width,
            height: level.height,
            data: &level.data,
        };
        let lowest = if level.factor == 1 {
            FINEST_MIN_FWHM
        } else {
            *LEVEL_FWHM.start()
        };
        let fresh: Vec<Measured> = stars_at_scale(&image, params)
            .into_iter()
            .filter(|star| (lowest..=*LEVEL_FWHM.end()).contains(&star.fwhm))
            .collect();
        let mut fresh: Vec<Star> = without_duplicates(fresh)
            .iter()
            .map(|star| placed(star, level.factor))
            .collect();
        fresh.retain(|star| {
            !accepted.iter().any(|other| {
                (other.x - star.x).hypot(other.y - star.y)
                    < FOOTPRINT * f64::from(other.fwhm.max(star.fwhm))
            })
        });
        accepted.extend(fresh);
    }
    accepted.sort_by(|a, b| b.flux.total_cmp(&a.flux));
    accepted.truncate(params.max_stars);
    accepted
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub struct Field {
        pub width: usize,
        pub height: usize,
        pub data: Vec<f32>,
    }

    /// A noisy sky with a gradient and Gaussian stars at the given positions.
    pub fn render(width: usize, height: usize, stars: &[(f64, f64, f32)], seed: u64) -> Field {
        let mut state = seed;
        let mut noise = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as f32 / (1u64 << 31) as f32 - 0.5) * 6.0
        };
        let mut data: Vec<f32> = (0..width * height)
            .map(|index| {
                800.0 + 0.3 * (index % width) as f32 + 0.1 * (index / width) as f32 + noise()
            })
            .collect();
        let sigma = 1.7f64;
        for &(sx, sy, amplitude) in stars {
            for y in (sy as isize - 8).max(0)..(sy as isize + 9).min(height as isize) {
                for x in (sx as isize - 8).max(0)..(sx as isize + 9).min(width as isize) {
                    let d2 = (x as f64 - sx).powi(2) + (y as f64 - sy).powi(2);
                    data[y as usize * width + x as usize] +=
                        amplitude * (-d2 / (2.0 * sigma * sigma)).exp() as f32;
                }
            }
        }
        Field {
            width,
            height,
            data,
        }
    }

    fn stars_of(field: Field, params: &DetectParams) -> Vec<Star> {
        find_stars(field.width, field.height, field.data, params)
    }

    #[test]
    fn stars_are_found_with_subpixel_centres_and_a_hot_pixel_is_ignored() {
        let truth = [
            (50.3, 40.7, 3000.0),
            (120.8, 90.2, 1500.0),
            (200.5, 60.5, 800.0),
        ];
        let mut field = render(256, 160, &truth, 7);
        field.data[100 * 256 + 30] += 5000.0;
        let stars = stars_of(field, &DetectParams::default());
        assert_eq!(stars.len(), 3, "{stars:?}");
        for (sx, sy, _) in truth {
            let found = stars
                .iter()
                .find(|star| (star.x - sx).abs() < 1.0 && (star.y - sy).abs() < 1.0)
                .unwrap_or_else(|| panic!("star at {sx},{sy} missing"));
            assert!(
                (found.x - sx).abs() < 0.15 && (found.y - sy).abs() < 0.15,
                "{found:?}"
            );
            assert!((found.fwhm - 4.0).abs() < 0.8, "{found:?}");
        }
        assert!(stars[0].flux > stars[1].flux && stars[1].flux > stars[2].flux);
    }

    #[test]
    fn saturated_stars_are_left_out() {
        let field = render(128, 96, &[(40.0, 40.0, 60000.0), (90.0, 50.0, 2000.0)], 3);
        let params = DetectParams {
            saturation: 30000.0,
            ..DetectParams::default()
        };
        let stars = stars_of(field, &params);
        assert_eq!(stars.len(), 1);
        assert!((stars[0].x - 90.0).abs() < 0.5);
    }

    #[test]
    fn an_empty_sky_has_no_stars_and_tiny_images_do_not_panic() {
        let field = render(128, 96, &[], 5);
        assert!(stars_of(field, &DetectParams::default()).is_empty());
        assert!(find_stars(5, 5, vec![0.0; 25], &DetectParams::default()).is_empty());
        assert!(find_stars(50, 50, vec![0.0; 10], &DetectParams::default()).is_empty());
    }

    #[test]
    fn the_brightest_stars_are_kept_when_there_are_too_many() {
        let truth: Vec<(f64, f64, f32)> = (0..6)
            .map(|i| {
                (
                    30.0 + 25.0 * f64::from(i),
                    40.0 + 7.0 * f64::from(i),
                    1000.0 + 500.0 * i as f32,
                )
            })
            .collect();
        let field = render(220, 120, &truth, 11);
        let params = DetectParams {
            max_stars: 3,
            ..DetectParams::default()
        };
        let stars = stars_of(field, &params);
        assert_eq!(stars.len(), 3);
        assert!(stars.iter().all(|star| star.x > 30.0 + 25.0 * 2.5));
    }
}
