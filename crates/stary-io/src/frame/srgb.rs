use std::sync::OnceLock;

const FULL_SCALE: f32 = 65535.0;
const TABLE_LEN: usize = 65536;

fn linear_of(encoded: f32) -> f32 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn table() -> &'static [u16; TABLE_LEN] {
    static TABLE: OnceLock<Box<[u16; TABLE_LEN]>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Box::new([0u16; TABLE_LEN]);
        for (code, slot) in table.iter_mut().enumerate() {
            *slot = (linear_of(code as f32 / FULL_SCALE) * FULL_SCALE).round() as u16;
        }
        table
    })
}

/// Turns sRGB-encoded 16-bit codes into linear 16-bit values in place.
pub fn linearize(samples: &mut [u16]) {
    let table = table();
    for sample in samples {
        *sample = table[usize::from(*sample)];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ends_stay_and_the_middle_gets_darker() {
        let mut samples = [0u16, 65535, 32768];
        linearize(&mut samples);
        assert_eq!(samples[0], 0);
        assert_eq!(samples[1], 65535);
        // sRGB 50% is about 21.4% linear
        assert!((i32::from(samples[2]) - 14025).abs() < 30);
    }

    #[test]
    fn the_table_never_decreases() {
        let table = table();
        assert!(table.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
