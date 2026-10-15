use std::{
    io::{BufRead, Seek},
    path::Path,
};

use image::{ColorType, DynamicImage, ImageDecoder, ImageReader};
use stary_core::{PreviewImage, stretch::auto_stretch};

use crate::error::IoError;

pub fn decode_oriented<R: BufRead + Seek>(
    reader: ImageReader<R>,
    path: &Path,
) -> Result<DynamicImage, IoError> {
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| IoError::decode(path, error))?;
    let orientation = decoder
        .orientation()
        .map_err(|error| IoError::decode(path, error))?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|error| IoError::decode(path, error))?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn is_float(image: &DynamicImage) -> bool {
    matches!(image.color(), ColorType::Rgb32F | ColorType::Rgba32F)
}

pub fn fit_within(image: DynamicImage, max_edge: u32) -> PreviewImage {
    let needs_resize = image.width().max(image.height()) > max_edge;
    let resized = if needs_resize {
        image.thumbnail(max_edge, max_edge)
    } else {
        image
    };
    if is_float(&resized) {
        // linear float data would look almost black, so it is stretched for display
        let floats = resized.to_rgb32f();
        let (width, height) = (floats.width(), floats.height());
        let rgb8 = auto_stretch(floats.as_raw(), 3);
        return PreviewImage {
            width,
            height,
            rgb8,
        };
    }
    let rgb = resized.to_rgb8();
    PreviewImage {
        width: rgb.width(),
        height: rgb.height(),
        rgb8: rgb.into_raw(),
    }
}
