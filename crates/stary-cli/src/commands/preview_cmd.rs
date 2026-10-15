use std::path::Path;

use anyhow::{Context, anyhow};
use image::RgbImage;

pub fn run(path: &Path, out: &Path, max_edge: u32) -> anyhow::Result<()> {
    let preview = stary_io::read_preview(path, max_edge)?;
    let image = RgbImage::from_raw(preview.width, preview.height, preview.rgb8)
        .ok_or_else(|| anyhow!("preview buffer has unexpected size"))?;
    image
        .save(out)
        .with_context(|| format!("cannot write {}", out.display()))?;
    println!("{}x{} -> {}", preview.width, preview.height, out.display());
    Ok(())
}
