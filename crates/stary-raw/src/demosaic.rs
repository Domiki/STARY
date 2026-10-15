use rsraw::{BIT_DEPTH_16, RawImage};
use stary_core::LinearRgb16;

use crate::{
    error::RawError,
    sensor_buffer::{ActiveArea, active_area, sensor_buffer_mut},
};

/// The color space of a demosaiced RAW. Converting to sRGB mixes the channels with a 3x3 matrix, which
/// looks right on screen but adds noise to the weaker channels; the camera's own RGB keeps the
/// data as the sensor recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputColor {
    Camera,
    Srgb,
}

impl OutputColor {
    const fn libraw_value(self) -> i32 {
        match self {
            Self::Camera => 0,
            Self::Srgb => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemosaicQuality {
    Fast,
    High,
}

impl DemosaicQuality {
    const fn libraw_quality(self) -> i32 {
        match self {
            Self::Fast => 0,
            Self::High => 3,
        }
    }
}

pub fn paste_active_area(
    buffer: &mut [u16],
    pitch: usize,
    rows: usize,
    area: &ActiveArea,
    samples: &[u16],
) -> Result<(), RawError> {
    let fits = area.left + area.width <= pitch && area.top + area.height <= rows;
    if !fits || samples.len() != area.width * area.height || buffer.len() < pitch * rows {
        return Err(RawError::BadGeometry);
    }
    for (row, line) in samples.chunks_exact(area.width).enumerate() {
        let start = (area.top + row) * pitch + area.left;
        buffer[start..start + area.width].copy_from_slice(line);
    }
    Ok(())
}

fn configure_linear_output(raw: &mut RawImage, quality: DemosaicQuality, color: OutputColor) {
    let params = &mut raw.as_mut().params;
    params.user_qual = quality.libraw_quality();
    params.output_color = color.libraw_value();
    params.no_auto_bright = 1;
    params.gamm[0] = 1.0;
    params.gamm[1] = 1.0;
    params.use_camera_wb = 1;
    params.user_flip = 0;
}

/// Replaces the sensor samples of `bytes` (a RAW file of the same camera and size) with `cfa` and
/// demosaics the result once. This is how a stack built on raw sensor data becomes a color image.
pub fn demosaic_cfa(
    bytes: &[u8],
    cfa: &[u16],
    quality: DemosaicQuality,
    color: OutputColor,
) -> Result<LinearRgb16, RawError> {
    let mut raw = RawImage::open(bytes).map_err(|error| RawError::Open(format!("{error:?}")))?;
    raw.unpack()
        .map_err(|error| RawError::Unpack(format!("{error:?}")))?;
    let area = active_area(&raw);
    {
        let buffer = sensor_buffer_mut(&mut raw)?;
        paste_active_area(buffer.samples, buffer.pitch, buffer.rows, &area, cfa)?;
    }
    configure_linear_output(&mut raw, quality, color);
    let image = raw
        .process::<BIT_DEPTH_16>()
        .map_err(|error| RawError::Unpack(format!("{error:?}")))?;
    let (width, height) = (image.width() as usize, image.height() as usize);
    if image.colors() != 3 || image.bits() != 16 || image.len() != width * height * 3 {
        return Err(RawError::BadGeometry);
    }
    Ok(LinearRgb16 {
        width,
        height,
        samples: image.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(left: usize, top: usize, width: usize, height: usize) -> ActiveArea {
        ActiveArea {
            left,
            top,
            width,
            height,
        }
    }

    #[test]
    fn pastes_into_the_visible_area_and_leaves_the_margins_alone() {
        let mut buffer = vec![0u16; 12];
        paste_active_area(&mut buffer, 4, 3, &area(1, 1, 2, 2), &[1, 2, 3, 4]).unwrap();
        assert_eq!(buffer, vec![0, 0, 0, 0, 0, 1, 2, 0, 0, 3, 4, 0]);
    }

    #[test]
    fn pasting_is_the_inverse_of_cropping() {
        let original: Vec<u16> = (0..12).collect();
        let a = area(1, 1, 2, 2);
        let cropped = crate::crop_active_area(&original, 4, 3, &a).unwrap();
        let mut copy = original.clone();
        paste_active_area(&mut copy, 4, 3, &a, &cropped).unwrap();
        assert_eq!(copy, original);
    }

    #[test]
    fn rejects_samples_of_the_wrong_size_or_an_area_outside_the_buffer() {
        let mut buffer = vec![0u16; 12];
        assert!(paste_active_area(&mut buffer, 4, 3, &area(0, 0, 2, 2), &[1, 2, 3]).is_err());
        assert!(paste_active_area(&mut buffer, 4, 3, &area(3, 0, 2, 1), &[1, 2]).is_err());
    }
}
