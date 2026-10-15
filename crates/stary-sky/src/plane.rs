use rayon::prelude::*;

/// One value per pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

fn kernel(sigma: f32) -> Vec<f32> {
    let radius = (3.0 * sigma).ceil().max(1.0) as isize;
    let weights: Vec<f32> = (-radius..=radius)
        .map(|offset| (-((offset * offset) as f32) / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f32 = weights.iter().sum();
    weights.into_iter().map(|weight| weight / total).collect()
}

fn clamped(index: isize, len: usize) -> usize {
    index.clamp(0, len as isize - 1) as usize
}

impl Plane {
    pub fn new(width: usize, height: usize, data: Vec<f32>) -> Self {
        debug_assert_eq!(data.len(), width * height);
        Self {
            width,
            height,
            data,
        }
    }

    pub fn filled(width: usize, height: usize, value: f32) -> Self {
        Self::new(width, height, vec![value; width * height])
    }

    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    pub fn map(&self, change: impl Fn(f32) -> f32 + Sync) -> Self {
        Self::new(
            self.width,
            self.height,
            self.data.par_iter().map(|&value| change(value)).collect(),
        )
    }

    fn across(&self, weights: &[f32]) -> Self {
        let radius = (weights.len() / 2) as isize;
        let mut out = vec![0.0; self.data.len()];
        out.par_chunks_mut(self.width)
            .zip(self.data.par_chunks(self.width))
            .for_each(|(line, source)| {
                for (x, value) in line.iter_mut().enumerate() {
                    *value = weights
                        .iter()
                        .enumerate()
                        .map(|(tap, weight)| {
                            weight * source[clamped(x as isize + tap as isize - radius, self.width)]
                        })
                        .sum();
                }
            });
        Self::new(self.width, self.height, out)
    }

    fn down(&self, weights: &[f32]) -> Self {
        let radius = (weights.len() / 2) as isize;
        let mut out = vec![0.0; self.data.len()];
        out.par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, line)| {
                for (tap, weight) in weights.iter().enumerate() {
                    let row = clamped(y as isize + tap as isize - radius, self.height);
                    let source = &self.data[row * self.width..(row + 1) * self.width];
                    for (value, sample) in line.iter_mut().zip(source) {
                        *value += weight * sample;
                    }
                }
            });
        Self::new(self.width, self.height, out)
    }

    pub fn gaussian(&self, sigma: f32) -> Self {
        let weights = kernel(sigma);
        self.across(&weights).down(&weights)
    }

    fn extreme(&self, radius: usize, pick: fn(f32, f32) -> f32) -> Self {
        let reach = radius as isize;
        let mut across = vec![0.0; self.data.len()];
        across
            .par_chunks_mut(self.width)
            .zip(self.data.par_chunks(self.width))
            .for_each(|(line, source)| {
                for (x, value) in line.iter_mut().enumerate() {
                    *value = (-reach..=reach)
                        .map(|offset| source[clamped(x as isize + offset, self.width)])
                        .reduce(pick)
                        .unwrap_or(0.0);
                }
            });
        let mut out = vec![0.0; self.data.len()];
        out.par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, line)| {
                let first = clamped(y as isize - reach, self.height);
                line.copy_from_slice(&across[first * self.width..(first + 1) * self.width]);
                for offset in -reach..=reach {
                    let row = clamped(y as isize + offset, self.height);
                    for (value, sample) in line
                        .iter_mut()
                        .zip(&across[row * self.width..(row + 1) * self.width])
                    {
                        *value = pick(*value, *sample);
                    }
                }
            });
        Self::new(self.width, self.height, out)
    }

    /// Takes away what is brighter than a square of `2 * radius + 1` pixels: stars and noise.
    pub fn opened(&self, radius: usize) -> Self {
        self.extreme(radius, f32::min).extreme(radius, f32::max)
    }

    /// Half the size, every pixel the mean of the 2 x 2 it covers.
    pub fn halved(&self) -> Self {
        let (width, height) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let mut out = vec![0.0; width * height];
        out.par_chunks_mut(width).enumerate().for_each(|(y, line)| {
            for (x, value) in line.iter_mut().enumerate() {
                let sample = |dx: usize, dy: usize| {
                    self.at(
                        (2 * x + dx).min(self.width - 1),
                        (2 * y + dy).min(self.height - 1),
                    )
                };
                *value = (sample(0, 0) + sample(1, 0) + sample(0, 1) + sample(1, 1)) / 4.0;
            }
        });
        Self::new(width, height, out)
    }

    /// The plane stretched to another size, between pixel centres.
    pub fn resized(&self, width: usize, height: usize) -> Self {
        let (scale_x, scale_y) = (
            self.width as f32 / width as f32,
            self.height as f32 / height as f32,
        );
        let mut out = vec![0.0; width * height];
        out.par_chunks_mut(width).enumerate().for_each(|(y, line)| {
            let source_y = ((y as f32 + 0.5) * scale_y - 0.5).clamp(0.0, (self.height - 1) as f32);
            let (top, along_y) = (source_y.floor() as usize, source_y.fract());
            let bottom = (top + 1).min(self.height - 1);
            for (x, value) in line.iter_mut().enumerate() {
                let source_x =
                    ((x as f32 + 0.5) * scale_x - 0.5).clamp(0.0, (self.width - 1) as f32);
                let (left, along_x) = (source_x.floor() as usize, source_x.fract());
                let right = (left + 1).min(self.width - 1);
                let upper = self.at(left, top) * (1.0 - along_x) + self.at(right, top) * along_x;
                let lower =
                    self.at(left, bottom) * (1.0 - along_x) + self.at(right, bottom) * along_x;
                *value = upper * (1.0 - along_y) + lower * along_y;
            }
        });
        Self::new(width, height, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blurring_a_flat_plane_changes_nothing() {
        let flat = Plane::filled(9, 7, 0.4);
        let blurred = flat.gaussian(1.5);
        assert!(blurred.data.iter().all(|value| (value - 0.4).abs() < 1e-5));
    }

    #[test]
    fn blurring_spreads_a_point_and_keeps_its_total() {
        let mut point = Plane::filled(21, 21, 0.0);
        point.data[10 * 21 + 10] = 1.0;
        let blurred = point.gaussian(2.0);
        assert!(blurred.at(10, 10) < 0.2 && blurred.at(10, 10) > blurred.at(13, 10));
        let total: f32 = blurred.data.iter().sum();
        assert!((total - 1.0).abs() < 1e-3);
    }

    #[test]
    fn opening_removes_a_bright_point_but_not_a_bright_block() {
        let mut plane = Plane::filled(20, 20, 0.1);
        plane.data[5 * 20 + 5] = 1.0;
        for y in 10..18 {
            for x in 10..18 {
                plane.data[y * 20 + x] = 0.8;
            }
        }
        let opened = plane.opened(2);
        assert!((opened.at(5, 5) - 0.1).abs() < 1e-6);
        assert!((opened.at(13, 13) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn halving_averages_blocks() {
        let plane = Plane::new(2, 2, vec![1.0, 3.0, 5.0, 7.0]);
        assert_eq!(plane.halved().data, vec![4.0]);
    }

    #[test]
    fn resizing_keeps_a_constant_and_follows_a_ramp() {
        let constant = Plane::filled(4, 3, 0.7).resized(9, 8);
        assert!(constant.data.iter().all(|value| (value - 0.7).abs() < 1e-6));
        let ramp = Plane::new(4, 1, vec![0.0, 1.0, 2.0, 3.0]).resized(8, 1);
        assert!(ramp.data.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
