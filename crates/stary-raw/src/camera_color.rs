use rsraw::RawImage;

use crate::error::RawError;

pub use stary_core::CameraToSrgb;

const ROW_SUM_TOLERANCE: f32 = 0.02;

fn valid(matrix: &CameraToSrgb) -> bool {
    matrix.iter().all(|row| {
        row.iter().all(|value| value.is_finite())
            && (row.iter().sum::<f32>() - 1.0).abs() < ROW_SUM_TOLERANCE
    })
}

/// The camera's color matrix, from the file itself. `None` when the file does not carry one the
/// decoder knows.
pub fn camera_to_srgb(bytes: &[u8]) -> Result<Option<CameraToSrgb>, RawError> {
    let raw = RawImage::open(bytes).map_err(|error| RawError::Open(format!("{error:?}")))?;
    let found = raw.as_ref().color.rgb_cam;
    let matrix: CameraToSrgb =
        std::array::from_fn(|row| std::array::from_fn(|column| found[row][column]));
    Ok(valid(&matrix).then_some(matrix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matrix_whose_rows_do_not_keep_white_is_not_taken() {
        assert!(valid(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]));
        assert!(!valid(&[[0.0; 3]; 3]));
        assert!(!valid(&[
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, f32::NAN]
        ]));
    }
}
