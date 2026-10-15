use std::{fs::File, io::BufReader, path::Path};

use stary_core::{CfaPattern, PixelLayout, PreviewImage};

use super::color_image::ColorImage;
use crate::{
    error::IoError,
    fits::{
        Geometry, RowOrder, geometry_of, layout_of, read_header, read_physical_samples,
        row_order_of,
    },
};

fn full_scale(geometry: &Geometry, samples: &[f32]) -> f32 {
    let max_seen = samples.iter().copied().fold(f32::MIN, f32::max);
    match geometry.bitpix {
        8 => 255.0,
        16 => 65535.0,
        _ if max_seen <= 1.0 => 1.0,
        _ => max_seen,
    }
}

fn normalized(samples: Vec<f32>, geometry: &Geometry) -> Vec<f32> {
    let scale = full_scale(geometry, &samples);
    samples
        .into_iter()
        .map(|value| (value / scale).clamp(0.0, 1.0))
        .collect()
}

fn color_image_of(samples: &[f32], geometry: &Geometry, layout: PixelLayout) -> ColorImage {
    let (width, height) = (geometry.width, geometry.height);
    match layout {
        PixelLayout::Cfa(CfaPattern::Bayer(pattern)) => {
            ColorImage::from_bayer(width, height, samples, pattern)
        }
        PixelLayout::Rgb if geometry.planes >= 3 => {
            ColorImage::from_planes(width, height, &samples[..width * height * 3])
        }
        _ => ColorImage::mono(width, height, samples[..width * height].to_vec()),
    }
}

pub fn fits_preview(path: &Path, max_edge: u32) -> Result<PreviewImage, IoError> {
    let file = File::open(path).map_err(IoError::read(path))?;
    let header = read_header(path, &mut BufReader::new(file))?;
    let geometry = geometry_of(path, &header)?;
    let layout = layout_of(&header, geometry.planes);
    let samples = normalized(read_physical_samples(path, &header, &geometry)?, &geometry);
    let image = color_image_of(&samples, &geometry, layout).box_downscale(max_edge as usize);
    let upright = match row_order_of(&header) {
        RowOrder::BottomUp => image.flip_vertical(),
        RowOrder::TopDown => image,
    };
    Ok(upright.into_stretched_preview())
}
