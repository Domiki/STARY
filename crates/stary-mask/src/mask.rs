use std::ops::Range;

use crate::error::MaskError;

/// A pixel counts as sky for finding stars when its mask value is at least this.
pub const SKY_THRESHOLD: f32 = 0.8;

/// How much of each pixel is sky: 0 for ground, 1 for sky, in between where the two blend. The
/// mask may have any size; it is laid over a frame by position, not by pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    width: usize,
    height: usize,
    values: Vec<f32>,
}

/// The two neighbours of a position and the weight of the second one.
fn neighbours(position: f64, len: usize) -> (usize, usize, f32) {
    let at = (position - 0.5).clamp(0.0, (len - 1) as f64);
    let first = at.floor() as usize;
    (first, (first + 1).min(len - 1), (at - first as f64) as f32)
}

impl Mask {
    pub fn new(width: usize, height: usize, values: Vec<f32>) -> Result<Self, MaskError> {
        if width == 0 || height == 0 {
            return Err(MaskError::NoPixels);
        }
        if values.len() != width * height {
            return Err(MaskError::SizeMismatch);
        }
        let values = values
            .into_iter()
            .map(|value| {
                if value.is_nan() {
                    0.0
                } else {
                    value.clamp(0.0, 1.0)
                }
            })
            .collect();
        Ok(Self {
            width,
            height,
            values,
        })
    }

    pub fn from_bytes(width: usize, height: usize, bytes: &[u8]) -> Result<Self, MaskError> {
        Self::new(
            width,
            height,
            bytes.iter().map(|&byte| f32::from(byte) / 255.0).collect(),
        )
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// The share of the mask that is sky.
    pub fn sky_fraction(&self) -> f32 {
        let sky = self.values.iter().filter(|&&value| value >= SKY_THRESHOLD);
        sky.count() as f32 / self.values.len() as f32
    }

    /// The mask value at a position given as fractions of the width and the height.
    pub fn value_at(&self, across: f64, down: f64) -> f32 {
        let (left, right, x) = neighbours(across * self.width as f64, self.width);
        let (top, bottom, y) = neighbours(down * self.height as f64, self.height);
        let at = |row: usize, column: usize| self.values[row * self.width + column];
        let upper = at(top, left) * (1.0 - x) + at(top, right) * x;
        let lower = at(bottom, left) * (1.0 - x) + at(bottom, right) * x;
        upper * (1.0 - y) + lower * y
    }

    /// Whether the pixel of a `width` x `height` frame at this position (pixel centres at whole
    /// numbers) is sky.
    pub fn is_sky_at(&self, x: f64, y: f64, width: usize, height: usize) -> bool {
        let across = (x + 0.5) / width as f64;
        let down = (y + 0.5) / height as f64;
        self.value_at(across, down) >= SKY_THRESHOLD
    }

    /// The mask value of every pixel of one row of a `width` x `height` frame.
    pub fn fill_row(&self, row: usize, width: usize, height: usize, out: &mut [f32]) {
        let (top, bottom, y) = neighbours(
            (row as f64 + 0.5) / height as f64 * self.height as f64,
            self.height,
        );
        let (upper, lower) = (
            &self.values[top * self.width..(top + 1) * self.width],
            &self.values[bottom * self.width..(bottom + 1) * self.width],
        );
        for (column, value) in out.iter_mut().enumerate().take(width) {
            let across = (column as f64 + 0.5) / width as f64 * self.width as f64;
            let (left, right, x) = neighbours(across, self.width);
            let top_value = upper[left] * (1.0 - x) + upper[right] * x;
            let bottom_value = lower[left] * (1.0 - x) + lower[right] * x;
            *value = top_value * (1.0 - y) + bottom_value * y;
        }
    }

    /// Whether any pixel of these rows of a `height` high frame is not fully sky.
    pub fn has_ground_in(&self, rows: Range<usize>, height: usize) -> bool {
        let scale = self.height as f64 / height as f64;
        let first = ((rows.start as f64 * scale).floor() as usize).saturating_sub(1);
        let last = (((rows.end as f64 * scale).ceil() as usize) + 1).min(self.height);
        self.values[first * self.width..last * self.width]
            .iter()
            .any(|&value| value < 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn halves() -> Mask {
        // top half sky, bottom half ground, 2 x 4 pixels
        Mask::new(2, 4, vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]).unwrap()
    }

    #[test]
    fn a_mask_needs_pixels_and_matching_data() {
        assert_eq!(Mask::new(0, 3, vec![]), Err(MaskError::NoPixels));
        assert_eq!(Mask::new(2, 2, vec![1.0]), Err(MaskError::SizeMismatch));
    }

    #[test]
    fn values_are_kept_between_zero_and_one() {
        let mask = Mask::new(2, 1, vec![f32::NAN, 7.0]).unwrap();
        assert_eq!(mask.values(), &[0.0, 1.0]);
    }

    #[test]
    fn bytes_become_fractions() {
        let mask = Mask::from_bytes(3, 1, &[0, 51, 255]).unwrap();
        assert_eq!(mask.values(), &[0.0, 0.2, 1.0]);
    }

    #[test]
    fn the_sky_fraction_counts_pixels_that_are_sky_enough() {
        let mask = Mask::new(4, 1, vec![1.0, 0.85, 0.5, 0.0]).unwrap();
        assert_eq!(mask.sky_fraction(), 0.5);
    }

    #[test]
    fn a_mask_is_laid_over_a_frame_of_another_size_by_position() {
        let mask = halves();
        assert!(mask.is_sky_at(10.0, 10.0, 100, 100));
        assert!(!mask.is_sky_at(10.0, 90.0, 100, 100));
        assert!(mask.is_sky_at(500.0, 200.0, 1000, 1000));
        assert!(!mask.is_sky_at(500.0, 800.0, 1000, 1000));
    }

    #[test]
    fn values_blend_between_pixels() {
        let mask = halves();
        assert!((mask.value_at(0.5, 0.5) - 0.5).abs() < 1e-6);
        assert_eq!(mask.value_at(0.5, 0.0), 1.0);
        assert_eq!(mask.value_at(0.5, 1.0), 0.0);
    }

    #[test]
    fn a_row_of_a_frame_follows_the_mask() {
        let mask = halves();
        let mut row = vec![0.0; 6];
        mask.fill_row(0, 6, 8, &mut row);
        assert!(row.iter().all(|&value| value == 1.0));
        mask.fill_row(7, 6, 8, &mut row);
        assert!(row.iter().all(|&value| value == 0.0));
    }

    #[test]
    fn rows_without_ground_are_told_apart() {
        let mask = halves();
        assert!(!mask.has_ground_in(0..20, 100));
        assert!(mask.has_ground_in(40..60, 100));
        assert!(mask.has_ground_in(90..100, 100));
    }
}
