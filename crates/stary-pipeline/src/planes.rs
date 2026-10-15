use rayon::prelude::*;
use stary_core::COLORS;

/// The three color planes of a demosaiced frame.
pub struct Planes {
    pub width: usize,
    pub height: usize,
    pub data: [Vec<u16>; COLORS],
}

/// Splits interleaved RGB samples into planes.
pub fn split_planes(interleaved: &[u16], width: usize, height: usize) -> Planes {
    let mut data: [Vec<u16>; COLORS] = std::array::from_fn(|_| vec![0u16; width * height]);
    let [red, green, blue] = &mut data;
    red.par_iter_mut()
        .zip(green.par_iter_mut())
        .zip(blue.par_iter_mut())
        .zip(interleaved.par_chunks_exact(COLORS))
        .for_each(|(((r, g), b), pixel)| {
            *r = pixel[0];
            *g = pixel[1];
            *b = pixel[2];
        });
    Planes {
        width,
        height,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaved_pixels_are_split_into_planes() {
        let planes = split_planes(&[1, 2, 3, 4, 5, 6], 2, 1);
        assert_eq!(planes.data, [vec![1, 4], vec![2, 5], vec![3, 6]]);
    }
}
