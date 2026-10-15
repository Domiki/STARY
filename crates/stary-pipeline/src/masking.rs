use std::sync::Arc;

use rayon::prelude::*;
use stary_mask::Mask;

/// What is done with the ground of a frame, the part the mask does not call sky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    /// The ground goes where the sky goes. Stars are only looked for in the sky.
    WithSky,
    /// The ground is combined on its own, with the frames laid on each other without being moved,
    /// and put under the sky where the mask says ground.
    Separate,
}

/// The sky of a job, as painted on one of its lights, and what to do with the rest.
#[derive(Debug, Clone)]
pub struct Masking {
    pub mask: Arc<Mask>,
    pub ground: Ground,
}

impl Masking {
    pub fn separates_ground(&self) -> bool {
        self.ground == Ground::Separate
    }
}

/// Where a run of rows sits in a frame.
#[derive(Debug, Clone, Copy)]
pub struct Rows {
    pub width: usize,
    /// Rows of the whole frame.
    pub height: usize,
    /// Values per pixel.
    pub channels: usize,
    /// The frame row the run starts at.
    pub first: usize,
}

/// Puts `ground` under `sky` in the proportion the mask gives for each pixel of the rows.
pub fn blend_rows<T: Copy + Send + Sync>(
    sky: &mut [T],
    ground: &[T],
    rows: Rows,
    mask: &Mask,
    mix: impl Fn(T, T, f32) -> T + Sync,
) {
    let line = rows.width * rows.channels;
    sky.par_chunks_mut(line)
        .zip(ground.par_chunks(line))
        .enumerate()
        .for_each_init(
            || vec![0.0f32; rows.width],
            |alpha, (row, (sky_line, ground_line))| {
                mask.fill_row(rows.first + row, rows.width, rows.height, alpha);
                for (pixel, (sky_pixel, ground_pixel)) in sky_line
                    .chunks_exact_mut(rows.channels)
                    .zip(ground_line.chunks_exact(rows.channels))
                    .enumerate()
                {
                    let share = alpha[pixel];
                    if share >= 1.0 {
                        continue;
                    }
                    for (sky_value, ground_value) in pixel_values(sky_pixel, ground_pixel) {
                        *sky_value = mix(*sky_value, ground_value, share);
                    }
                }
            },
        );
}

fn pixel_values<'a, T: Copy>(
    sky: &'a mut [T],
    ground: &'a [T],
) -> impl Iterator<Item = (&'a mut T, T)> {
    sky.iter_mut().zip(ground.iter().copied())
}

pub fn mix_f32(sky: f32, ground: f32, sky_share: f32) -> f32 {
    sky * sky_share + ground * (1.0 - sky_share)
}

pub fn mix_u16(sky: u16, ground: u16, sky_share: f32) -> u16 {
    mix_f32(f32::from(sky), f32::from(ground), sky_share).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn halves() -> Mask {
        Mask::new(1, 4, vec![1.0, 1.0, 0.0, 0.0]).unwrap()
    }

    #[test]
    fn ground_replaces_the_sky_where_the_mask_says_ground() {
        let mut sky = vec![10.0f32; 4 * 2 * 3];
        let ground = vec![2.0f32; 4 * 2 * 3];
        let rows = Rows {
            width: 2,
            height: 4,
            channels: 3,
            first: 0,
        };
        blend_rows(&mut sky, &ground, rows, &halves(), mix_f32);
        assert!(sky[..2 * 2 * 3].iter().all(|&value| value == 10.0));
        assert!(sky[2 * 2 * 3..].iter().all(|&value| value == 2.0));
    }

    #[test]
    fn a_strip_is_blended_by_the_rows_it_covers() {
        let mut sky = vec![9u16; 2];
        let rows = Rows {
            width: 1,
            height: 4,
            channels: 1,
            first: 2,
        };
        blend_rows(&mut sky, &[1, 1], rows, &halves(), mix_u16);
        assert_eq!(sky, vec![1, 1]);
    }

    #[test]
    fn a_half_mask_mixes_both() {
        let mask = Mask::new(1, 1, vec![0.5]).unwrap();
        let mut sky = vec![100u16];
        let rows = Rows {
            width: 1,
            height: 1,
            channels: 1,
            first: 0,
        };
        blend_rows(&mut sky, &[50], rows, &mask, mix_u16);
        assert_eq!(sky, vec![75]);
    }
}
