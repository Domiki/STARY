use std::{path::PathBuf, sync::Mutex, time::Instant};

use anyhow::Context;
use clap::ValueEnum;
use stary_io::{
    master_file::write_calibrated_light,
    rgb_out::{OutputFormat, write_result},
};
use stary_pipeline::{
    Cancel, ComputeDevice, Device, MemoryBudget, Reference, StackOptions, Stage,
    prepare_calibration, run_stacking,
};
use stary_raw::DemosaicQuality;
use stary_register::{DetectParams, Model};
use stary_stack::{Kernel, Rejection};

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
enum Alignment {
    Similarity,
    Affine,
    Homography,
}

#[derive(Clone, Copy, ValueEnum)]
enum Resampler {
    Bilinear,
    Bicubic,
    Lanczos3,
}

#[derive(Clone, Copy, ValueEnum)]
enum Combine {
    Average,
    Sigma,
    Winsorized,
}

#[derive(Clone, Copy, ValueEnum)]
enum DeviceChoice {
    Auto,
    Cpu,
    Gpu,
}

#[derive(Clone, Copy, ValueEnum)]
enum ReferenceChoice {
    First,
    Best,
}

#[derive(clap::Args)]
pub struct StackArgs {
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
    #[arg(long, value_enum, default_value = "similarity")]
    model: Alignment,
    #[arg(long, value_enum, default_value = "bicubic")]
    resampler: Resampler,
    #[arg(long, value_enum, default_value = "sigma")]
    combine: Combine,
    #[arg(long, default_value_t = 3.0)]
    low: f32,
    #[arg(long, default_value_t = 3.0)]
    high: f32,
    /// What lays the frames on the picture: the GPU when it can hold the picture (auto), or the
    /// processor
    #[arg(long, value_enum, default_value = "auto")]
    device: DeviceChoice,
    #[arg(long, value_enum, default_value = "first")]
    reference: ReferenceChoice,
    /// Use the light at this position (0 is the first) as the reference, instead of `--reference`
    #[arg(long)]
    reference_index: Option<usize>,
    /// Do not match the sky level and noise of the frames to the reference
    #[arg(long)]
    no_normalize: bool,
    /// Detection threshold for stars, in units of noise
    #[arg(long, default_value_t = 6.0)]
    star_threshold: f32,
    /// Use every frame, even those that are much worse than the rest (cloudy, blurry, trailed)
    #[arg(long)]
    keep_poor_frames: bool,
    /// Print what was found for every frame
    #[arg(long)]
    verbose: bool,
}

fn light_file_name(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("light");
    format!("{stem}.fits")
}

fn options(args: &StackArgs, budget: MemoryBudget) -> StackOptions {
    let mut options = StackOptions::new(budget);
    options.quality = match args.quality {
        Quality::Fast => DemosaicQuality::Fast,
        Quality::High => DemosaicQuality::High,
    };
    options.model = match args.model {
        Alignment::Similarity => Model::Similarity,
        Alignment::Affine => Model::Affine,
        Alignment::Homography => Model::Homography,
    };
    options.kernel = match args.resampler {
        Resampler::Bilinear => Kernel::Bilinear,
        Resampler::Bicubic => Kernel::Bicubic,
        Resampler::Lanczos3 => Kernel::Lanczos3,
    };
    options.rejection = match args.combine {
        Combine::Average => Rejection::None,
        Combine::Sigma => Rejection::Sigma {
            low: args.low,
            high: args.high,
        },
        Combine::Winsorized => Rejection::Winsorized {
            low: args.low,
            high: args.high,
        },
    };
    options.device = match args.device {
        DeviceChoice::Auto => Device::Auto,
        DeviceChoice::Cpu => Device::Cpu,
        DeviceChoice::Gpu => Device::Gpu,
    };
    options.reference = match (args.reference_index, args.reference) {
        (Some(index), _) => Reference::Index(index),
        (None, ReferenceChoice::First) => Reference::Index(0),
        (None, ReferenceChoice::Best) => Reference::Best,
    };
    options.normalize = !args.no_normalize;
    options.skip_poor_frames = !args.keep_poor_frames;
    options.detect = DetectParams {
        threshold_sigma: args.star_threshold,
        ..DetectParams::default()
    };
    options
}

pub fn run(args: &StackArgs) -> anyhow::Result<()> {
    let calibration = &args.calibration;
    let lights = calibration.lights();
    anyhow::ensure!(!lights.is_empty(), "no light frames given (--light)");
    let run_options = calibration.run_options();
    let cancel = Cancel::default();
    let seen = Mutex::new((Stage::Lights, usize::MAX));
    let progress = progress_printer(&seen);
    let started = Instant::now();

    let prepared = prepare_calibration(&calibration.inputs()?, &run_options, &cancel, &progress)?;
    println!(
        "calibration ready in {:.2} s",
        started.elapsed().as_secs_f64()
    );
    calibration.report_reads(&prepared);
    calibration.save_built_masters(&prepared)?;
    if let Some(dir) = &calibration.save_calibrated {
        std::fs::create_dir_all(dir).context("cannot create the calibrated folder")?;
    }

    let budget = MemoryBudget::detect(Some(calibration.memory_percent));
    let mut options = options(args, budget);
    options.masking = args.masking.masking()?;
    let stacking_started = Instant::now();
    let result = run_stacking(
        &lights,
        prepared.set.as_ref(),
        &options,
        &run_options,
        &|index, frame| {
            if let Some(dir) = &calibration.save_calibrated {
                write_calibrated_light(&dir.join(light_file_name(&lights[index])), frame)?;
            }
            Ok(())
        },
        &cancel,
        &progress,
    )?;
    println!(
        "{} of {} lights stacked in {:.2} s (reference: {}, device: {})",
        result.frames,
        lights.len(),
        stacking_started.elapsed().as_secs_f64(),
        lights[result.reference].display(),
        match &result.device {
            ComputeDevice::Cpu => "CPU".to_string(),
            ComputeDevice::Gpu(info) => format!("GPU {info}"),
        }
    );
    for skipped in &result.skipped {
        println!(
            "  skipped {}: {}",
            lights[skipped.index].display(),
            skipped.reason
        );
    }
    if args.verbose {
        for (index, report) in result.reports.iter().enumerate() {
            if let Some(poor) = report.poor {
                println!("  {index:>3} left out: {poor}");
            }
            match &report.alignment {
                Ok(alignment) => {
                    let m = &alignment.transform.m;
                    println!(
                        "  {index:>3} {:>4} stars, fwhm {:>6.2}, round {:.3}, sky {:>8.1}, noise {:>6.2}, {:>4} matched, rms {:.3} px, shift {:+.2},{:+.2}, rotation {:+.4} deg, H {} {} {} {} {} {} {} {} {}",
                        report.quality.stars,
                        report.quality.fwhm,
                        report.quality.roundness,
                        report.quality.background,
                        report.quality.noise,
                        alignment.inliers,
                        alignment.rms,
                        m[0][2],
                        m[1][2],
                        alignment.transform.rotation().to_degrees(),
                        m[0][0],
                        m[0][1],
                        m[0][2],
                        m[1][0],
                        m[1][1],
                        m[1][2],
                        m[2][0],
                        m[2][1],
                        m[2][2]
                    );
                }
                Err(error) => println!("  {index:>3} {:>4} stars, {error}", report.quality.stars),
            }
        }
    }

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
        &result.image,
        format,
        "Stacking",
        result.frames,
        &result.color,
    )?;
    println!(
        "saved {} ({:.2} s total)",
        path.display(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
