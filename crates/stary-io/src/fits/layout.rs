use stary_core::{BayerPattern, CfaPattern, PixelLayout};

use crate::fits::Header;

fn shifted_pattern(pattern: BayerPattern, x_offset: u8, y_offset: u8) -> Option<BayerPattern> {
    let [a, b, c, d] = pattern.colors();
    let columns_swapped = if x_offset % 2 == 1 {
        [b, a, d, c]
    } else {
        [a, b, c, d]
    };
    let [a, b, c, d] = columns_swapped;
    BayerPattern::from_colors(if y_offset % 2 == 1 {
        [c, d, a, b]
    } else {
        [a, b, c, d]
    })
}

fn offset(header: &Header, key: &str, period: u8) -> u8 {
    header
        .number(key)
        .map_or(0, |value| value.rem_euclid(f64::from(period)) as u8)
}

const XTRANS_SIDE: usize = 6;

/// An X-Trans pattern written as 36 letters, row by row as the image looks from the top.
fn xtrans_from_keyword(text: &str) -> Option<[[u8; XTRANS_SIDE]; XTRANS_SIDE]> {
    let colors: Vec<u8> = text
        .trim()
        .chars()
        .map(|letter| match letter.to_ascii_uppercase() {
            'R' => Some(0),
            'G' => Some(1),
            'B' => Some(2),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let rows: Vec<[u8; XTRANS_SIDE]> = colors
        .chunks(XTRANS_SIDE)
        .map(|row| row.try_into().ok())
        .collect::<Option<_>>()?;
    rows.try_into().ok()
}

fn shifted_xtrans(
    matrix: [[u8; XTRANS_SIDE]; XTRANS_SIDE],
    x_offset: u8,
    y_offset: u8,
) -> [[u8; XTRANS_SIDE]; XTRANS_SIDE] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            matrix[(row + usize::from(y_offset)) % XTRANS_SIDE]
                [(column + usize::from(x_offset)) % XTRANS_SIDE]
        })
    })
}

/// The layout the header states. A plain single plane carries no statement: it may be mono or
/// the data of a camera whose pattern the file does not describe.
pub fn explicit_layout_of(header: &Header, planes: usize) -> Option<PixelLayout> {
    if let Some(pattern) = crate::fits::stary_cfa::layout_from_header(header) {
        return Some(PixelLayout::Cfa(pattern));
    }
    if planes != 1 {
        return Some(PixelLayout::Rgb);
    }
    let keyword = header.text("BAYERPAT")?;
    if let Some(matrix) = xtrans_from_keyword(keyword) {
        return Some(PixelLayout::Cfa(CfaPattern::XTrans(shifted_xtrans(
            matrix,
            offset(header, "XBAYROFF", 6),
            offset(header, "YBAYROFF", 6),
        ))));
    }
    BayerPattern::from_keyword(keyword)
        .and_then(|pattern| {
            shifted_pattern(
                pattern,
                offset(header, "XBAYROFF", 2),
                offset(header, "YBAYROFF", 2),
            )
        })
        .map(|pattern| PixelLayout::Cfa(CfaPattern::Bayer(pattern)))
}

pub fn layout_of(header: &Header, planes: usize) -> PixelLayout {
    explicit_layout_of(header, planes).unwrap_or(PixelLayout::Mono)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_rotate_the_pattern() {
        assert_eq!(
            shifted_pattern(BayerPattern::Rggb, 0, 0),
            Some(BayerPattern::Rggb)
        );
        assert_eq!(
            shifted_pattern(BayerPattern::Rggb, 1, 0),
            Some(BayerPattern::Grbg)
        );
        assert_eq!(
            shifted_pattern(BayerPattern::Rggb, 0, 1),
            Some(BayerPattern::Gbrg)
        );
        assert_eq!(
            shifted_pattern(BayerPattern::Rggb, 1, 1),
            Some(BayerPattern::Bggr)
        );
    }

    const SIRIL_XTRANS: &str = "GGRGGBGGBGGRBRGRBGGGBGGRGGRGGBRBGBRG";

    #[test]
    fn an_xtrans_pattern_is_read_from_its_thirty_six_letters() {
        let matrix = xtrans_from_keyword(SIRIL_XTRANS).unwrap();
        assert_eq!(matrix[0], [1, 1, 0, 1, 1, 2]);
        assert_eq!(matrix[2], [2, 0, 1, 0, 2, 1]);
        assert_eq!(matrix[5], [0, 2, 1, 2, 0, 1]);
    }

    #[test]
    fn other_lengths_and_letters_are_not_a_pattern() {
        assert_eq!(xtrans_from_keyword("RGGB"), None);
        assert_eq!(xtrans_from_keyword(&"X".repeat(36)), None);
    }

    #[test]
    fn offsets_move_the_origin_of_an_xtrans_pattern() {
        let matrix = xtrans_from_keyword(SIRIL_XTRANS).unwrap();
        let shifted = shifted_xtrans(matrix, 1, 2);
        assert_eq!(shifted[0][0], matrix[2][1]);
        assert_eq!(shifted[5][5], matrix[1][0]);
        assert_eq!(shifted_xtrans(matrix, 6, 6), matrix);
    }
}
