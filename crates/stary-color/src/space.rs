use crate::{
    curve::Curve,
    matrix::{Matrix, Vector, apply, inverse, multiply},
};

/// The white of the profile connection space of ICC profiles.
pub const D50: Vector = [0.9642, 1.0, 0.8249];
/// The D65 white of the sRGB standard.
const D65: Vector = [0.9505, 1.0, 1.0890];

/// An RGB working space the result can be saved in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Srgb,
    DisplayP3,
    AdobeRgb,
}

struct Primaries {
    red: [f64; 2],
    green: [f64; 2],
    blue: [f64; 2],
}

fn xyz_of(xy: [f64; 2]) -> Vector {
    [xy[0] / xy[1], 1.0, (1.0 - xy[0] - xy[1]) / xy[1]]
}

/// The matrix from linear RGB with these primaries and the white `white` (XYZ) to XYZ.
fn rgb_to_xyz(primaries: &Primaries, white: &Vector) -> Matrix {
    let (r, g, b) = (
        xyz_of(primaries.red),
        xyz_of(primaries.green),
        xyz_of(primaries.blue),
    );
    let columns: Matrix = std::array::from_fn(|row| [r[row], g[row], b[row]]);
    let scale = apply(
        &inverse(&columns).expect("primaries are not on a line"),
        white,
    );
    std::array::from_fn(|row| std::array::from_fn(|column| columns[row][column] * scale[column]))
}

/// Moves colors seen under a D65 white to a D50 white (Bradford).
pub fn d65_to_d50() -> Matrix {
    const BRADFORD: Matrix = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    let source = apply(&BRADFORD, &D65);
    let target = apply(&BRADFORD, &D50);
    let scale: Matrix = [
        [target[0] / source[0], 0.0, 0.0],
        [0.0, target[1] / source[1], 0.0],
        [0.0, 0.0, target[2] / source[2]],
    ];
    multiply(
        &inverse(&BRADFORD).expect("Bradford is invertible"),
        &multiply(&scale, &BRADFORD),
    )
}

impl Space {
    fn primaries(self) -> Primaries {
        match self {
            Self::Srgb => Primaries {
                red: [0.64, 0.33],
                green: [0.30, 0.60],
                blue: [0.15, 0.06],
            },
            Self::DisplayP3 => Primaries {
                red: [0.680, 0.320],
                green: [0.265, 0.690],
                blue: [0.150, 0.060],
            },
            Self::AdobeRgb => Primaries {
                red: [0.64, 0.33],
                green: [0.21, 0.71],
                blue: [0.15, 0.06],
            },
        }
    }

    /// Linear RGB of this space to XYZ with a D50 white (the columns are the primaries as an ICC
    /// profile lists them).
    pub fn to_xyz_d50(self) -> Matrix {
        multiply(&d65_to_d50(), &rgb_to_xyz(&self.primaries(), &D65))
    }

    /// How the values of this space are encoded for display.
    pub const fn curve(self) -> Curve {
        match self {
            Self::Srgb | Self::DisplayP3 => Curve::Srgb,
            Self::AdobeRgb => Curve::Gamma(563.0 / 256.0),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Srgb => "sRGB",
            Self::DisplayP3 => "Display P3",
            Self::AdobeRgb => "Adobe RGB",
        }
    }
}

/// Linear sRGB values of a camera's color (after white balance) as XYZ with a D50 white, from the
/// matrix that turns the camera's RGB into linear sRGB.
pub fn camera_to_xyz_d50(camera_to_srgb: &[[f32; 3]; 3]) -> Matrix {
    let to_srgb: Matrix = std::array::from_fn(|row| {
        std::array::from_fn(|column| f64::from(camera_to_srgb[row][column]))
    });
    multiply(&Space::Srgb.to_xyz_d50(), &to_srgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Matrix, b: &Matrix, tolerance: f64) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (a[row][column] - b[row][column]).abs() < tolerance,
                    "{a:?} vs {b:?}"
                );
            }
        }
    }

    #[test]
    fn srgb_has_the_primaries_of_the_standard_profile() {
        // the colorants of the sRGB profile that ships with every system (D50, Bradford); its blue is
        // 3e-4 off the exact white, which ours keeps
        let known = [
            [0.4360747, 0.3850649, 0.1430804],
            [0.2225045, 0.7168786, 0.0606169],
            [0.0139322, 0.0971045, 0.7141733],
        ];
        close(&Space::Srgb.to_xyz_d50(), &known, 3e-4);
    }

    #[test]
    fn white_stays_white_in_every_space() {
        for space in [Space::Srgb, Space::DisplayP3, Space::AdobeRgb] {
            let white = apply(&space.to_xyz_d50(), &[1.0, 1.0, 1.0]);
            for (value, expected) in white.iter().zip(D50) {
                assert!((value - expected).abs() < 2e-4, "{space:?}: {white:?}");
            }
        }
    }

    #[test]
    fn display_p3_is_wider_than_srgb() {
        let green = apply(&Space::DisplayP3.to_xyz_d50(), &[0.0, 1.0, 0.0]);
        let back = apply(&inverse(&Space::Srgb.to_xyz_d50()).unwrap(), &green);
        assert!(back[0] < -0.1, "{back:?}");
    }

    #[test]
    fn a_camera_whose_matrix_is_the_identity_is_srgb() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        close(
            &camera_to_xyz_d50(&identity),
            &Space::Srgb.to_xyz_d50(),
            1e-9,
        );
    }
}
