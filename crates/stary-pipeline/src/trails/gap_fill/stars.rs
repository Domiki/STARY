use rayon::prelude::*;
use stary_core::{Frame, PixelLayout};
use stary_register::{DetectParams, Star, find_stars};

const SATURATION_SHARE: f32 = 0.9;

/// The brightness of blocks of 2x2 pixels, one fourth the size of the frame. A sensor frame is
/// used as it is, before it has a color: a block of a Bayer pattern holds all the colors, and the
/// other patterns come close enough to find where the stars are.
fn binned_brightness(frame: &Frame) -> (Vec<f32>, usize, usize) {
    let channels = match frame.layout {
        PixelLayout::Rgb => 3,
        _ => 1,
    };
    let (width, height) = (frame.width / 2, frame.height / 2);
    let row_len = frame.width * channels;
    let mut out = vec![0.0f32; width * height];
    out.par_chunks_mut(width)
        .enumerate()
        .for_each(|(row, line)| {
            let top = &frame.samples[2 * row * row_len..(2 * row + 1) * row_len];
            let bottom = &frame.samples[(2 * row + 1) * row_len..(2 * row + 2) * row_len];
            for (column, value) in line.iter_mut().enumerate() {
                let start = 2 * column * channels;
                let block = |line: &[u16]| -> f32 {
                    line[start..start + 2 * channels]
                        .iter()
                        .map(|&sample| f32::from(sample))
                        .sum()
                };
                *value = (block(top) + block(bottom)) / (4 * channels) as f32;
            }
        });
    (out, width, height)
}

/// Where the stars of a calibrated light are, on the pixels of the frame.
pub fn frame_stars(frame: &Frame, detect: &DetectParams) -> Vec<Star> {
    let (brightness, width, height) = binned_brightness(frame);
    let params = DetectParams {
        saturation: SATURATION_SHARE * f32::from(frame.maximum),
        ..*detect
    };
    let mut stars = find_stars(width, height, brightness, &params);
    for star in &mut stars {
        star.x = 2.0 * star.x + 0.5;
        star.y = 2.0 * star.y + 0.5;
        star.fwhm *= 2.0;
    }
    stars
}

#[cfg(test)]
mod tests {
    use stary_core::{BayerPattern, CfaPattern};

    use super::*;

    const TRUTH: [(f64, f64); 4] = [(60.0, 50.0), (150.5, 120.5), (250.0, 80.25), (100.0, 190.0)];

    fn scene(width: usize, height: usize, channels: usize) -> Vec<u16> {
        let mut rng = 12345u64;
        let mut samples = Vec::with_capacity(width * height * channels);
        for y in 0..height {
            for x in 0..width {
                let mut value = 3000.0f32;
                for &(sx, sy) in &TRUTH {
                    let d2 = (x as f64 - sx).powi(2) + (y as f64 - sy).powi(2);
                    value += 12000.0 * (-d2 / (2.0 * 1.7 * 1.7)).exp() as f32;
                }
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                value += ((rng >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 12.0;
                samples.extend(std::iter::repeat_n(
                    value.clamp(0.0, 65535.0) as u16,
                    channels,
                ));
            }
        }
        samples
    }

    fn detect() -> DetectParams {
        DetectParams {
            threshold_sigma: 8.0,
            ..DetectParams::default()
        }
    }

    fn assert_found(stars: &[Star]) {
        assert!(stars.len() >= TRUTH.len(), "{} stars", stars.len());
        for (sx, sy) in TRUTH {
            let nearest = stars
                .iter()
                .map(|star| (star.x - sx).hypot(star.y - sy))
                .fold(f64::MAX, f64::min);
            assert!(nearest < 0.7, "star at {sx},{sy} is {nearest} px away");
        }
    }

    #[test]
    fn stars_of_a_sensor_frame_are_found_without_a_color_image() {
        let frame = Frame {
            width: 320,
            height: 240,
            layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
            black: 0,
            maximum: u16::MAX,
            samples: scene(320, 240, 1),
        };
        assert_found(&frame_stars(&frame, &detect()));
    }

    #[test]
    fn stars_of_a_color_frame_are_found_on_its_brightness() {
        let frame = Frame {
            width: 320,
            height: 240,
            layout: PixelLayout::Rgb,
            black: 0,
            maximum: u16::MAX,
            samples: scene(320, 240, 3),
        };
        assert_found(&frame_stars(&frame, &detect()));
    }

    #[test]
    fn blocks_are_averaged() {
        let frame = Frame {
            width: 2,
            height: 2,
            layout: PixelLayout::Mono,
            black: 0,
            maximum: u16::MAX,
            samples: vec![4, 8, 12, 16],
        };
        let (brightness, width, height) = binned_brightness(&frame);
        assert_eq!((width, height), (1, 1));
        assert_eq!(brightness, vec![10.0]);
    }
}
