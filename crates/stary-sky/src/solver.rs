use rayon::prelude::*;

const SEED_STRENGTH: f32 = 1e6;

/// A picture as nodes joined to their right and lower neighbour. A heavy join says the two pixels
/// belong together, a light one that an edge lies between them.
#[derive(Debug, Clone)]
pub struct Lattice {
    pub width: usize,
    pub height: usize,
    right: Vec<f32>,
    down: Vec<f32>,
}

/// What a pixel looks like, to tell edges from smooth parts.
pub type Feature = [f32; 4];

fn join(first: &Feature, second: &Feature, sharpness: f32) -> f32 {
    let distance: f32 = first
        .iter()
        .zip(second)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    (-sharpness * distance).exp()
}

impl Lattice {
    /// The pixels `features` (row by row) joined by how alike they look; `sharpness` is how fast
    /// the join weakens with the difference.
    pub fn from_features(
        width: usize,
        height: usize,
        features: &[Feature],
        sharpness: f32,
    ) -> Self {
        let mut right = vec![0.0; width * height];
        let mut down = vec![0.0; width * height];
        right
            .par_chunks_mut(width)
            .zip(down.par_chunks_mut(width))
            .enumerate()
            .for_each(|(y, (right_line, down_line))| {
                for x in 0..width {
                    let here = &features[y * width + x];
                    if x + 1 < width {
                        right_line[x] = join(here, &features[y * width + x + 1], sharpness);
                    }
                    if y + 1 < height {
                        down_line[x] = join(here, &features[(y + 1) * width + x], sharpness);
                    }
                }
            });
        Self {
            width,
            height,
            right,
            down,
        }
    }

    fn degrees(&self) -> Vec<f32> {
        let mut degree = vec![0.0; self.width * self.height];
        degree
            .par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, line)| {
                for (x, value) in line.iter_mut().enumerate() {
                    let index = y * self.width + x;
                    let mut total = self.right[index] + self.down[index];
                    if x > 0 {
                        total += self.right[index - 1];
                    }
                    if y > 0 {
                        total += self.down[index - self.width];
                    }
                    *value = total;
                }
            });
        degree
    }

    fn apply(&self, strength: &[f32], degree: &[f32], x: &[f32], out: &mut [f32]) {
        out.par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(row, line)| {
                for (column, value) in line.iter_mut().enumerate() {
                    let index = row * self.width + column;
                    let mut sum = (degree[index] + strength[index]) * x[index];
                    if column + 1 < self.width {
                        sum -= self.right[index] * x[index + 1];
                    }
                    if column > 0 {
                        sum -= self.right[index - 1] * x[index - 1];
                    }
                    if row + 1 < self.height {
                        sum -= self.down[index] * x[index + self.width];
                    }
                    if row > 0 {
                        sum -= self.down[index - self.width] * x[index - self.width];
                    }
                    *value = sum;
                }
            });
    }

    /// Smooths `anchor` over the lattice: the result stays near the anchor where `strength` is
    /// high, and is as flat as the joins allow between the pixels. Where `seeds` holds a value the
    /// result is that value. It stops when the values are still off by `tolerance` on average
    /// (judged by what is left of the equations) or after `rounds` rounds.
    pub fn smooth(
        &self,
        anchor: &[f32],
        strength: f32,
        seeds: &[Option<f32>],
        start: &[f32],
        tolerance: f32,
        rounds: usize,
    ) -> Vec<f32> {
        let strengths: Vec<f32> = seeds
            .iter()
            .map(|seed| {
                if seed.is_some() {
                    SEED_STRENGTH
                } else {
                    strength
                }
            })
            .collect();
        let target: Vec<f32> = anchor
            .iter()
            .zip(seeds)
            .map(|(&value, seed)| seed.unwrap_or(value))
            .collect();
        let degree = self.degrees();
        let wanted: Vec<f32> = target
            .iter()
            .zip(&strengths)
            .map(|(value, weight)| value * weight)
            .collect();
        let inverse: Vec<f32> = degree
            .iter()
            .zip(&strengths)
            .map(|(d, s)| 1.0 / (d + s))
            .collect();
        let mut x: Vec<f32> = start
            .iter()
            .zip(seeds)
            .map(|(&value, seed)| seed.unwrap_or(value))
            .collect();
        let mut product = vec![0.0; x.len()];
        self.apply(&strengths, &degree, &x, &mut product);
        let mut residual: Vec<f32> = wanted.iter().zip(&product).map(|(w, p)| w - p).collect();
        let mut direction: Vec<f32> = residual.iter().zip(&inverse).map(|(r, i)| r * i).collect();
        let mut work = dot(&residual, &direction);
        let stop = f64::from(tolerance).powi(2) * x.len() as f64;
        for _ in 0..rounds {
            if work <= stop {
                break;
            }
            self.apply(&strengths, &degree, &direction, &mut product);
            let curvature = dot(&direction, &product);
            if curvature <= 0.0 {
                break;
            }
            let step = (work / curvature) as f32;
            x.par_iter_mut()
                .zip(direction.par_iter())
                .for_each(|(value, d)| *value += step * d);
            residual
                .par_iter_mut()
                .zip(product.par_iter())
                .for_each(|(r, p)| *r -= step * p);
            let preconditioned: Vec<f32> = residual
                .par_iter()
                .zip(&inverse)
                .map(|(r, i)| r * i)
                .collect();
            let next = dot(&residual, &preconditioned);
            let keep = (next / work.max(1e-30)) as f32;
            work = next;
            direction
                .par_iter_mut()
                .zip(preconditioned.par_iter())
                .for_each(|(d, p)| *d = p + keep * *d);
        }
        x.iter().map(|value| value.clamp(0.0, 1.0)).collect()
    }
}

