/// A detected star. Coordinates are in pixels with the centre of the top-left pixel at (0, 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    pub x: f64,
    pub y: f64,
    pub flux: f32,
    pub peak: f32,
    pub fwhm: f32,
    /// Narrow over wide extent of the star: 1 for a round star, less for a streak.
    pub roundness: f32,
}

/// A single-channel image to search for stars in.
#[derive(Debug, Clone, Copy)]
pub struct GrayImage<'a> {
    pub width: usize,
    pub height: usize,
    pub data: &'a [f32],
}
