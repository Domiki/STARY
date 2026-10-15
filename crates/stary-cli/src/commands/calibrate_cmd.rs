use std::{path::Path, sync::Mutex, time::Instant};

use anyhow::Context;
use stary_io::master_file::write_calibrated_light;
use stary_pipeline::{Cancel, Stage, calibrate_lights, prepare_calibration};

use super::calibration_args::{CalibrationArgs, progress_printer};

#[derive(clap::Args)]
pub struct CalibrateArgs {
    #[command(flatten)]
    calibration: CalibrationArgs,
}

fn light_file_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("light");
    format!("{stem}.fits")
}

pub fn run(args: &CalibrateArgs) -> anyhow::Result<()> {
    let calibration = &args.calibration;
    let lights = calibration.lights();
    let options = calibration.run_options();
    let cancel = Cancel::default();
    let seen = Mutex::new((Stage::Lights, usize::MAX));
    let progress = progress_printer(&seen);

    let started = Instant::now();
    let prepared = prepare_calibration(&calibration.inputs()?, &options, &cancel, &progress)?;
    println!("masters ready in {:.2} s", started.elapsed().as_secs_f64());
    calibration.report_reads(&prepared);
    calibration.save_built_masters(&prepared)?;
    if lights.is_empty() {
        return Ok(());
    }
    if let Some(dir) = &calibration.save_calibrated {
        std::fs::create_dir_all(dir).context("cannot create the calibrated folder")?;
    }
    let started = Instant::now();
    let mut checksum = 0u64;
    calibrate_lights(
        &lights,
        prepared.set.as_ref(),
        &options,
        &cancel,
        &progress,
        &|index, frame| {
            if let Some(dir) = &calibration.save_calibrated {
                write_calibrated_light(&dir.join(light_file_name(&lights[index])), frame)?;
            }
            Ok(())
        },
        |_, frame| {
            checksum = checksum.wrapping_add(
                frame
                    .samples
                    .iter()
                    .step_by(997)
                    .map(|value| u64::from(*value))
                    .sum::<u64>(),
            );
            Ok(())
        },
    )?;
    let seconds = started.elapsed().as_secs_f64();
    println!(
        "{} lights calibrated in {seconds:.2} s ({:.0} ms per frame), checksum {checksum}",
        lights.len(),
        seconds * 1e3 / lights.len() as f64
    );
    Ok(())
}
