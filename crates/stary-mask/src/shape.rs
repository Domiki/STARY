use crate::{distance::distance_to, error::MaskError, mask::Mask};

/// Half a pixel: the edge of a region lies between its last pixel and the next one.
const HALF_PIXEL: f32 = 0.5;

fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Turns a region of sky into a mask. The edge of the region is moved by `offset` pixels (positive
/// grows the sky, negative shrinks it), and the mask falls from sky to ground over `feather`
/// pixels centred on that edge.
pub fn shaped(
    width: usize,
    height: usize,
    sky: &[bool],
    offset: f32,
    feather: f32,
) -> Result<Mask, MaskError> {
    if width == 0 || height == 0 {
        return Err(MaskError::NoPixels);
    }
    if sky.len() != width * height {
        return Err(MaskError::SizeMismatch);
    }
    let ground: Vec<bool> = sky.iter().map(|&inside| !inside).collect();
    let (Some(to_ground), Some(to_sky)) = (
        distance_to(&ground, width, height),
        distance_to(sky, width, height),
    ) else {
        let all = if sky[0] { 1.0 } else { 0.0 };
        return Mask::new(width, height, vec![all; width * height]);
    };
    let values = sky
        .iter()
        .zip(to_ground.iter().zip(&to_sky))
        .map(|(&inside, (&from_ground, &from_sky))| {
            let inside_edge = if inside {
                from_ground - HALF_PIXEL
            } else {
                HALF_PIXEL - from_sky
            };
            let signed = inside_edge + offset;
            if feather <= 0.0 {
                f32::from(signed > 0.0)
            } else {
                smooth_step(signed / feather + 0.5)
            }
        })
        .collect();
    Mask::new(width, height, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: usize = 20;

    fn upper_half(rows: usize) -> Vec<bool> {
        (0..SIZE * SIZE).map(|index| index / SIZE < rows).collect()
    }

    fn column(mask: &Mask) -> Vec<f32> {
        (0..SIZE).map(|row| mask.values()[row * SIZE]).collect()
    }

    #[test]
    fn without_offset_or_feather_the_mask_is_the_region() {
        let mask = shaped(SIZE, SIZE, &upper_half(10), 0.0, 0.0).unwrap();
        let rows = column(&mask);
        assert!(rows[..10].iter().all(|&value| value == 1.0));
        assert!(rows[10..].iter().all(|&value| value == 0.0));
    }

    #[test]
    fn a_positive_offset_grows_the_sky_and_a_negative_one_shrinks_it() {
        let grown = column(&shaped(SIZE, SIZE, &upper_half(10), 3.0, 0.0).unwrap());
        assert_eq!(grown.iter().filter(|&&value| value == 1.0).count(), 13);
        let shrunk = column(&shaped(SIZE, SIZE, &upper_half(10), -4.0, 0.0).unwrap());
        assert_eq!(shrunk.iter().filter(|&&value| value == 1.0).count(), 6);
    }

    #[test]
    fn the_feather_blends_around_the_edge() {
        let rows = column(&shaped(SIZE, SIZE, &upper_half(10), 0.0, 6.0).unwrap());
        assert!((rows[9] + rows[10] - 1.0).abs() < 1e-5, "{rows:?}");
        assert!(rows[9] > 0.5 && rows[9] < 1.0);
        assert_eq!(rows[0], 1.0);
        assert_eq!(rows[19], 0.0);
        assert!(rows.windows(2).all(|pair| pair[0] >= pair[1]));
        let blended = rows.iter().filter(|&&v| v > 0.0 && v < 1.0).count();
        assert_eq!(blended, 6);
    }

    #[test]
    fn a_region_without_an_edge_stays_as_it_is() {
        let sky = shaped(4, 4, &[true; 16], -3.0, 5.0).unwrap();
        assert!(sky.values().iter().all(|&value| value == 1.0));
        let ground = shaped(4, 4, &[false; 16], 3.0, 5.0).unwrap();
        assert!(ground.values().iter().all(|&value| value == 0.0));
    }

    #[test]
    fn a_curved_edge_is_moved_by_the_same_distance_everywhere() {
        let center = (SIZE as f32 - 1.0) / 2.0;
        let disc: Vec<bool> = (0..SIZE * SIZE)
            .map(|index| {
                let (x, y) = ((index % SIZE) as f32, (index / SIZE) as f32);
                (x - center).hypot(y - center) < 4.0
            })
            .collect();
        let plain = shaped(SIZE, SIZE, &disc, 0.0, 0.0).unwrap();
        let grown = shaped(SIZE, SIZE, &disc, 3.0, 0.0).unwrap();
        let radius = |mask: &Mask| mask.values().iter().sum::<f32>().sqrt();
        let (before, after) = (radius(&plain), radius(&grown));
        assert!(after > before + 2.0, "{before} -> {after}");
    }

    #[test]
    fn the_data_must_match_the_size() {
        assert_eq!(
            shaped(2, 2, &[true; 3], 0.0, 0.0),
            Err(MaskError::SizeMismatch)
        );
    }
}
