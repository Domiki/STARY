const SHADOW_CLIP_SIGMAS: f32 = -2.8;
const TARGET_BACKGROUND: f32 = 0.25;
const MAD_TO_SIGMA: f32 = 1.4826;
const MAX_SAMPLED: usize = 100_000;
const MIN_X: f32 = 1e-4;

#[derive(Debug, Clone, Copy)]
struct ChannelCurve {
    shadows: f32,
    midtones: f32,
}

fn midtones_transfer(midtones: f32, x: f32) -> f32 {
    ((midtones - 1.0) * x) / (((2.0 * midtones - 1.0) * x) - midtones)
}

fn median_of(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let middle = values.len() / 2;
    *values.select_nth_unstable_by(middle, f32::total_cmp).1
}

fn sampled_channel(samples: &[f32], channels: usize, channel: usize) -> Vec<f32> {
    let step = (samples.len() / channels / MAX_SAMPLED).max(1) * channels;
    samples
        .iter()
        .skip(channel)
        .step_by(step)
        .copied()
        .collect()
}

fn curve_for(values: Vec<f32>) -> ChannelCurve {
    let median = median_of(values.clone());
    let deviations = values.iter().map(|value| (value - median).abs()).collect();
    let sigma = MAD_TO_SIGMA * median_of(deviations);
    let shadows = (median + SHADOW_CLIP_SIGMAS * sigma).clamp(0.0, 0.99);
    let x = ((median - shadows) / (1.0 - shadows)).max(MIN_X);
    ChannelCurve {
        shadows,
        midtones: midtones_transfer(TARGET_BACKGROUND, x),
    }
}

fn apply_curve(curve: ChannelCurve, value: f32) -> f32 {
    let x = ((value - curve.shadows) / (1.0 - curve.shadows)).clamp(0.0, 1.0);
    midtones_transfer(curve.midtones, x).clamp(0.0, 1.0)
}

/// The curves that show a linear image: each channel's background brought to a quarter of the
/// range, the darkest noise cut off.
#[derive(Debug, Clone)]
pub struct Stretch {
    curves: Vec<ChannelCurve>,
}

impl Stretch {
    /// Fits the curves to interleaved samples of `channels` channels.
    pub fn fit(samples: &[f32], channels: usize) -> Self {
        Self {
            curves: (0..channels)
                .map(|channel| curve_for(sampled_channel(samples, channels, channel)))
                .collect(),
        }
    }

    /// The stretched value, between 0 and 1.
    pub fn apply(&self, channel: usize, value: f32) -> f32 {
        apply_curve(self.curves[channel], value)
    }
}

pub fn auto_stretch(samples: &[f32], channels: usize) -> Vec<u8> {
    let stretch = Stretch::fit(samples, channels);
    samples
        .iter()
        .enumerate()
        .map(|(index, &value)| (stretch.apply(index % channels, value) * 255.0).round() as u8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_lands_near_target() {
        let samples: Vec<f32> = (0..1001)
            .map(|i| 0.015 + 0.01 * (i as f32 / 1000.0))
            .collect();
        let stretched = auto_stretch(&samples, 1);
        let expected = (TARGET_BACKGROUND * 255.0).round() as i32;
        assert!((stretched[500] as i32 - expected).abs() <= 2);
    }

    #[test]
    fn bright_values_stay_bright() {
        let mut samples = vec![0.02_f32; 1000];
        samples[0] = 1.0;
        assert_eq!(auto_stretch(&samples, 1)[0], 255);
    }

    #[test]
    fn the_fitted_curves_stretch_the_background_to_a_quarter() {
        let samples: Vec<f32> = (0..1001)
            .map(|i| 0.015 + 0.01 * (i as f32 / 1000.0))
            .collect();
        let stretch = Stretch::fit(&samples, 1);
        let value = stretch.apply(0, samples[500]);
        assert!((value - TARGET_BACKGROUND).abs() < 0.01, "{value}");
        assert!((stretch.apply(0, 5.0) - 1.0).abs() < 1e-5);
        assert_eq!(stretch.apply(0, -5.0), 0.0);
    }

    #[test]
    fn keeps_channel_count() {
        let samples = vec![0.1_f32; 30];
        assert_eq!(auto_stretch(&samples, 3).len(), 30);
    }
}
