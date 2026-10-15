use crate::{
    curve::Curve,
    matrix::{Matrix, inverse, multiply},
    profile::Profile,
};

/// Turns values of one profile into linear values of another.
#[derive(Debug, Clone)]
pub struct Conversion {
    matrix: [[f32; 3]; 3],
    decode: Curve,
}

impl Conversion {
    /// `None` when the destination has primaries that do not span colors (a profile that cannot be
    /// inverted).
    pub fn between(source: &Profile, destination: &Profile) -> Option<Self> {
        let matrix: Matrix = multiply(&inverse(&destination.to_xyz)?, &source.to_xyz);
        Some(Self {
            matrix: std::array::from_fn(|row| {
                std::array::from_fn(|column| matrix[row][column] as f32)
            }),
            decode: source.curve.clone(),
        })
    }

    pub fn is_decoding(&self) -> bool {
        self.decode != Curve::Linear
    }

    /// The linear color in the destination, which may lie outside 0 to 1 where the destination
    /// cannot show the color.
    pub fn linear(&self, rgb: [f32; 3]) -> [f32; 3] {
        let rgb = if self.is_decoding() {
            rgb.map(|value| self.decode.decode(f64::from(value)) as f32)
        } else {
            rgb
        };
        std::array::from_fn(|row| {
            self.matrix[row][0] * rgb[0]
                + self.matrix[row][1] * rgb[1]
                + self.matrix[row][2] * rgb[2]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::Space;

    #[test]
    fn the_same_space_changes_nothing() {
        let srgb = Profile::of_space(Space::Srgb, true);
        let conversion = Conversion::between(&srgb, &srgb).unwrap();
        let out = conversion.linear([0.2, 0.5, 0.9]);
        for (a, b) in out.iter().zip([0.2, 0.5, 0.9]) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn grays_stay_gray_in_every_space() {
        let source = Profile::of_space(Space::Srgb, true);
        for space in [Space::DisplayP3, Space::AdobeRgb] {
            let conversion = Conversion::between(&source, &Profile::of_space(space, true)).unwrap();
            let out = conversion.linear([0.3, 0.3, 0.3]);
            assert!(
                (out[0] - out[1]).abs() < 1e-3 && (out[1] - out[2]).abs() < 1e-3,
                "{out:?}"
            );
            assert!((out[0] - 0.3).abs() < 1e-3);
        }
    }

    #[test]
    fn a_saturated_srgb_color_is_less_saturated_in_a_wider_space() {
        let source = Profile::of_space(Space::Srgb, true);
        let conversion =
            Conversion::between(&source, &Profile::of_space(Space::DisplayP3, true)).unwrap();
        let out = conversion.linear([0.0, 1.0, 0.0]);
        assert!(out[1] < 1.0 && out[0] > 0.0);
    }

    #[test]
    fn a_wider_color_goes_out_of_range_in_a_narrower_space() {
        let source = Profile::of_space(Space::DisplayP3, true);
        let conversion =
            Conversion::between(&source, &Profile::of_space(Space::Srgb, true)).unwrap();
        let out = conversion.linear([0.0, 1.0, 0.0]);
        assert!(out[0] < -0.1);
    }

    #[test]
    fn an_encoded_source_is_decoded_first() {
        let encoded = Profile::of_space(Space::Srgb, false);
        let linear = Profile::of_space(Space::Srgb, true);
        let out = Conversion::between(&encoded, &linear)
            .unwrap()
            .linear([0.5, 0.5, 0.5]);
        assert!((out[0] - 0.214).abs() < 2e-3);
    }
}
