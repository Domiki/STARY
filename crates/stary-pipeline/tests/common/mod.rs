#![allow(dead_code)]

use std::path::{Path, PathBuf};

use stary_core::{BayerPattern, CfaPattern, Frame, PixelLayout};
use stary_io::master_file::write_calibrated_light;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
pub const SKY: f32 = 3000.0;

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

pub fn star_field() -> Vec<(f64, f64, f32)> {
    let mut rng = Lcg(2024);
    (0..60)
        .map(|_| {
            (
                30.0 + rng.next() * (WIDTH as f64 - 60.0),
                30.0 + rng.next() * (HEIGHT as f64 - 60.0),
                (4000.0 + rng.next() * 40000.0) as f32,
            )
        })
        .collect()
}

/// A Bayer mosaic of the field seen after a shift and a small rotation about the centre.
pub fn mosaic(stars: &[(f64, f64, f32)], shift: (f64, f64), angle: f64, seed: u64) -> Frame {
    mosaic_with_ground(stars, shift, angle, &[], seed)
}

/// Like `mosaic`, with stars that stay where they are in every frame (lights on the ground).
pub fn mosaic_with_ground(
    stars: &[(f64, f64, f32)],
    shift: (f64, f64),
    angle: f64,
    ground: &[(f64, f64, f32)],
    seed: u64,
) -> Frame {
    let mut rng = Lcg(seed);
    let (cx, cy) = (WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0);
    let (cos, sin) = (angle.cos(), angle.sin());
    let placed: Vec<(f64, f64, f32)> = stars
        .iter()
        .map(|&(x, y, a)| {
            let (dx, dy) = (x - cx, y - cy);
            (
                cx + cos * dx - sin * dy + shift.0,
                cy + sin * dx + cos * dy + shift.1,
                a,
            )
        })
        .chain(ground.iter().copied())
        .collect();
    let sigma = 1.7f64;
    let mut samples: Vec<f32> = (0..WIDTH * HEIGHT)
        .map(|index| SKY + 0.5 * (index % WIDTH) as f32 + 12.0 * (rng.next() as f32 - 0.5))
        .collect();
    for (sx, sy, amplitude) in placed {
        for y in (sy as isize - 9).max(0)..(sy as isize + 10).min(HEIGHT as isize) {
            for x in (sx as isize - 9).max(0)..(sx as isize + 10).min(WIDTH as isize) {
                let d2 = (x as f64 - sx).powi(2) + (y as f64 - sy).powi(2);
                samples[y as usize * WIDTH + x as usize] +=
                    amplitude * (-d2 / (2.0 * sigma * sigma)).exp() as f32;
            }
        }
    }
    Frame {
        width: WIDTH,
        height: HEIGHT,
        layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        black: 0,
        maximum: u16::MAX,
        samples: samples
            .iter()
            .map(|value| value.clamp(0.0, 65535.0) as u16)
            .collect(),
    }
}

pub fn write_all(dir: &Path, frames: &[Frame]) -> Vec<PathBuf> {
    std::fs::create_dir_all(dir).unwrap();
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let path = dir.join(format!("light{index:02}.fits"));
            write_calibrated_light(&path, frame).unwrap();
            path
        })
        .collect()
}
