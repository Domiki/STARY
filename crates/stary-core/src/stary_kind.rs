use serde::{Deserialize, Serialize};

pub mod keyword {
    pub const KIND: &str = "STRYTYPE";
    pub const STACK_COUNT: &str = "STACKCNT";
    pub const CALIBRATED: &str = "STRYCAL";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaryKind {
    MasterBias,
    MasterDark,
    MasterFlat,
    MasterDarkFlat,
    StackResult,
}

impl StaryKind {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "MASTER_BIAS" => Some(Self::MasterBias),
            "MASTER_DARK" => Some(Self::MasterDark),
            "MASTER_FLAT" => Some(Self::MasterFlat),
            "MASTER_DARKFLAT" => Some(Self::MasterDarkFlat),
            "STACK_RESULT" => Some(Self::StackResult),
            _ => None,
        }
    }

    pub const fn keyword_value(self) -> &'static str {
        match self {
            Self::MasterBias => "MASTER_BIAS",
            Self::MasterDark => "MASTER_DARK",
            Self::MasterFlat => "MASTER_FLAT",
            Self::MasterDarkFlat => "MASTER_DARKFLAT",
            Self::StackResult => "STACK_RESULT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_value_round_trips() {
        let kinds = [
            StaryKind::MasterBias,
            StaryKind::MasterDark,
            StaryKind::MasterFlat,
            StaryKind::MasterDarkFlat,
            StaryKind::StackResult,
        ];
        for kind in kinds {
            assert_eq!(StaryKind::parse(kind.keyword_value()), Some(kind));
        }
    }
}
