use std::{fs::File, io::BufReader, path::Path};

use stary_core::PixelLayout;

use crate::{
    error::IoError,
    fits::{
        Geometry, Header, RowOrder, explicit_layout_of, geometry_of, read_header,
        read_physical_samples, reverse_rows, row_order_of,
    },
};

/// The pixels of a FITS file as top-down rows, with the samples of a color image interleaved.
#[derive(Debug)]
pub struct NormalizedFits {
    pub header: Header,
    pub geometry: Geometry,
    pub layout: Option<PixelLayout>,
    pub samples: Vec<f32>,
}

fn interleave_planes(samples: &[f32], width: usize, height: usize, flip: bool) -> Vec<f32> {
    let plane_len = width * height;
    let mut out = vec![0.0; plane_len * 3];
    for plane in 0..3 {
        for row in 0..height {
            let target = if flip { height - 1 - row } else { row };
            let source = &samples[plane * plane_len + row * width..][..width];
            for (column, value) in source.iter().enumerate() {
                out[(target * width + column) * 3 + plane] = *value;
            }
        }
    }
    out
}

pub fn read_normalized(path: &Path) -> Result<NormalizedFits, IoError> {
    let file = File::open(path).map_err(IoError::read(path))?;
    let header = read_header(path, &mut BufReader::new(file))?;
    let geometry = geometry_of(path, &header)?;
    if geometry.planes != 1 && geometry.planes != 3 {
        return Err(IoError::unsupported(
            path,
            format!("FITS with {} image planes", geometry.planes),
        ));
    }
    let bottom_up = row_order_of(&header) == RowOrder::BottomUp;
    let mut samples = read_physical_samples(path, &header, &geometry)?;
    let layout = explicit_layout_of(&header, geometry.planes);
    if geometry.planes == 3 {
        samples = interleave_planes(&samples, geometry.width, geometry.height, bottom_up);
    } else if bottom_up {
        reverse_rows(&mut samples, geometry.width);
    }
    Ok(NormalizedFits {
        header,
        geometry,
        layout,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planes_become_interleaved_pixels_and_can_be_flipped() {
        // 2x2, planes R=[1,2,3,4] G=[5,6,7,8] B=[9,10,11,12]
        let samples: Vec<f32> = (1..=12).map(|value| value as f32).collect();
        assert_eq!(
            interleave_planes(&samples, 2, 2, false),
            vec![
                1.0, 5.0, 9.0, 2.0, 6.0, 10.0, 3.0, 7.0, 11.0, 4.0, 8.0, 12.0
            ]
        );
        assert_eq!(
            interleave_planes(&samples, 2, 2, true),
            vec![
                3.0, 7.0, 11.0, 4.0, 8.0, 12.0, 1.0, 5.0, 9.0, 2.0, 6.0, 10.0
            ]
        );
    }
}
