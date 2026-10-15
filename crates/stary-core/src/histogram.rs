use serde::{Deserialize, Serialize};

pub const BIN_COUNT: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Histogram {
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    pub clipped_low: f32,
    pub clipped_high: f32,
}

type Bins = [[u32; BIN_COUNT]; 3];

fn accumulate(mut bins: Bins, pixel: &[u8]) -> Bins {
    pixel
        .iter()
        .enumerate()
        .for_each(|(channel, &value)| bins[channel][value as usize] += 1);
    bins
}

fn fraction(count: u32, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

pub fn histogram_of_rgb8(pixels: &[u8]) -> Histogram {
    let (triples, _) = pixels.as_chunks::<3>();
    let bins = triples.iter().fold([[0u32; BIN_COUNT]; 3], |bins, pixel| {
        accumulate(bins, pixel)
    });
    let samples = triples.len() as u64 * 3;
    let count_at = |bin: usize| bins.iter().map(|channel| channel[bin]).sum::<u32>();
    Histogram {
        red: bins[0].to_vec(),
        green: bins[1].to_vec(),
        blue: bins[2].to_vec(),
        clipped_low: fraction(count_at(0), samples),
        clipped_high: fraction(count_at(BIN_COUNT - 1), samples),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_channel_and_clipping() {
        let pixels = [0, 10, 255, 0, 10, 255];
        let histogram = histogram_of_rgb8(&pixels);
        assert_eq!(histogram.red[0], 2);
        assert_eq!(histogram.green[10], 2);
        assert_eq!(histogram.blue[255], 2);
        assert!((histogram.clipped_low - 1.0 / 3.0).abs() < 1e-6);
        assert!((histogram.clipped_high - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn empty_input_has_no_clipping() {
        let histogram = histogram_of_rgb8(&[]);
        assert_eq!(histogram.clipped_low, 0.0);
        assert_eq!(histogram.clipped_high, 0.0);
    }
}
