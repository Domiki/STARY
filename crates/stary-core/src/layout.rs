use crate::{cfa::CfaPattern, meta::PixelLayout};

pub const COLORS: usize = 3;

impl CfaPattern {
    pub fn color_at(&self, row: usize, column: usize) -> usize {
        match self {
            Self::Bayer(pattern) => usize::from(pattern.colors()[(row % 2) * 2 + column % 2]),
            Self::XTrans(matrix) => usize::from(matrix[row % 6][column % 6]),
        }
        .min(COLORS - 1)
    }
}

impl PixelLayout {
    pub const fn channels(&self) -> usize {
        match self {
            Self::Rgb => COLORS,
            Self::Cfa(_) | Self::Mono => 1,
        }
    }

    /// The color class (0 red, 1 green, 2 blue) of a sample. Samples of an RGB frame are
    /// interleaved, a mono frame counts as one class.
    pub fn color_at(&self, row: usize, column: usize, channel: usize) -> usize {
        match self {
            Self::Cfa(pattern) => pattern.color_at(row, column),
            Self::Rgb => channel.min(COLORS - 1),
            Self::Mono => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfa::BayerPattern;

    #[test]
    fn bayer_rggb_has_two_greens_per_block() {
        let layout = CfaPattern::Bayer(BayerPattern::Rggb);
        assert_eq!(layout.color_at(0, 0), 0);
        assert_eq!(layout.color_at(0, 1), 1);
        assert_eq!(layout.color_at(1, 0), 1);
        assert_eq!(layout.color_at(1, 1), 2);
        assert_eq!(layout.color_at(2, 2), 0);
    }

    #[test]
    fn xtrans_repeats_every_six() {
        let mut matrix = [[1u8; 6]; 6];
        matrix[2][3] = 2;
        let layout = CfaPattern::XTrans(matrix);
        assert_eq!(layout.color_at(2, 3), 2);
        assert_eq!(layout.color_at(8, 9), 2);
        assert_eq!(layout.color_at(0, 0), 1);
    }

    #[test]
    fn rgb_samples_follow_their_channel_and_mono_is_one_class() {
        assert_eq!(PixelLayout::Rgb.color_at(5, 7, 2), 2);
        assert_eq!(PixelLayout::Mono.color_at(5, 7, 0), 0);
        assert_eq!(PixelLayout::Rgb.channels(), 3);
        assert_eq!(PixelLayout::Mono.channels(), 1);
    }
}
