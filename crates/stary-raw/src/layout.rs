use rsraw::RawImage;
use stary_core::{BayerPattern, CfaPattern, PixelLayout};

use crate::error::RawError;

const XTRANS_FILTERS: u32 = 9;
const SECOND_GREEN: u32 = 3;

fn bayer_color(filters: u32, row: u32, column: u32) -> u8 {
    let index = filters >> (((row << 1 & 14) | (column & 1)) << 1) & 3;
    (if index == SECOND_GREEN { 1 } else { index }) as u8
}

fn bayer_pattern(filters: u32) -> Option<BayerPattern> {
    BayerPattern::from_colors([
        bayer_color(filters, 0, 0),
        bayer_color(filters, 0, 1),
        bayer_color(filters, 1, 0),
        bayer_color(filters, 1, 1),
    ])
}

fn xtrans_matrix(matrix: [[i8; 6]; 6]) -> [[u8; 6]; 6] {
    matrix.map(|row| row.map(|color| (color as u8).min(2)))
}

pub fn layout_of(raw: &RawImage) -> Result<PixelLayout, RawError> {
    let data = raw.as_ref();
    match data.idata.filters {
        0 if data.idata.colors == 1 => Ok(PixelLayout::Mono),
        0 => Ok(PixelLayout::Rgb),
        XTRANS_FILTERS => Ok(PixelLayout::Cfa(CfaPattern::XTrans(xtrans_matrix(
            data.idata.xtrans,
        )))),
        filters => bayer_pattern(filters)
            .map(|pattern| PixelLayout::Cfa(CfaPattern::Bayer(pattern)))
            .ok_or(RawError::UnsupportedCfa),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libraw_filter_word_maps_to_rggb() {
        assert_eq!(bayer_pattern(0x94949494), Some(BayerPattern::Rggb));
        assert_eq!(bayer_pattern(0x16161616), Some(BayerPattern::Bggr));
    }
}
