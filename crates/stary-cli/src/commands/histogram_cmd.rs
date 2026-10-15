use std::path::Path;

use stary_core::histogram::histogram_of_rgb8;

fn peak_bin(bins: &[u32]) -> usize {
    bins.iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map_or(0, |(bin, _)| bin)
}

pub fn run(path: &Path, max_edge: u32) -> anyhow::Result<()> {
    let preview = stary_io::read_preview(path, max_edge)?;
    let histogram = histogram_of_rgb8(&preview.rgb8);
    println!(
        "peak bin  R:{} G:{} B:{}",
        peak_bin(&histogram.red),
        peak_bin(&histogram.green),
        peak_bin(&histogram.blue)
    );
    println!(
        "clipped   low:{:.3}% high:{:.3}%",
        histogram.clipped_low * 100.0,
        histogram.clipped_high * 100.0
    );
    Ok(())
}
