use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BayerPattern {
    Rggb,
    Bggr,
    Grbg,
    Gbrg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CfaPattern {
    Bayer(BayerPattern),
    XTrans([[u8; 6]; 6]),
}

const RED: u8 = 0;
const GREEN: u8 = 1;
const BLUE: u8 = 2;

impl BayerPattern {
    pub fn from_keyword(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "RGGB" => Some(Self::Rggb),
            "BGGR" => Some(Self::Bggr),
            "GRBG" => Some(Self::Grbg),
            "GBRG" => Some(Self::Gbrg),
            _ => None,
        }
    }

    pub fn from_colors(colors: [u8; 4]) -> Option<Self> {
        match colors {
            [RED, GREEN, GREEN, BLUE] => Some(Self::Rggb),
            [BLUE, GREEN, GREEN, RED] => Some(Self::Bggr),
            [GREEN, RED, BLUE, GREEN] => Some(Self::Grbg),
            [GREEN, BLUE, RED, GREEN] => Some(Self::Gbrg),
            _ => None,
        }
    }

    pub const fn colors(self) -> [u8; 4] {
        match self {
            Self::Rggb => [RED, GREEN, GREEN, BLUE],
            Self::Bggr => [BLUE, GREEN, GREEN, RED],
            Self::Grbg => [GREEN, RED, BLUE, GREEN],
            Self::Gbrg => [GREEN, BLUE, RED, GREEN],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_round_trips_through_colors() {
        for text in ["RGGB", "BGGR", "GRBG", "GBRG"] {
            let pattern = BayerPattern::from_keyword(text).unwrap();
            assert_eq!(BayerPattern::from_colors(pattern.colors()), Some(pattern));
        }
    }

    #[test]
    fn unknown_keyword_is_rejected() {
        assert_eq!(BayerPattern::from_keyword("XYZW"), None);
    }
}
