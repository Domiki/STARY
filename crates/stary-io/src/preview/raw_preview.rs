use std::{io::Cursor, path::Path};

use image::{DynamicImage, ImageReader, RgbImage};
use rsraw::{BIT_DEPTH_8, RawImage, ThumbFormat, ThumbnailImage};
use stary_core::PreviewImage;

use super::decode::{decode_oriented, fit_within};
use crate::error::IoError;

const RGB_COLORS: u16 = 3;

fn largest_usable(thumbnails: Vec<ThumbnailImage>) -> Option<ThumbnailImage> {
    thumbnails
        .into_iter()
        .filter(|thumb| matches!(thumb.format, ThumbFormat::Jpeg | ThumbFormat::Bitmap))
        .max_by_key(|thumb| thumb.height)
}

fn preview_from_thumbnail(
    thumb: ThumbnailImage,
    path: &Path,
    max_edge: u32,
) -> Result<PreviewImage, IoError> {
    match thumb.format {
        ThumbFormat::Jpeg => {
            let reader = ImageReader::new(Cursor::new(thumb.data))
                .with_guessed_format()
                .map_err(IoError::read(path))?;
            decode_oriented(reader, path).map(|image| fit_within(image, max_edge))
        }
        _ if thumb.colors == RGB_COLORS => {
            RgbImage::from_raw(thumb.width, thumb.height, thumb.data)
                .map(|image| fit_within(DynamicImage::ImageRgb8(image), max_edge))
                .ok_or_else(|| IoError::decode(path, "embedded bitmap has unexpected size"))
        }
        _ => Err(IoError::unsupported(path, "embedded preview format")),
    }
}

fn preview_from_half_size_decode(
    raw: &mut RawImage,
    path: &Path,
    max_edge: u32,
) -> Result<PreviewImage, IoError> {
    raw.set_half_size(true);
    raw.unpack()
        .map_err(|error| IoError::decode(path, format!("{error:?}")))?;
    let processed = raw
        .process::<BIT_DEPTH_8>()
        .map_err(|error| IoError::decode(path, format!("{error:?}")))?;
    RgbImage::from_raw(processed.width(), processed.height(), processed.to_vec())
        .map(|image| fit_within(DynamicImage::ImageRgb8(image), max_edge))
        .ok_or_else(|| IoError::decode(path, "processed image has unexpected size"))
}

pub fn raw_preview(path: &Path, max_edge: u32) -> Result<PreviewImage, IoError> {
    let bytes = std::fs::read(path).map_err(IoError::read(path))?;
    let mut raw =
        RawImage::open(&bytes).map_err(|error| IoError::decode(path, format!("{error:?}")))?;
    let thumbnails = raw.extract_thumbs().unwrap_or_default();
    match largest_usable(thumbnails) {
        Some(thumb) => preview_from_thumbnail(thumb, path, max_edge)
            .or_else(|_| preview_from_half_size_decode(&mut raw, path, max_edge)),
        None => preview_from_half_size_decode(&mut raw, path, max_edge),
    }
}