fn dot(a: &[f32], b: &[f32]) -> f64 {
    a.par_iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(values: &[f32]) -> Vec<Feature> {
        values.iter().map(|&v| [v, 0.0, 0.0, 0.0]).collect()
    }

    #[test]
    fn a_noisy_anchor_is_smoothed_inside_a_region_but_not_across_an_edge() {
        let (width, height) = (20, 4);
        let look: Vec<f32> = (0..width * height)
            .map(|index| if index % width < 10 { 0.0 } else { 5.0 })
            .collect();
        let anchor: Vec<f32> = (0..width * height)
            .map(|index| {
                let base: f32 = if index % width < 10 { 0.0 } else { 1.0 };
                let wobble: f32 = if index % 3 == 0 { 0.3 } else { -0.15 };
                (base + if base > 0.5 { -wobble } else { wobble }).clamp(0.0, 1.0)
            })
            .collect();
        let lattice = Lattice::from_features(width, height, &features(&look), 1.0);
        let none = vec![None; width * height];
        let smooth = lattice.smooth(&anchor, 0.05, &none, &anchor, 1e-5, 300);
        let (left, right) = (smooth[width + 2], smooth[width + 17]);
        assert!(left < 0.3, "{left}");
        assert!(right > 0.7, "{right}");
        assert!(smooth[width + 9] < 0.5 && smooth[width + 10] > 0.5);
    }

    #[test]
    fn seeds_are_kept_and_spread_across_a_flat_picture() {
        let (width, height) = (16, 1);
        let lattice = Lattice::from_features(width, height, &features(&vec![0.0; width]), 1.0);
        let mut seeds = vec![None; width];
        seeds[0] = Some(1.0);
        seeds[width - 1] = Some(0.0);
        let anchor = vec![0.5; width];
        let smooth = lattice.smooth(&anchor, 1e-4, &seeds, &anchor, 1e-6, 500);
        assert!((smooth[0] - 1.0).abs() < 1e-3 && smooth[width - 1].abs() < 1e-3);
        assert!(smooth.windows(2).all(|pair| pair[0] >= pair[1] - 1e-4));
        assert!((smooth[width / 2] - 0.5).abs() < 0.1);
    }
}
