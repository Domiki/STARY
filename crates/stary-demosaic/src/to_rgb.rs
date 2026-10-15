use stary_core::{COLORS, Frame, LinearRgb16, PixelLayout};

use crate::{bayer::demosaic_bayer, generic::demosaic_generic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DemosaicError {
    #[error("the frame has {actual} samples, expected {expected}")]
    BadSampleCount { expected: usize, actual: usize },
}

const FULL_SCALE: u32 = u16::MAX as u32;

/// Removes the black level and spreads the rest over the 16-bit range, as the RAW path does.
fn normalized(frame: &Frame) -> Vec<u16> {
    let black = u32::from(frame.black);
    let span = u32::from(frame.maximum).saturating_sub(black).max(1);
    if black == 0 && u32::from(frame.maximum) == FULL_SCALE {
        return frame.samples.clone();
    }
    frame
        .samples
        .iter()
        .map(|value| {
            let signal = u32::from(*value).saturating_sub(black);
            ((signal * FULL_SCALE + span / 2) / span).min(FULL_SCALE) as u16
        })
        .collect()
}

/// Makes a color image of any decoded frame without help from a RAW file: color data is passed
/// through, mono data becomes gray and sensor data is demosaiced.
pub fn to_rgb(frame: &Frame) -> Result<LinearRgb16, DemosaicError> {
    let expected = frame.expected_len();
    if frame.samples.len() != expected {
        return Err(DemosaicError::BadSampleCount {
            expected,
            actual: frame.samples.len(),
        });
    }
    let samples = normalized(frame);
    let (width, height) = (frame.width, frame.height);
    let rgb = match frame.layout {
        PixelLayout::Rgb => samples,
        PixelLayout::Mono => samples.iter().flat_map(|value| [*value; COLORS]).collect(),
        PixelLayout::Cfa(stary_core::CfaPattern::Bayer(pattern)) => {
            demosaic_bayer(&samples, width, height, pattern)
        }
        PixelLayout::Cfa(pattern) => demosaic_generic(&samples, width, height, &pattern),
    };
    Ok(LinearRgb16 {
        width,
        height,
        samples: rgb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use stary_core::{BayerPattern, CfaPattern};

    fn frame(layout: PixelLayout, black: u16, maximum: u16, samples: Vec<u16>) -> Frame {
        Frame {
            width: samples.len() / layout.channels(),
            height: 1,
            layout,
            black,
            maximum,
            samples,
        }
    }

    #[test]
    fn mono_becomes_gray() {
        let image = to_rgb(&frame(PixelLayout::Mono, 0, 65535, vec![5, 9])).unwrap();
        assert_eq!(image.samples, vec![5, 5, 5, 9, 9, 9]);
    }

    #[test]
    fn color_data_is_passed_through() {
        let image = to_rgb(&frame(PixelLayout::Rgb, 0, 65535, vec![1, 2, 3])).unwrap();
        assert_eq!(image.samples, vec![1, 2, 3]);
    }

    #[test]
    fn the_black_level_is_removed_and_the_range_is_spread() {
        let image = to_rgb(&frame(
            PixelLayout::Mono,
            100,
            1100,
            vec![50, 100, 600, 1100],
        ))
        .unwrap();
        assert_eq!(
            image.samples.iter().step_by(3).copied().collect::<Vec<_>>(),
            vec![0, 0, 32768, 65535]
        );
    }

    #[test]
    fn sensor_data_comes_back_with_three_channels() {
        let layout = PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb));
        let mut input = frame(layout, 0, 65535, vec![7; 16]);
        input.width = 4;
        input.height = 4;
        let image = to_rgb(&input).unwrap();
        assert_eq!(image.samples.len(), 48);
        assert!(image.samples.iter().all(|value| *value == 7));
    }

    #[test]
    fn a_wrong_sample_count_is_an_error() {
        let mut input = frame(PixelLayout::Mono, 0, 65535, vec![1, 2, 3]);
        input.width = 5;
        assert!(to_rgb(&input).is_err());
    }
}
