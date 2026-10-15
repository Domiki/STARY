use std::path::PathBuf;

use anyhow::Context;
use stary_io::mask_file::read_mask;
use stary_pipeline::{Ground, Masking};

#[derive(clap::Args)]
pub struct MaskArgs {
    /// A gray image of the sky, of any size: white is sky, black is ground
    #[arg(long)]
    sky_mask: Option<PathBuf>,
    /// Combine the ground on its own, with the frames laid on each other without being moved
    #[arg(long, requires = "sky_mask")]
    separate_ground: bool,
}

impl MaskArgs {
    pub fn masking(&self) -> anyhow::Result<Option<Masking>> {
        let Some(path) = &self.sky_mask else {
            return Ok(None);
        };
        let mask = read_mask(path).with_context(|| format!("cannot read {}", path.display()))?;
        Ok(Some(Masking {
            mask: mask.into(),
            ground: if self.separate_ground {
                Ground::Separate
            } else {
                Ground::WithSky
            },
        }))
    }
}
