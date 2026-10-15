use std::path::Path;

use image::ImageReader;
use stary_core::PreviewImage;

use super::decode::{decode_oriented, fit_within};
use crate::error::IoError;

pub fn image_preview(path: &Path, max_edge: u32) -> Result<PreviewImage, IoError> {
    let reader = ImageReader::open(path)
        .map_err(IoError::read(path))?
        .with_guessed_format()
        .map_err(IoError::read(path))?;
    decode_oriented(reader, path).map(|image| fit_within(image, max_edge))
}
