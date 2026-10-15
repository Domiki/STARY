use std::path::Path;

use image::{ExtendedColorType, codecs::jpeg::JpegEncoder};
use serde::Serialize;
use stary_core::{Histogram, PreviewImage, histogram::histogram_of_rgb8};

use super::{binary_packet::pack, command_error::CommandError};

const JPEG_QUALITY: u8 = 92;

#[derive(Debug, Serialize)]
struct ImageHeader {
    width: u32,
    height: u32,
    histogram: Histogram,
}

fn encode_jpeg(preview: &PreviewImage) -> Result<Vec<u8>, CommandError> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
        .encode(
            &preview.rgb8,
            preview.width,
            preview.height,
            ExtendedColorType::Rgb8,
        )
        .map_err(CommandError::internal)?;
    Ok(bytes)
}

pub fn image_packet(path: &Path, max_edge: u32) -> Result<Vec<u8>, CommandError> {
    let preview = stary_io::read_preview(path, max_edge)?;
    let header = ImageHeader {
        width: preview.width,
        height: preview.height,
        histogram: histogram_of_rgb8(&preview.rgb8),
    };
    pack(&header, &encode_jpeg(&preview)?)
}
