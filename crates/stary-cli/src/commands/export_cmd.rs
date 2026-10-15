use std::{path::PathBuf, time::Instant};

use anyhow::anyhow;
use clap::ValueEnum;
use stary_color::Space;
use stary_io::export::{Container, Depth, Export, Tone, export_result};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Tiff,
    Png,
    Jpeg,
    Fits,
}

#[derive(Clone, Copy, ValueEnum)]
enum Colors {
    /// The colors the result has
    Keep,
    Srgb,
    P3,
    Adobe,
}

#[derive(Clone, Copy, ValueEnum)]
enum Brightness {
    Linear,
    Gamma,
    Stretch,
}

#[derive(clap::Args)]
pub struct ExportArgs {
    /// A result the program saved (a TIFF of 32-bit floats)
    source: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, value_enum, default_value = "tiff")]
    format: Format,
    /// Bits of a value: 8, 16, or 32 (floating point)
    #[arg(long, default_value_t = 16, value_parser = clap::value_parser!(u8).range(8..=32))]
    bits: u8,
    #[arg(long, value_enum, default_value = "srgb")]
    colors: Colors,
    #[arg(long, value_enum, default_value = "linear")]
    tone: Brightness,
    /// For JPEG
    #[arg(long, default_value_t = 92)]
    quality: u8,
}

pub fn run(args: &ExportArgs) -> anyhow::Result<()> {
    let export = Export {
        container: match args.format {
            Format::Tiff => Container::Tiff,
            Format::Png => Container::Png,
            Format::Jpeg => Container::Jpeg,
            Format::Fits => Container::Fits,
        },
        depth: match args.bits {
            8 => Depth::Eight,
            16 => Depth::Sixteen,
            32 => Depth::Float,
            _ => return Err(anyhow!("bits are 8, 16 or 32")),
        },
        space: match args.colors {
            Colors::Keep => None,
            Colors::Srgb => Some(Space::Srgb),
            Colors::P3 => Some(Space::DisplayP3),
            Colors::Adobe => Some(Space::AdobeRgb),
        },
        tone: match args.tone {
            Brightness::Linear => Tone::Linear,
            Brightness::Gamma => Tone::Display,
            Brightness::Stretch => Tone::Stretch,
        },
        quality: args.quality,
    };
    let started = Instant::now();
    export_result(&args.source, &args.out, &export)?;
    println!(
        "saved {} in {:.2} s{}",
        args.out.display(),
        started.elapsed().as_secs_f64(),
        if export.loses_detail() {
            " (holds less than the result)"
        } else {
            ""
        }
    );
    Ok(())
}
