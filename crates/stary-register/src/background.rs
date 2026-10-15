use rayon::prelude::*;

use crate::star::GrayImage;

const TILE: usize = 64;
const STRIDE: usize = 2;
const MAD_TO_SIGMA: f32 = 1.4826;

/// The slowly varying sky level of an image on a coarse mesh, with the typical pixel noise.
pub struct Background {
    mesh: Vec<f32>,
    columns: usize,
    rows: usize,
    pub sigma: f32,
}

pub fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let middle = values.len() / 2;
    let (_, value, _) = values.select_nth_unstable_by(middle, f32::total_cmp);
    *value
}

fn tile_statistics(image: &GrayImage, column: usize, row: usize) -> (f32, f32) {
    let x_range = column * TILE..((column + 1) * TILE).min(image.width);
    let y_range = row * TILE..((row + 1) * TILE).min(image.height);
    let mut values: Vec<f32> = y_range
        .step_by(STRIDE)
        .flat_map(|y| {
            x_range
                .clone()
                .step_by(STRIDE)
                .map(move |x| image.data[y * image.width + x])
        })
        .filter(|value| value.is_finite())
        .collect();
    let center = median(&mut values);
    let mut deviations: Vec<f32> = values.iter().map(|value| (value - center).abs()).collect();
    (center, MAD_TO_SIGMA * median(&mut deviations))
}

fn smoothed_mesh(mesh: &[f32], columns: usize, rows: usize) -> Vec<f32> {
    (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (row, column)))
        .map(|(row, column)| {
            let inside = (1..rows.saturating_sub(1)).contains(&row)
                && (1..columns.saturating_sub(1)).contains(&column);
            if !inside {
                return mesh[row * columns + column];
            }
            let mut neighbours: Vec<f32> = (row - 1..=row + 1)
                .flat_map(|r| (column - 1..=column + 1).map(move |c| mesh[r * columns + c]))
                .collect();
            median(&mut neighbours)
        })
        .collect()
}

impl Background {
    pub fn estimate(image: &GrayImage) -> Self {
        let columns = image.width.div_ceil(TILE).max(1);
        let rows = image.height.div_ceil(TILE).max(1);
        let stats: Vec<(f32, f32)> = (0..columns * rows)
            .into_par_iter()
            .map(|index| tile_statistics(image, index % columns, index / columns))
            .collect();
        let centers: Vec<f32> = stats.iter().map(|stat| stat.0).collect();
        let mut sigmas: Vec<f32> = stats.iter().map(|stat| stat.1).collect();
        Self {
            mesh: smoothed_mesh(&centers, columns, rows),
            columns,
            rows,
            sigma: median(&mut sigmas),
        }
    }

    /// The sky level at a pixel, interpolated between the tile centres and continued linearly
    /// beyond the outermost centres.
    pub fn at(&self, x: usize, y: usize) -> f32 {
        let locate = |position: usize, count: usize| {
            let tile = (position as f32 + 0.5) / TILE as f32 - 0.5;
            if count < 2 {
                return (0, 0, 0.0);
            }
            let low = (tile.floor().max(0.0) as usize).min(count - 2);
            (low, low + 1, tile - low as f32)
        };
        let (c0, c1, fx) = locate(x, self.columns);
        let (r0, r1, fy) = locate(y, self.rows);
        let at = |r: usize, c: usize| self.mesh[r * self.columns + c];
        let top = at(r0, c0) * (1.0 - fx) + at(r0, c1) * fx;
        let bottom = at(r1, c0) * (1.0 - fx) + at(r1, c1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_noisy_sky_gives_its_level_and_noise() {
        let (width, height) = (200, 150);
        let data: Vec<f32> = (0..width * height)
            .map(|index| 1000.0 + ((index * 7919) % 21) as f32 - 10.0)
            .collect();
        let background = Background::estimate(&GrayImage {
            width,
            height,
            data: &data,
        });
        assert!((background.at(100, 70) - 1000.0).abs() < 2.0);
        assert!(
            background.sigma > 3.0 && background.sigma < 9.0,
            "{}",
            background.sigma
        );
    }

    #[test]
    fn a_gradient_is_followed() {
        let (width, height) = (256, 128);
        let data: Vec<f32> = (0..width * height)
            .map(|index| 500.0 + 2.0 * (index % width) as f32)
            .collect();
        let background = Background::estimate(&GrayImage {
            width,
            height,
            data: &data,
        });
        for x in [40usize, 128, 220] {
            assert!(
                (background.at(x, 60) - (500.0 + 2.0 * x as f32)).abs() < 3.0,
                "{x}"
            );
        }
    }

    #[test]
    fn median_of_an_even_count_takes_the_upper_middle() {
        assert_eq!(median(&mut [4.0, 1.0, 3.0, 2.0]), 3.0);
        assert_eq!(median(&mut []), 0.0);
    }
}
