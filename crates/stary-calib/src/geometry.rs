use stary_core::PixelLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorGeometry {
    pub width: usize,
    pub height: usize,
    pub layout: PixelLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    #[error("frame sizes differ")]
    SizeMismatch,
    #[error("color filter layouts differ")]
    LayoutMismatch,
    #[error("sample count does not match the frame size")]
    BadSampleCount,
}

impl SensorGeometry {
    pub const fn len(&self) -> usize {
        self.width * self.height * self.layout.channels()
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn check_matches(&self, other: &SensorGeometry) -> Result<(), GeometryError> {
        if self.width != other.width || self.height != other.height {
            Err(GeometryError::SizeMismatch)
        } else if self.layout != other.layout {
            Err(GeometryError::LayoutMismatch)
        } else {
            Ok(())
        }
    }

    pub fn check_samples(&self, samples: usize) -> Result<(), GeometryError> {
        if samples == self.len() {
            Ok(())
        } else {
            Err(GeometryError::BadSampleCount)
        }
    }

    /// The color class (0 red, 1 green, 2 blue) of the sample at `index`.
    pub fn color_of(&self, index: usize) -> usize {
        let channels = self.layout.channels();
        let (pixel, channel) = (index / channels, index % channels);
        self.layout
            .color_at(pixel / self.width, pixel % self.width, channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stary_core::{BayerPattern, CfaPattern};

    #[test]
    fn the_color_of_a_sample_follows_the_layout() {
        let bayer = SensorGeometry {
            width: 4,
            height: 2,
            layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        };
        assert_eq!(
            (0..8)
                .map(|index| bayer.color_of(index))
                .collect::<Vec<_>>(),
            vec![0, 1, 0, 1, 1, 2, 1, 2]
        );
        let rgb = SensorGeometry {
            width: 2,
            height: 1,
            layout: PixelLayout::Rgb,
        };
        assert_eq!(rgb.len(), 6);
        assert_eq!(
            (0..6).map(|index| rgb.color_of(index)).collect::<Vec<_>>(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }
}
