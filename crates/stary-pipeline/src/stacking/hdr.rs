use stary_core::{COLORS, LinearRgbF32};

/// What a stack was exposed with: the light that reaches the sensor grows with both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exposure {
    pub seconds: f64,
    pub iso: f64,
}

impl Exposure {
    fn light(self) -> f64 {
        self.seconds * self.iso
    }

    /// How many times more light `self` collected than `other`.
    pub fn ratio_to(self, other: Self) -> f64 {
        self.light() / other.light()
    }

    pub fn same_as(self, other: Self) -> bool {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-3 * a.abs().max(b.abs());
        close(self.seconds, other.seconds) && close(self.iso, other.iso)
    }
}

/// Below this (as a share of the full scale) a value is far from full, above it a value may have
/// been cut off by the sensor.
const FULL_FROM: f32 = 0.85;
const FULL_TO: f32 = 0.97;
/// The values used to find how the shorter stack maps onto the longer one are neither too dark
/// (noise) nor too bright (cut off).
const FIT_FROM: f32 = 0.05;
const FIT_TO: f32 = 0.6;
const FIT_SAMPLES: usize = 200_000;
const MIN_FIT_SAMPLES: usize = 500;
const TRIM_ROUNDS: usize = 3;
const TRIM_SIGMAS: f64 = 3.0;
/// The fitted scale may differ from the one the exposures say by at most this factor.
const MAX_SCALE_ERROR: f64 = 2.0;

/// The picture made so far and, for every pixel, how full the values it is made of were.
pub struct Layered {
    pub image: LinearRgbF32,
    fullness: Vec<f32>,
}

fn largest_color(pixel: &[f32]) -> f32 {
    pixel.iter().copied().fold(f32::MIN, f32::max)
}

impl Layered {
    /// The longest exposure: its values are the ones that are kept where they are not full.
    pub fn start(image: LinearRgbF32) -> Self {
        let fullness = image
            .samples
            .as_chunks::<COLORS>()
            .0
            .iter()
            .map(|pixel| largest_color(pixel))
            .collect();
        Self { image, fullness }
    }
}

