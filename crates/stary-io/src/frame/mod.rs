mod fits_frame;
mod image_frame;
mod quantize;
mod srgb;

use std::path::Path;

use stary_core::{Frame, SourceFormat};

use crate::{detect::detect_format, error::IoError};

pub use fits_frame::read_fits_frame;
pub use image_frame::read_image_frame;

fn read_raw_frame(path: &Path) -> Result<Frame, IoError> {
    let bytes = std::fs::read(path).map_err(IoError::read(path))?;
    stary_raw::decode_cfa(&bytes).map_err(|error| IoError::decode(path, error))
}

/// Decodes any supported input into the frame the pipeline works with.
pub fn decode_frame(path: &Path) -> Result<Frame, IoError> {
    match detect_format(path)? {
        SourceFormat::Raw => read_raw_frame(path),
        SourceFormat::Fits => read_fits_frame(path),
        SourceFormat::Jpeg | SourceFormat::Png | SourceFormat::Tiff => read_image_frame(path),
    }
}
