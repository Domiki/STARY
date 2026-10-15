use std::path::Path;

use image::ImageReader;
use stary_mask::Mask;

use crate::{error::IoError, preview::decode::decode_oriented};

const FULL_SCALE: f32 = u16::MAX as f32;

/// A gray image (PNG, TIFF) as a mask: white is sky, black is ground. Color images are read by
/// their brightness.
pub fn read_mask(path: &Path) -> Result<Mask, IoError> {
    let reader = ImageReader::open(path)
        .map_err(IoError::read(path))?
        .with_guessed_format()
        .map_err(IoError::read(path))?;
    let image = decode_oriented(reader, path)?.into_luma16();
    let (width, height) = (image.width() as usize, image.height() as usize);
    let values = image
        .into_raw()
        .into_iter()
        .map(|value| f32::from(value) / FULL_SCALE)
        .collect();
    Mask::new(width, height, values).map_err(|error| IoError::decode(path, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gray_png_becomes_a_mask() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mask.png");
        image::GrayImage::from_raw(3, 2, vec![255, 0, 255, 0, 255, 0])
            .unwrap()
            .save(&path)
            .unwrap();
        let mask = read_mask(&path).unwrap();
        assert_eq!((mask.width(), mask.height()), (3, 2));
        assert_eq!(mask.values(), &[1.0, 0.0, 1.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(read_mask(Path::new("/no/such/mask.png")).is_err());
    }
}
