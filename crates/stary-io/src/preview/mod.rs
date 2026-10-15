mod color_image;
pub(crate) mod decode;
mod fits_preview;
mod image_preview;
mod raw_preview;

use std::path::Path;

use stary_core::{PreviewImage, SourceFormat};

use crate::{detect::detect_format, error::IoError};

pub fn read_preview(path: &Path, max_edge: u32) -> Result<PreviewImage, IoError> {
    match detect_format(path)? {
        SourceFormat::Raw => raw_preview::raw_preview(path, max_edge),
        SourceFormat::Fits => fits_preview::fits_preview(path, max_edge),
        SourceFormat::Jpeg | SourceFormat::Png | SourceFormat::Tiff => {
            image_preview::image_preview(path, max_edge)
        }
    }
}
