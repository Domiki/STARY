const PATTERN_START: usize = 6;

fn mean(values: &[u32]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().map(|value| f64::from(*value)).sum::<f64>() / values.len() as f64
    }
}

// LibRaw keeps the black level in three places: a common `black`, four per-color offsets in
// `cblack[0..4]`, and for some sensors (X-Trans) a repeating pattern in `cblack[6..]` whose size
// is given by `cblack[4] * cblack[5]`.
pub fn effective_black(black: u32, cblack: &[u32]) -> u16 {
    let pattern_len = cblack
        .get(4)
        .zip(cblack.get(5))
        .map_or(0, |(rows, columns)| (*rows as usize) * (*columns as usize));
    let offset = if pattern_len > 0 {
        mean(
            cblack
                .get(PATTERN_START..PATTERN_START + pattern_len)
                .unwrap_or(&[]),
        )
    } else {
        mean(cblack.get(..4).unwrap_or(&[]))
    };
    (f64::from(black) + offset).round().min(f64::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cblack(head: [u32; 6], pattern: &[u32]) -> Vec<u32> {
        head.into_iter().chain(pattern.iter().copied()).collect()
    }

    #[test]
    fn uses_the_repeating_pattern_when_present() {
        let values = cblack([0, 0, 0, 0, 6, 6], &[1024; 36]);
        assert_eq!(effective_black(0, &values), 1024);
    }

    #[test]
    fn averages_a_pattern_with_different_values() {
        let values = cblack([0, 0, 0, 0, 2, 2], &[100, 102, 98, 100]);
        assert_eq!(effective_black(0, &values), 100);
    }

    #[test]
    fn falls_back_to_the_per_color_offsets() {
        let values = cblack([510, 512, 510, 512, 0, 0], &[]);
        assert_eq!(effective_black(0, &values), 511);
    }

    #[test]
    fn adds_the_common_black_level() {
        let values = cblack([0, 0, 0, 0, 0, 0], &[]);
        assert_eq!(effective_black(512, &values), 512);
    }

    #[test]
    fn survives_a_truncated_array() {
        assert_eq!(effective_black(10, &[]), 10);
    }
}
