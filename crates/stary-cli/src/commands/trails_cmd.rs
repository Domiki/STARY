use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};

use anyhow::Context;
use clap::ValueEnum;
use stary_core::ResultColor;
use stary_io::{
    master_file::write_calibrated_light,
    rgb_out::{OutputFormat, write_result},
};
use stary_pipeline::{
    Cancel, Stage, TrailsMethod, TrailsSettings, prepare_calibration, run_trails,
};
use stary_raw::DemosaicQuality;

use super::{
    calibration_args::{CalibrationArgs, progress_printer},
    mask_args::MaskArgs,
};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Tiff32,
    Tiff16,
    Fits32,
}

#[derive(Clone, Copy, ValueEnum)]
enum Quality {
    Fast,
    High,
}

#[derive(Clone, Copy, ValueEnum)]
enum Method {
    Lighten,
    Comet,
    GapFilling,
}

#[derive(clap::Args)]
pub struct TrailsArgs {
    #[command(flatten)]
    calibration: CalibrationArgs,
    #[command(flatten)]
    masking: MaskArgs,
    /// Result file (the extension is added when missing)
    #[arg(long)]
    out: PathBuf,
    #[arg(long, value_enum, default_value = "tiff32")]
    format: Format,
    #[arg(long, value_enum, default_value = "fast")]
    quality: Quality,
    #[arg(long, value_enum, default_value = "lighten")]
    method: Method,
    /// For a comet: how many frames the tail takes to fade
    #[arg(long, default_value_t = 10)]
    tail_frames: u32,
    /// For a comet: read the lights from the last one to the first
    #[arg(long)]
    reverse: bool,
    /// Gap filling: lights merged into one picture before it is filled (the stars are taken to
    /// move the same way within one)
    #[arg(long, default_value_t = 100)]
    stretch_frames: usize,
    /// Gap filling: the most stretches the lights are cut into
    #[arg(long, default_value_t = 16)]
    max_stretches: usize,
    /// Gap filling: how far apart, in pixels, the images of a star are laid
    #[arg(long, default_value_t = 0.8)]
    spacing: f64,
}

fn light_file_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("light");
    format!("{stem}.fits")
}

pub fn run(args: &TrailsArgs) -> anyhow::Result<()> {
    let calibration = &args.calibration;
    let lights = calibration.lights();
    anyhow::ensure!(!lights.is_empty(), "no light frames given (--light)");
    let options = calibration.run_options();
    let cancel = Cancel::default();
    let seen = Mutex::new((Stage::Lights, usize::MAX));
    let progress = progress_printer(&seen);
    let started = Instant::now();

    let prepared = prepare_calibration(&calibration.inputs()?, &options, &cancel, &progress)?;
    println!(
        "calibration ready in {:.2} s",
        started.elapsed().as_secs_f64()
    );
    calibration.report_reads(&prepared);
    calibration.save_built_masters(&prepared)?;
    if let Some(dir) = &calibration.save_calibrated {
        std::fs::create_dir_all(dir).context("cannot create the calibrated folder")?;
    }

    let quality = match args.quality {
        Quality::Fast => DemosaicQuality::Fast,
        Quality::High => DemosaicQuality::High,
    };
    let lights_started = Instant::now();
    let method = match args.method {
        Method::Lighten => TrailsMethod::Lighten,
        Method::Comet => TrailsMethod::Comet {
            tail_frames: args.tail_frames,
            reverse: args.reverse,
        },
        Method::GapFilling => TrailsMethod::GapFilling,
    };
    let masking = args.masking.masking()?;
    let save_calibrated = |index: usize, frame: &stary_core::Frame| {
        if let Some(dir) = &calibration.save_calibrated {
            write_calibrated_light(&dir.join(light_file_name(&lights[index])), frame)?;
        }
        Ok(())
    };
    let mut settings = TrailsSettings::new(quality, method);
    settings.masking = masking;
    settings.gap_fill.stretch_frames = args.stretch_frames;
    settings.gap_fill.max_stretches = args.max_stretches;
    settings.gap_fill.spacing = args.spacing;
    let result = run_trails(
        &lights,
        prepared.set.as_ref(),
        settings,
        &options,
        &cancel,
        &progress,
        &save_calibrated,
    )?;
    if let Some(gaps) = result.gaps {
        println!("{} gaps filled, {} could not be", gaps.filled, gaps.skipped);
    }
    let (image, frames) = (result.image, result.frames);
    println!(
        "{frames} lights merged in {:.2} s",
        lights_started.elapsed().as_secs_f64()
    );

    let format = match args.format {
        Format::Tiff32 => OutputFormat::Tiff32,
        Format::Tiff16 => OutputFormat::Tiff16,
        Format::Fits32 => OutputFormat::Fits32,
    };
    let path = args.out.with_extension(format.extension());
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).context("cannot create the result folder")?;
    }
    write_result(
        &path,
        &image,
        format,
        "Trails",
        frames,
        &ResultColor::LinearSrgb,
    )?;
    println!(
        "saved {} ({:.2} s total)",
        path.display(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
