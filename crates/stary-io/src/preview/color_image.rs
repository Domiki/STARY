use stary_core::{BayerPattern, PreviewImage, stretch::auto_stretch};

#[derive(Debug, Clone)]
pub struct ColorImage {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub samples: Vec<f32>,
}

impl ColorImage {
    pub fn mono(width: usize, height: usize, samples: Vec<f32>) -> Self {
        Self {
            width,
            height,
            channels: 1,
            samples,
        }
    }

    pub fn from_planes(width: usize, height: usize, planes: &[f32]) -> Self {
        let plane_len = width * height;
        let samples = (0..plane_len)
            .flat_map(|index| (0..3).map(move |plane| planes[plane * plane_len + index]))
            .collect();
        Self {
            width,
            height,
            channels: 3,
            samples,
        }
    }

    pub fn from_bayer(width: usize, height: usize, mosaic: &[f32], pattern: BayerPattern) -> Self {
        let (out_width, out_height) = (width / 2, height / 2);
        let colors = pattern.colors();
        let samples = (0..out_height)
            .flat_map(|y| (0..out_width).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let mut sums = [0.0f32; 3];
                let mut counts = [0.0f32; 3];
                for (slot, color) in colors.iter().enumerate() {
                    let value = mosaic[(2 * y + slot / 2) * width + 2 * x + slot % 2];
                    sums[*color as usize] += value;
                    counts[*color as usize] += 1.0;
                }
                [0, 1, 2].map(|channel| sums[channel] / counts[channel].max(1.0))
            })
            .collect();
        Self {
            width: out_width,
            height: out_height,
            channels: 3,
            samples,
        }
    }

    pub fn box_downscale(self, max_edge: usize) -> Self {
        let factor = self.width.max(self.height).div_ceil(max_edge.max(1));
        if factor <= 1 {
            return self;
        }
        let (out_width, out_height) = (self.width.div_ceil(factor), self.height.div_ceil(factor));
        let samples = (0..out_height)
            .flat_map(|y| (0..out_width).map(move |x| (x, y)))
            .flat_map(|(x, y)| (0..self.channels).map(move |channel| (x, y, channel)))
            .map(|(x, y, channel)| self.block_mean(x * factor, y * factor, factor, channel))
            .collect();
        Self {
            width: out_width,
            height: out_height,
            channels: self.channels,
            samples,
        }
    }

    fn block_mean(&self, left: usize, top: usize, size: usize, channel: usize) -> f32 {
        let right = (left + size).min(self.width);
        let bottom = (top + size).min(self.height);
        let total: f32 = (top..bottom)
            .flat_map(|y| (left..right).map(move |x| (x, y)))
            .map(|(x, y)| self.samples[(y * self.width + x) * self.channels + channel])
            .sum();
        total / ((right - left) * (bottom - top)) as f32
    }

    pub fn flip_vertical(self) -> Self {
        let row_len = self.width * self.channels;
        let samples = self
            .samples
            .chunks_exact(row_len)
            .rev()
            .flatten()
            .copied()
            .collect();
        Self { samples, ..self }
    }

    pub fn into_stretched_preview(self) -> PreviewImage {
        let stretched = auto_stretch(&self.samples, self.channels);
        let rgb8 = if self.channels == 1 {
            stretched.iter().flat_map(|&value| [value; 3]).collect()
        } else {
            stretched
        };
        PreviewImage {
            width: self.width as u32,
            height: self.height as u32,
            rgb8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayer_binning_averages_the_two_greens() {
        let mosaic = [10.0, 4.0, 6.0, 20.0];
        let image = ColorImage::from_bayer(2, 2, &mosaic, BayerPattern::Rggb);
        assert_eq!(image.samples, vec![10.0, 5.0, 20.0]);
    }

    #[test]
    fn box_downscale_averages_blocks() {
        let image = ColorImage::mono(4, 2, vec![1.0, 3.0, 5.0, 7.0, 1.0, 3.0, 5.0, 7.0]);
        let reduced = image.box_downscale(2);
        assert_eq!((reduced.width, reduced.height), (2, 1));
        assert_eq!(reduced.samples, vec![2.0, 6.0]);
    }

    #[test]
    fn flip_reverses_row_order() {
        let image = ColorImage::mono(2, 2, vec![1.0, 2.0, 3.0, 4.0]).flip_vertical();
        assert_eq!(image.samples, vec![3.0, 4.0, 1.0, 2.0]);
    }
}
