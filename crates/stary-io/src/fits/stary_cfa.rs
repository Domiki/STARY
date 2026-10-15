use stary_core::{BayerPattern, CfaPattern};

use crate::fits::Header;

pub const KEY_CFA: &str = "STRYCFA";
pub const KEY_MATRIX: &str = "STRYMTX";

const BAYER_PREFIX: &str = "BAYER:";
const XTRANS: &str = "XTRANS";

fn bayer_keyword(pattern: BayerPattern) -> &'static str {
    match pattern {
        BayerPattern::Rggb => "RGGB",
        BayerPattern::Bggr => "BGGR",
        BayerPattern::Grbg => "GRBG",
        BayerPattern::Gbrg => "GBRG",
    }
}

/// The keyword/value pairs that describe a sensor color layout inside a FITS header.
pub fn layout_cards(layout: &CfaPattern) -> Vec<(&'static str, String)> {
    match layout {
        CfaPattern::Bayer(pattern) => {
            vec![(
                KEY_CFA,
                format!("{BAYER_PREFIX}{}", bayer_keyword(*pattern)),
            )]
        }
        CfaPattern::XTrans(matrix) => vec![
            (KEY_CFA, XTRANS.to_string()),
            (
                KEY_MATRIX,
                matrix
                    .iter()
                    .flatten()
                    .map(|color| char::from(b'0' + color))
                    .collect(),
            ),
        ],
    }
}

fn parse_matrix(text: &str) -> Option<[[u8; 6]; 6]> {
    let digits: Vec<u8> = text
        .chars()
        .map(|character| character.to_digit(3).map(|digit| digit as u8))
        .collect::<Option<_>>()?;
    let rows: Vec<[u8; 6]> = digits
        .chunks(6)
        .map(|row| row.try_into().ok())
        .collect::<Option<_>>()?;
    rows.try_into().ok()
}

pub fn layout_from_header(header: &Header) -> Option<CfaPattern> {
    let text = header.text(KEY_CFA)?;
    if text == XTRANS {
        return header
            .text(KEY_MATRIX)
            .and_then(parse_matrix)
            .map(CfaPattern::XTrans);
    }
    text.strip_prefix(BAYER_PREFIX)
        .and_then(BayerPattern::from_keyword)
        .map(CfaPattern::Bayer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_round_trips_through_its_keyword_text() {
        let mut matrix = [[1u8; 6]; 6];
        matrix[0][2] = 0;
        matrix[1][5] = 2;
        let cards = layout_cards(&CfaPattern::XTrans(matrix));
        let text: String = cards[1].1.clone();
        assert_eq!(text.len(), 36);
        assert_eq!(parse_matrix(&text), Some(matrix));
    }

    #[test]
    fn rejects_malformed_matrices() {
        assert_eq!(parse_matrix("012"), None);
        assert_eq!(parse_matrix(&"3".repeat(36)), None);
    }
}
