const FULL_SCALE: f32 = 65535.0;

/// Float data in 0..1 is taken to be normalised and spread over the 16-bit range; anything above
/// is taken to be in counts already.
pub fn float_to_u16(samples: &[f32]) -> Vec<u16> {
    let highest = samples
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(0.0, f32::max);
    let scale = if highest <= 1.0 { FULL_SCALE } else { 1.0 };
    samples
        .iter()
        .map(|value| (value * scale).round().clamp(0.0, FULL_SCALE) as u16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalised_floats_use_the_whole_range() {
        assert_eq!(float_to_u16(&[0.0, 0.5, 1.0]), vec![0, 32768, 65535]);
    }

    #[test]
    fn floats_in_counts_are_kept_and_clamped() {
        assert_eq!(
            float_to_u16(&[0.0, 1200.4, 70000.0, -5.0]),
            vec![0, 1200, 65535, 0]
        );
    }

    #[test]
    fn not_a_number_becomes_zero() {
        assert_eq!(float_to_u16(&[f32::NAN, 0.25]), vec![0, 16384]);
    }
}
