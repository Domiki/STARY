use stary_register::Transform;

const MAX_STEPS: usize = 400;

/// The transform a fraction `t` of the way from the identity to `motion`. The motion between two
/// consecutive frames is small, so stepping along it in a straight line is as good as following
/// the arc of the sky's rotation.
pub fn partial_motion(motion: &Transform, t: f64) -> Transform {
    let mut m = motion.m;
    for (row, line) in m.iter_mut().enumerate() {
        for (column, value) in line.iter_mut().enumerate() {
            let identity = if row == column { 1.0 } else { 0.0 };
            *value = identity + t * (*value - identity);
        }
    }
    Transform { m }
}

/// How far a point of the image moves at most (corners and middle of the edges and the centre).
pub fn largest_motion(motion: &Transform, width: usize, height: usize) -> f64 {
    let (right, bottom) = (
        width.saturating_sub(1) as f64,
        height.saturating_sub(1) as f64,
    );
    let (middle_x, middle_y) = (right / 2.0, bottom / 2.0);
    [
        (0.0, 0.0),
        (right, 0.0),
        (0.0, bottom),
        (right, bottom),
        (middle_x, 0.0),
        (middle_x, bottom),
        (0.0, middle_y),
        (right, middle_y),
        (middle_x, middle_y),
    ]
    .into_iter()
    .map(|point| {
        let moved = motion.apply(point);
        (moved.0 - point.0).hypot(moved.1 - point.1)
    })
    .fold(0.0, f64::max)
}

/// How many frames to make between two frames so that the images of a star lie no further apart
/// than `spacing` pixels.
pub fn steps_between(motion: &Transform, width: usize, height: usize, spacing: f64) -> usize {
    let intervals = (largest_motion(motion, width, height) / spacing.max(0.1)).ceil() as usize;
    intervals.saturating_sub(1).min(MAX_STEPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shift(dx: f64, dy: f64) -> Transform {
        Transform::from_affine(1.0, 0.0, dx, 0.0, 1.0, dy)
    }

    #[test]
    fn half_of_a_shift_is_half_the_distance() {
        let half = partial_motion(&shift(10.0, -4.0), 0.5);
        assert_eq!(half.apply((3.0, 3.0)), (8.0, 1.0));
        assert_eq!(partial_motion(&shift(10.0, -4.0), 0.0), Transform::IDENTITY);
        assert_eq!(partial_motion(&shift(10.0, -4.0), 1.0), shift(10.0, -4.0));
    }

    #[test]
    fn a_rotation_is_stepped_along_its_direction() {
        let angle = 0.01f64;
        let turn = Transform::from_affine(
            angle.cos(),
            -angle.sin(),
            0.0,
            angle.sin(),
            angle.cos(),
            0.0,
        );
        let half = partial_motion(&turn, 0.5);
        let (x, y) = half.apply((1000.0, 0.0));
        // a point 1000 px from the centre moves about 5 px after half of a 0.01 rad turn
        assert!((y - 5.0).abs() < 0.1 && (x - 1000.0).abs() < 0.1, "{x},{y}");
    }

    #[test]
    fn the_biggest_motion_of_a_shift_is_its_length() {
        assert!((largest_motion(&shift(3.0, 4.0), 500, 400) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn the_biggest_motion_of_a_rotation_is_at_the_edge() {
        let angle = 0.01f64;
        let turn = Transform::from_affine(
            angle.cos(),
            -angle.sin(),
            0.0,
            angle.sin(),
            angle.cos(),
            0.0,
        );
        let motion = largest_motion(&turn, 3000, 2000);
        let far = (3000.0f64.powi(2) + 2000.0f64.powi(2)).sqrt() * angle;
        assert!((motion - far).abs() / far < 0.01, "{motion} vs {far}");
    }

    #[test]
    fn the_number_of_steps_fills_the_gap_at_the_spacing() {
        // 20 px with images 2 px apart: 10 intervals, 9 frames in between
        assert_eq!(steps_between(&shift(20.0, 0.0), 100, 100, 2.0), 9);
        assert_eq!(steps_between(&shift(1.0, 0.0), 100, 100, 2.0), 0);
        assert_eq!(steps_between(&Transform::IDENTITY, 100, 100, 2.0), 0);
        assert_eq!(steps_between(&shift(1.0e6, 0.0), 100, 100, 1.0), MAX_STEPS);
    }
}
