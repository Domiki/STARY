use std::path::Path;

use image::{ColorType, ImageReader};
use stary_core::{Frame, PixelLayout};

use super::{quantize::float_to_u16, srgb::linearize};
use crate::{error::IoError, preview::decode::decode_oriented};

const FULL_SCALE: u16 = u16::MAX;

fn frame_of(width: usize, height: usize, layout: PixelLayout, samples: Vec<u16>) -> Frame {
    Frame {
        width,
        height,
        layout,
        black: 0,
        maximum: FULL_SCALE,
        samples,
    }
}

/// JPEG, PNG and TIFF. Integer images are taken to be sRGB encoded and made linear; floating
/// point TIFFs are taken to be linear already.
pub fn read_image_frame(path: &Path) -> Result<Frame, IoError> {
    let reader = ImageReader::open(path)
        .map_err(IoError::read(path))?
        .with_guessed_format()
        .map_err(IoError::read(path))?;
    let image = decode_oriented(reader, path)?;
    let (width, height) = (image.width() as usize, image.height() as usize);
    match image.color() {
        ColorType::Rgb32F | ColorType::Rgba32F => {
            let samples = float_to_u16(&image.into_rgb32f().into_raw());
            Ok(frame_of(width, height, PixelLayout::Rgb, samples))
        }
        ColorType::L8 | ColorType::La8 | ColorType::L16 | ColorType::La16 => {
            let mut samples = image.into_luma16().into_raw();
            linearize(&mut samples);
            Ok(frame_of(width, height, PixelLayout::Mono, samples))
        }
        _ => {
            let mut samples = image.into_rgb16().into_raw();
            linearize(&mut samples);
            Ok(frame_of(width, height, PixelLayout::Rgb, samples))
        }
    }
}
