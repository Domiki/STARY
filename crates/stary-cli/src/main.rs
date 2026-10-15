mod commands;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stary", about = "STARY headless tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read metadata from files without decoding pixels
    Probe {
        paths: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Write a downscaled preview as PNG
    Preview {
        path: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 1600)]
        max_edge: u32,
    },
    /// Build masters and calibrate lights
    Calibrate(Box<commands::calibrate_cmd::CalibrateArgs>),
    /// Star trails (Lighten) from calibrated lights
    Trails(Box<commands::trails_cmd::TrailsArgs>),
    /// Align and stack calibrated lights with outlier rejection
    Stack(Box<commands::stack_cmd::StackArgs>),
    /// Save a result as another kind of file (format, bits, color space)
    Export(Box<commands::export_cmd::ExportArgs>),
    /// Find the sky in a photograph and write it as a mask
    Sky(Box<commands::sky_cmd::SkyArgs>),
    /// Print the histogram summary of a preview
    Histogram {
        path: PathBuf,
        #[arg(long, default_value_t = 1600)]
        max_edge: u32,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Probe { paths, json } => commands::probe_cmd::run(&paths, json),
        Command::Preview {
            path,
            out,
            max_edge,
        } => commands::preview_cmd::run(&path, &out, max_edge),
        Command::Calibrate(args) => commands::calibrate_cmd::run(&args),
        Command::Trails(args) => commands::trails_cmd::run(&args),
        Command::Stack(args) => commands::stack_cmd::run(&args),
        Command::Export(args) => commands::export_cmd::run(&args),
        Command::Sky(args) => commands::sky_cmd::run(&args),
        Command::Histogram { path, max_edge } => commands::histogram_cmd::run(&path, max_edge),
    }
}
