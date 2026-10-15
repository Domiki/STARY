/// How values are encoded: linear light, or a curve that gives the dark tones more room.
#[derive(Debug, Clone, PartialEq)]
pub enum Curve {
    Linear,
    Srgb,
    Gamma(f64),
    /// Linear values at evenly spaced encoded values from 0 to 1, as ICC profiles keep them.
    Table(Vec<f32>),
}

impl Curve {
    pub fn encode(&self, linear: f64) -> f64 {
        match self {
            Self::Linear => linear,
            Self::Srgb => {
                if linear <= 0.003_130_8 {
                    12.92 * linear
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                }
            }
            Self::Gamma(gamma) => linear.max(0.0).powf(1.0 / gamma),
            Self::Table(table) => invert_table(table, linear),
        }
    }

    pub fn decode(&self, encoded: f64) -> f64 {
        match self {
            Self::Linear => encoded,
            Self::Srgb => {
                if encoded <= 0.040_45 {
                    encoded / 12.92
                } else {
                    ((encoded + 0.055) / 1.055).powf(2.4)
                }
            }
            Self::Gamma(gamma) => encoded.max(0.0).powf(*gamma),
            Self::Table(table) => sample_table(table, encoded),
        }
    }
}

fn sample_table(table: &[f32], at: f64) -> f64 {
    let last = table.len() - 1;
    let position = at.clamp(0.0, 1.0) * last as f64;
    let (low, share) = (position.floor() as usize, position.fract());
    let high = (low + 1).min(last);
    f64::from(table[low]) * (1.0 - share) + f64::from(table[high]) * share
}

fn invert_table(table: &[f32], value: f64) -> f64 {
    let (mut low, mut high) = (0usize, table.len() - 1);
    let value = value.clamp(0.0, 1.0);
    while high - low > 1 {
        let middle = (low + high) / 2;
        if f64::from(table[middle]) <= value {
            low = middle;
        } else {
            high = middle;
        }
    }
    let (a, b) = (f64::from(table[low]), f64::from(table[high]));
    let share = if b > a { (value - a) / (b - a) } else { 0.0 };
    (low as f64 + share) / (table.len() - 1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_curve_comes_back_to_where_it_started() {
        let curves = [
            Curve::Linear,
            Curve::Srgb,
            Curve::Gamma(2.2),
            Curve::Table((0..=256).map(|i| (i as f32 / 256.0).powf(0.45)).collect()),
        ];
        for curve in curves {
            for step in 0..=20 {
                let linear = f64::from(step) / 20.0;
                let back = curve.decode(curve.encode(linear));
                assert!(
                    (back - linear).abs() < 1e-3,
                    "{curve:?}: {linear} -> {back}"
                );
            }
        }
    }

    #[test]
    fn the_srgb_curve_lifts_the_shadows() {
        assert!((Curve::Srgb.encode(0.2140) - 0.5).abs() < 1e-3);
        assert_eq!(Curve::Srgb.encode(0.0), 0.0);
        assert!((Curve::Srgb.encode(1.0) - 1.0).abs() < 1e-9);
    }
}
