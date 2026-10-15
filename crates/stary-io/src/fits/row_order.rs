use crate::fits::{Header, stary_cfa};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowOrder {
    TopDown,
    BottomUp,
}

impl RowOrder {
    pub const fn keyword_value(self) -> &'static str {
        match self {
            Self::TopDown => "TOP-DOWN",
            Self::BottomUp => "BOTTOM-UP",
        }
    }
}

/// The order of the rows in the file. The FITS standard stores the bottom row first; files written
/// by STARY for sensor data say `TOP-DOWN` and files that describe their layout with STARY's own
/// keywords are top-down even without the keyword.
pub fn row_order_of(header: &Header) -> RowOrder {
    match header
        .text("ROWORDER")
        .map(str::to_ascii_uppercase)
        .as_deref()
    {
        Some("TOP-DOWN") => RowOrder::TopDown,
        Some("BOTTOM-UP") => RowOrder::BottomUp,
        _ if header.text(stary_cfa::KEY_CFA).is_some() => RowOrder::TopDown,
        _ => RowOrder::BottomUp,
    }
}

pub fn reverse_rows<T: Copy>(samples: &mut [T], row_len: usize) {
    if row_len == 0 {
        return;
    }
    let rows = samples.len() / row_len;
    for row in 0..rows / 2 {
        let (top, bottom) = samples.split_at_mut((rows - 1 - row) * row_len);
        top[row * row_len..(row + 1) * row_len].swap_with_slice(&mut bottom[..row_len]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversing_rows_swaps_whole_rows() {
        let mut samples = vec![1, 2, 3, 4, 5, 6];
        reverse_rows(&mut samples, 2);
        assert_eq!(samples, vec![5, 6, 3, 4, 1, 2]);
        let mut odd = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        reverse_rows(&mut odd, 3);
        assert_eq!(odd, vec![7, 8, 9, 4, 5, 6, 1, 2, 3]);
    }
}
