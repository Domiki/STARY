const STRIDE: usize = 5;
const MAD_TO_SIGMA: f32 = 1.4826;
const CLIP: f32 = 3.0;
const MIN_SCALE: f32 = 1e-3;

/// The sky level and the pixel noise of one channel of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyLevel {
    pub location: f32,
    pub scale: f32,
}

fn median(values: &mut [f32]) -> f32 {
    let middle = values.len() / 2;
    let (_, value, _) = values.select_nth_unstable_by(middle, f32::total_cmp);
    *value
}

/// Estimates the sky level and noise from a sparse sample, leaving out the bright pixels of
/// stars with one round of clipping.
pub fn estimate_sky(samples: &[u16], width: usize) -> SkyLevel {
    let mut values: Vec<f32> = samples
        .chunks(width)
        .step_by(STRIDE)
        .flat_map(|line| line.iter().step_by(STRIDE).map(|value| f32::from(*value)))
        .collect();
    if values.is_empty() {
        return SkyLevel {
            location: 0.0,
            scale: 1.0,
        };
    }
    let center = median(&mut values.clone());
    let mut deviations: Vec<f32> = values.iter().map(|value| (value - center).abs()).collect();
    let spread = MAD_TO_SIGMA * median(&mut deviations);
    values.retain(|value| *value <= center + CLIP * spread);
    let location = median(&mut values);
    let mut deviations: Vec<f32> = values
        .iter()
        .map(|value| (value - location).abs())
        .collect();
    SkyLevel {
        location,
        scale: (MAD_TO_SIGMA * median(&mut deviations)).max(MIN_SCALE),
    }
}

/// Additive and scaling normalisation: `value * gain + offset` gives the frame the sky level and
/// noise of the reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matching {
    pub gain: f32,
    pub offset: f32,
}

impl Matching {
    pub const NONE: Self = Self {
        gain: 1.0,
        offset: 0.0,
    };

    pub fn between(frame: SkyLevel, reference: SkyLevel) -> Self {
        let gain = reference.scale / frame.scale;
        Self {
            gain,
            offset: reference.location - frame.location * gain,
        }
    }

    pub fn apply(&self, value: f32) -> f32 {
        value * self.gain + self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky(width: usize, height: usize, level: f32, noise: f32) -> Vec<u16> {
        (0..width * height)
            .map(|index| {
                let wobble = ((index * 7919) % 201) as f32 / 100.0 - 1.0;
                (level + noise * wobble * 1.7) as u16
            })
            .collect()
    }

    #[test]
    fn the_sky_level_ignores_bright_stars() {
        let mut data = sky(400, 300, 2000.0, 20.0);
        for index in (0..data.len()).step_by(97) {
            data[index] = 40000;
        }
        let sky = estimate_sky(&data, 400);
        assert!((sky.location - 2000.0).abs() < 8.0, "{sky:?}");
        assert!(sky.scale > 5.0 && sky.scale < 60.0, "{sky:?}");
    }

    #[test]
    fn matching_moves_a_frame_onto_the_reference_level_and_noise() {
        let frame = SkyLevel {
            location: 1500.0,
            scale: 30.0,
        };
        let reference = SkyLevel {
            location: 2000.0,
            scale: 20.0,
        };
        let matching = Matching::between(frame, reference);
        assert!((matching.apply(1500.0) - 2000.0).abs() < 1e-3);
        assert!((matching.apply(1530.0) - 2020.0).abs() < 1e-3);
    }

    #[test]
    fn matching_a_frame_with_itself_changes_nothing() {
        let level = SkyLevel {
            location: 800.0,
            scale: 12.0,
        };
        let matching = Matching::between(level, level);
        assert!((matching.gain - 1.0).abs() < 1e-6 && matching.offset.abs() < 1e-3);
    }

    #[test]
    fn a_blank_frame_does_not_divide_by_zero() {
        let blank = estimate_sky(&[0u16; 100], 10);
        assert!(blank.scale > 0.0);
        assert!(Matching::between(blank, blank).gain.is_finite());
    }
}