fn smoothstep(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much of the picture made so far stays at a pixel: all of it while it is far from full.
fn keep_share(fullness: f32) -> f32 {
    1.0 - smoothstep(FULL_FROM, FULL_TO, fullness)
}

/// `y = scale * x + offset` through the points, ignoring the ones far from the line.
fn fit_line(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    let mut kept: Vec<(f64, f64)> = points.to_vec();
    let mut line = None;
    for _ in 0..TRIM_ROUNDS {
        let n = kept.len() as f64;
        if kept.len() < MIN_FIT_SAMPLES {
            return line;
        }
        let (sx, sy) = kept.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
        let (mx, my) = (sx / n, sy / n);
        let (sxx, sxy) = kept.iter().fold((0.0, 0.0), |(xx, xy), p| {
            (xx + (p.0 - mx) * (p.0 - mx), xy + (p.0 - mx) * (p.1 - my))
        });
        if sxx <= f64::EPSILON {
            return line;
        }
        let scale = sxy / sxx;
        let offset = my - scale * mx;
        let spread = (kept
            .iter()
            .map(|p| (p.1 - scale * p.0 - offset).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        line = Some((scale, offset));
        kept = points
            .iter()
            .copied()
            .filter(|p| (p.1 - scale * p.0 - offset).abs() <= TRIM_SIGMAS * spread.max(1e-9))
            .collect();
    }
    line
}

/// How the shorter stack maps onto the picture so far, for each color: the scale the exposures
/// give, refined on the pixels where both are well measured.
fn fit_scales(layered: &Layered, shorter: &LinearRgbF32, ratio: f64) -> [(f64, f64); COLORS] {
    let pixels = layered.fullness.len();
    let step = (pixels / FIT_SAMPLES).max(1);
    std::array::from_fn(|color| {
        let points: Vec<(f64, f64)> = (0..pixels)
            .step_by(step)
            .filter(|&pixel| {
                let own = &shorter.samples[pixel * COLORS..(pixel + 1) * COLORS];
                (FIT_FROM..=FIT_TO).contains(&layered.fullness[pixel])
                    && own.iter().all(|&v| v > 0.0)
            })
            .map(|pixel| {
                (
                    f64::from(shorter.samples[pixel * COLORS + color]),
                    f64::from(layered.image.samples[pixel * COLORS + color]),
                )
            })
            .collect();
        match fit_line(&points) {
            Some((scale, offset))
                if (ratio / MAX_SCALE_ERROR..=ratio * MAX_SCALE_ERROR).contains(&scale) =>
            {
                (scale, offset)
            }
            _ => (ratio, 0.0),
        }
    })
}

/// Puts a shorter exposure into the picture: where the values so far may be cut off, those of the
/// shorter stack (brought to the scale of the picture) take their place, in a soft transition.
/// `ratio` is how many times more light the picture's exposure collected.
pub fn add_shorter(layered: Layered, shorter: &LinearRgbF32, ratio: f64) -> Layered {
    let scales = fit_scales(&layered, shorter, ratio);
    let Layered {
        mut image,
        mut fullness,
    } = layered;
    for (pixel, full) in fullness.iter_mut().enumerate() {
        let own = &shorter.samples[pixel * COLORS..(pixel + 1) * COLORS];
        let has_data = own.iter().any(|&v| v > 0.0);
        let keep = if has_data { keep_share(*full) } else { 1.0 };
        if keep >= 1.0 {
            continue;
        }
        let target = &mut image.samples[pixel * COLORS..(pixel + 1) * COLORS];
        for ((value, &sample), &(scale, offset)) in target.iter_mut().zip(own).zip(&scales) {
            let brought = (scale * f64::from(sample) + offset) as f32;
            *value = (keep * *value + (1.0 - keep) * brought).max(0.0);
        }
        *full = keep * *full + (1.0 - keep) * largest_color(own);
    }
    Layered { image, fullness }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIDE: usize = 128;
    const RATIO: f64 = 8.0;

    /// The light of a field that brightens from 0.05 to 4.0 (in units of the full scale of the
    /// longer exposure) along the rows, a little different in every color.
    fn truth(pixel: usize, color: usize) -> f32 {
        let along = (pixel % SIDE) as f32 / (SIDE - 1) as f32;
        (0.05 + 3.95 * along) * (1.0 + 0.1 * color as f32)
    }

    fn noise(pixel: usize, color: usize) -> f32 {
        let n = (pixel * 31 + color * 17) % 97;
        (n as f32 / 96.0 - 0.5) * 0.004
    }

    fn picture(make: impl Fn(usize, usize) -> f32) -> LinearRgbF32 {
        let samples = (0..SIDE * SIDE * COLORS)
            .map(|index| make(index / COLORS, index % COLORS))
            .collect();
        LinearRgbF32 {
            width: SIDE,
            height: SIDE,
            samples,
        }
    }

    fn long() -> LinearRgbF32 {
        picture(|p, c| (truth(p, c) + noise(p, c)).clamp(0.0, 1.0))
    }

    fn short() -> LinearRgbF32 {
        picture(|p, c| truth(p, c) / RATIO as f32 + noise(p + 5, c) / 4.0)
    }

    fn merged(ratio: f64) -> LinearRgbF32 {
        add_shorter(Layered::start(long()), &short(), ratio).image
    }

    #[test]
    fn the_cut_off_part_is_recovered_from_the_shorter_stack() {
        let image = merged(RATIO);
        for pixel in 0..SIDE * SIDE {
            for color in 0..COLORS {
                let want = truth(pixel, color);
                let got = image.samples[pixel * COLORS + color];
                if want * 1.0 > 1.4 {
                    assert!((got - want).abs() < 0.03 * want, "{want} -> {got}");
                }
            }
        }
    }

    #[test]
    fn the_part_that_is_not_full_stays_as_the_longer_stack_has_it() {
        let image = merged(RATIO);
        let long = long();
        for pixel in 0..SIDE * SIDE {
            let own = &long.samples[pixel * COLORS..(pixel + 1) * COLORS];
            if largest_color(own) < FULL_FROM {
                for (color, value) in own.iter().enumerate() {
                    assert_eq!(image.samples[pixel * COLORS + color], *value);
                }
            }
        }
    }

    #[test]
    fn the_scale_is_found_even_when_the_exposures_are_a_little_off() {
        let image = merged(7.2);
        let want = truth(SIDE - 1, 0);
        let got = image.samples[(SIDE - 1) * COLORS];
        assert!((got - want).abs() < 0.03 * want, "{want} -> {got}");
    }

    #[test]
    fn where_the_shorter_stack_has_no_data_the_picture_is_kept() {
        let mut empty = short();
        empty.samples.fill(0.0);
        let image = add_shorter(Layered::start(long()), &empty, RATIO).image;
        assert_eq!(image.samples, long().samples);
    }

    #[test]
    fn a_third_exposure_takes_over_where_the_second_is_full_too() {
        let second = picture(|p, c| (truth(p, c) / 2.0).clamp(0.0, 1.0));
        let third = picture(|p, c| truth(p, c) / 16.0);
        let layered = add_shorter(Layered::start(long()), &second, 2.0);
        let image = add_shorter(layered, &third, 16.0).image;
        let want = truth(SIDE - 1, 2);
        let got = image.samples[(SIDE - 1) * COLORS + 2];
        assert!((got - want).abs() < 0.03 * want, "{want} -> {got}");
    }
}
