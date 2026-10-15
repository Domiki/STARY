use crate::meta::PixelLayout;

/// One decoded input frame. `samples` hold `width * height * layout.channels()` values, 16-bit
/// full scale; the samples of an RGB frame are interleaved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub layout: PixelLayout,
    pub black: u16,
    pub maximum: u16,
    pub samples: Vec<u16>,
}

impl Frame {
    pub const fn expected_len(&self) -> usize {
        self.width * self.height * self.layout.channels()
    }
}

/// A demosaiced color image: interleaved R, G, B, 16-bit, linear (no gamma).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearRgb16 {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<u16>,
}

/// A demosaiced or stacked color image: interleaved R, G, B, linear, 1.0 being the full scale of
/// a 16-bit sample.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearRgbF32 {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<f32>,
}
