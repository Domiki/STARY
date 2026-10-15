use rsraw::RawImage;
use stary_core::{Frame, PixelLayout};

use crate::{
    black_level::effective_black,
    error::RawError,
    layout::layout_of,
    sensor_buffer::{ActiveArea, active_area, sensor_buffer},
};

pub fn crop_active_area(
    samples: &[u16],
    pitch: usize,
    rows: usize,
    area: &ActiveArea,
) -> Result<Vec<u16>, RawError> {
    let fits = area.left + area.width <= pitch && area.top + area.height <= rows;
    if !fits || area.width == 0 || area.height == 0 || samples.len() < pitch * rows {
        return Err(RawError::BadGeometry);
    }
    Ok((area.top..area.top + area.height)
        .flat_map(|row| {
            let start = row * pitch + area.left;
            samples[start..start + area.width].iter().copied()
        })
        .collect())
}

pub fn decode_cfa(bytes: &[u8]) -> Result<Frame, RawError> {
    let mut raw = RawImage::open(bytes).map_err(|error| RawError::Open(format!("{error:?}")))?;
    let layout @ PixelLayout::Cfa(_) = layout_of(&raw)? else {
        return Err(RawError::UnsupportedCfa);
    };
    raw.unpack()
        .map_err(|error| RawError::Unpack(format!("{error:?}")))?;
    let area = active_area(&raw);
    let buffer = sensor_buffer(&raw)?;
    let samples = crop_active_area(buffer.samples, buffer.pitch, buffer.rows, &area)?;
    let color = &raw.as_ref().color;
    Ok(Frame {
        width: area.width,
        height: area.height,
        layout,
        black: effective_black(color.black, &color.cblack),
        maximum: u16::try_from(color.maximum).unwrap_or(u16::MAX),
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(left: usize, top: usize, width: usize, height: usize) -> ActiveArea {
        ActiveArea {
            left,
            top,
            width,
            height,
        }
    }

    #[test]
    fn crops_the_visible_area_out_of_a_padded_buffer() {
        // 4 columns of pitch, 3 rows; visible area is the 2x2 block starting at (1, 1)
        let samples: Vec<u16> = (0..12).collect();
        let cropped = crop_active_area(&samples, 4, 3, &area(1, 1, 2, 2)).unwrap();
        assert_eq!(cropped, vec![5, 6, 9, 10]);
    }

    #[test]
    fn rejects_an_area_that_does_not_fit() {
        let samples: Vec<u16> = (0..12).collect();
        assert!(crop_active_area(&samples, 4, 3, &area(3, 0, 2, 1)).is_err());
        assert!(crop_active_area(&samples, 4, 3, &area(0, 2, 1, 2)).is_err());
        assert!(crop_active_area(&samples, 4, 3, &area(0, 0, 0, 1)).is_err());
    }

    #[test]
    fn rejects_a_buffer_shorter_than_its_geometry() {
        let samples: Vec<u16> = (0..5).collect();
        assert!(crop_active_area(&samples, 4, 3, &area(0, 0, 1, 1)).is_err());
    }
}
