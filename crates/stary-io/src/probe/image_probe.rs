use std::path::Path;

use image::{ImageDecoder, ImageReader};
use stary_core::{PixelLayout, ProbeDetails};

use super::exif_fields::read_exif_fields;
use crate::error::IoError;

pub fn probe_image(path: &Path) -> Result<ProbeDetails, IoError> {
    let decoder = ImageReader::open(path)
        .map_err(IoError::read(path))?
        .with_guessed_format()
        .map_err(IoError::read(path))?
        .into_decoder()
        .map_err(|error| IoError::decode(path, error))?;
    let (width, height) = decoder.dimensions();
    let color = decoder.color_type();
    let exif = read_exif_fields(path);
    Ok(ProbeDetails {
        width,
        height,
        layout: if color.has_color() {
            PixelLayout::Rgb
        } else {
            PixelLayout::Mono
        },
        bit_depth: u8::try_from(color.bits_per_pixel() / u16::from(color.channel_count())).ok(),
        camera: exif.camera,
        exposure_s: exif.exposure_s,
        iso: exif.iso,
        captured_at: exif.captured_at,
        stary_kind: None,
        stack_count: None,
        calibrated: false,
    })
}
