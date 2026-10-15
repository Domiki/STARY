use std::path::Path;

use stary_core::{Frame, PixelLayout};

use super::quantize::float_to_u16;
use crate::{
    error::IoError,
    fits::{read_normalized, stary_cfa},
};

const FULL_SCALE: u16 = u16::MAX;
const KEY_BLACK: &str = "STRYBLK";
const KEY_MAXIMUM: &str = "STRYMAX";

fn integer_scale(bitpix: i32) -> f32 {
    if bitpix == 8 { 257.0 } else { 1.0 }
}

fn quantize(samples: &[f32], bitpix: i32) -> Vec<u16> {
    if bitpix < 0 {
        return float_to_u16(samples);
    }
    let scale = integer_scale(bitpix);
    samples
        .iter()
        .map(|value| (value * scale).round().clamp(0.0, f32::from(FULL_SCALE)) as u16)
        .collect()
}

/// A FITS image as a frame. Files without a stated sensor layout are mono; the black and the
/// maximum come from STARY's keywords when the file was saved by STARY.
pub fn read_fits_frame(path: &Path) -> Result<Frame, IoError> {
    let fits = read_normalized(path)?;
    let layout = fits.layout.unwrap_or(PixelLayout::Mono);
    let number = |key: &str, default: u16| {
        fits.header.number(key).map_or(default, |value| {
            value.clamp(0.0, f64::from(FULL_SCALE)) as u16
        })
    };
    let known_to_stary = fits.header.text(stary_cfa::KEY_CFA).is_some();
    Ok(Frame {
        width: fits.geometry.width,
        height: fits.geometry.height,
        layout,
        black: if known_to_stary {
            number(KEY_BLACK, 0)
        } else {
            0
        },
        maximum: if known_to_stary {
            number(KEY_MAXIMUM, FULL_SCALE)
        } else {
            FULL_SCALE
        },
        samples: quantize(&fits.samples, fits.geometry.bitpix),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_values_fill_the_sixteen_bit_range() {
        assert_eq!(quantize(&[0.0, 255.0, 128.0], 8), vec![0, 65535, 32896]);
    }

    #[test]
    fn integer_counts_are_kept_as_they_are() {
        assert_eq!(
            quantize(&[10.0, 65535.0, -3.0, 70000.0], 16),
            vec![10, 65535, 0, 65535]
        );
    }

    #[test]
    fn float_data_is_spread_only_when_normalised() {
        assert_eq!(quantize(&[0.0, 1.0], -32), vec![0, 65535]);
        assert_eq!(quantize(&[0.0, 300.0], -32), vec![0, 300]);
    }
}
