use rayon::prelude::*;
use stary_core::{COLORS, LinearRgb16};
use stary_register::Transform;
use stary_stack::{
    Kernel, SourcePlane, estimate_sky, partial_motion, steps_between, warp_max_rows,
};

use crate::planes::Planes;

/// Values of a moved picture count only when they stand this many noise widths above the sky.
const FLOOR_SIGMAS: f32 = 3.0;

/// The picture the pictures of the stretches are laid on, keeping the brightest value everywhere.
pub struct Canvas {
    width: usize,
    height: usize,
    kept: [Vec<f32>; COLORS],
}

fn keep_brighter(kept: &mut [f32], plane: &[u16]) {
    kept.par_iter_mut()
        .zip(plane.par_iter())
        .for_each(|(kept, value)| *kept = kept.max(f32::from(*value)));
}

/// Where the pixels of each picture in between a picture and the next one come from: the picture
/// moved a fraction of `motion` along the way.
fn steps_to_source(
    motion: &Transform,
    width: usize,
    height: usize,
    spacing: f64,
) -> Vec<Transform> {
    let steps = steps_between(motion, width, height, spacing);
    (1..=steps)
        .filter_map(|step| {
            let fraction = step as f64 / (steps + 1) as f64;
            partial_motion(motion, fraction).inverse()
        })
        .collect()
}

/// What a color must exceed in a picture of merged lights: the sky is the lighter the more lights
/// were merged, and its noise is narrower, so the level is taken from the picture itself.
fn floors_of(planes: &Planes) -> [f32; COLORS] {
    std::array::from_fn(|channel| {
        let sky = estimate_sky(&planes.data[channel], planes.width);
        sky.location + FLOOR_SIGMAS * sky.scale
    })
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            kept: std::array::from_fn(|_| vec![0.0; width * height]),
        }
    }

    /// Lays the picture on as it is.
    pub fn add_unmoved(&mut self, planes: &Planes) {
        for (kept, plane) in self.kept.iter_mut().zip(&planes.data) {
            keep_brighter(kept, plane);
        }
    }

    /// Lays the picture on, and the pictures in between it and the next one, which it is moved
    /// along `motion` to make.
    pub fn add_filled(&mut self, planes: &Planes, motion: &Transform, spacing: f64) {
        self.add_unmoved(planes);
        let floors = floors_of(planes);
        for to_source in steps_to_source(motion, planes.width, planes.height, spacing) {
            for ((kept, plane), floor) in self.kept.iter_mut().zip(&planes.data).zip(floors) {
                let source = SourcePlane {
                    width: planes.width,
                    height: planes.height,
                    data: plane,
                };
                warp_max_rows(
                    &source,
                    &to_source,
                    Kernel::Bilinear,
                    0..planes.height,
                    planes.width,
                    kept,
                    floor,
                );
            }
        }
    }

    pub fn into_image(self) -> LinearRgb16 {
        let [red, green, blue] = &self.kept;
        let mut samples = vec![0u16; self.width * self.height * COLORS];
        samples
            .par_chunks_exact_mut(COLORS)
            .zip(red.par_iter().zip(green.par_iter()).zip(blue.par_iter()))
            .for_each(|(pixel, ((r, g), b))| {
                for (out, value) in pixel.iter_mut().zip([r, g, b]) {
                    *out = value.round().clamp(0.0, f32::from(u16::MAX)) as u16;
                }
            });
        LinearRgb16 {
            width: self.width,
            height: self.height,
            samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planes(values: [u16; 3]) -> Planes {
        Planes {
            width: 3,
            height: 1,
            data: values.map(|value| vec![value, 0, 0]),
        }
    }

    #[test]
    fn the_brighter_value_is_kept_in_every_channel() {
        let mut canvas = Canvas::new(3, 1);
        canvas.add_unmoved(&planes([5, 0, 9]));
        canvas.add_unmoved(&planes([3, 7, 1]));
        assert_eq!(canvas.into_image().samples, vec![5, 7, 9, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_picture_moved_along_its_motion_leaves_copies_in_between() {
        let mut canvas = Canvas::new(40, 1);
        let mut plane = vec![100u16; 40];
        plane[5] = 20000;
        let picture = Planes {
            width: 40,
            height: 1,
            data: [plane.clone(), plane.clone(), plane],
        };
        let motion = Transform::from_affine(1.0, 0.0, 10.0, 0.0, 1.0, 0.0);
        canvas.add_filled(&picture, &motion, 1.0);
        let image = canvas.into_image();
        let at = |x: usize| image.samples[x * 3];
        assert_eq!(at(5), 20000);
        assert!(at(10) > 10000 && at(12) > 10000, "{} {}", at(10), at(12));
        assert!(at(25) < 1000);
        assert!(at(2) <= 100);
    }

    #[test]
    fn an_empty_canvas_is_black() {
        assert_eq!(Canvas::new(3, 1).into_image().samples, vec![0; 9]);
    }
}
