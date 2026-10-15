use std::{path::PathBuf, sync::Mutex};

use anyhow::{Context, bail};
use stary_core::StaryKind;
use stary_io::master_file::{ExternalBlack, read_master_lenient, write_master};
use stary_pipeline::{
    CalibrationInputs, MemoryBudget, PreparedCalibration, Progress, ReadMode, RunOptions, Source,
    Stage, estimate_costs, plan_in_flight,
};

#[derive(clap::Args)]
pub struct CalibrationArgs {
    #[arg(long, num_args = 1..)]
    pub bias: Vec<PathBuf>,
    #[arg(long, num_args = 1..)]
    pub dark: Vec<PathBuf>,
    #[arg(long, num_args = 1.., id = "dark-flat")]
    pub dark_flat: Vec<PathBuf>,
    #[arg(long, num_args = 1..)]
    pub flat: Vec<PathBuf>,
    #[arg(long, num_args = 1..)]
    pub light: Vec<PathBuf>,
    /// Use a master FITS written by STARY instead of integrating frames
    #[arg(long)]
    pub bias_master: Option<PathBuf>,
    #[arg(long)]
    pub dark_master: Option<PathBuf>,
    #[arg(long)]
    pub flat_master: Option<PathBuf>,
    #[arg(long)]
    pub save_masters: Option<PathBuf>,
    #[arg(long)]
    pub save_calibrated: Option<PathBuf>,
    /// Most frames decoded at the same time (default: decided from cores and memory)
    #[arg(long)]
    pub workers: Option<usize>,
    /// Share of the available memory the run may use
    #[arg(long, default_value_t = 50)]
    pub memory_percent: u64,
    /// Times the frames of a master are read: 2 clips outliers exactly, 1 is faster on slow drives
    /// and leaves out the largest and smallest values of every pixel instead
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=2))]
    pub read_passes: u8,
}

fn source(
    frames: &[PathBuf],
    master: &Option<PathBuf>,
    external_black: ExternalBlack,
) -> anyhow::Result<Source> {
    match (frames.is_empty(), master) {
        (false, Some(_)) => bail!("give either frames or a master, not both"),
        (false, None) => Ok(Source::Frames(stary_io::collect_files(frames))),
        (true, Some(path)) => Ok(Source::Master(
            read_master_lenient(path, None, external_black)?.0,
        )),
        (true, None) => Ok(Source::None),
    }
}

fn file_size(path: Option<&PathBuf>) -> u64 {
    path.and_then(|path| std::fs::metadata(path).ok())
        .map_or(0, |meta| meta.len())
}

impl CalibrationArgs {
    pub fn inputs(&self) -> anyhow::Result<CalibrationInputs> {
        Ok(CalibrationInputs {
            bias: source(&self.bias, &self.bias_master, ExternalBlack::Median)?,
            dark_flat: source(&self.dark_flat, &None, ExternalBlack::Median)?,
            dark: source(&self.dark, &self.dark_master, ExternalBlack::Median)?,
            flat: source(&self.flat, &self.flat_master, ExternalBlack::Zero)?,
        })
    }

    pub fn lights(&self) -> Vec<PathBuf> {
        stary_io::collect_files(&self.light)
    }

    pub fn run_options(&self) -> RunOptions {
        let first = [&self.light, &self.dark, &self.flat, &self.bias]
            .into_iter()
            .find_map(|list| stary_io::collect_files(list).into_iter().next());
        let cores = std::thread::available_parallelism().map_or(1, usize::from);
        let workers = match self.workers {
            Some(workers) => workers.max(1),
            None => {
                let (fixed, per_worker) = estimate_costs(file_size(first.as_ref()));
                let budget = MemoryBudget::detect(Some(self.memory_percent));
                let workers = plan_in_flight(budget, fixed, per_worker, cores);
                println!(
                    "memory budget {:.1} GB, about {:.0} MB per decoder -> up to {workers} decoders ({cores} cores)",
                    budget.bytes as f64 / 1e9,
                    per_worker as f64 / 1e6
                );
                workers
            }
        };
        RunOptions {
            workers,
            read: if self.read_passes == 1 {
                ReadMode::Fast
            } else {
                ReadMode::Exact
            },
            ..RunOptions::default()
        }
    }

    /// Says how each master was made, when the frames were read once.
    pub fn report_reads(&self, prepared: &PreparedCalibration) {
        for ((kind, _), note) in prepared.built.iter().zip(&prepared.notes) {
            if note.reads == 1 && self.read_passes == 1 {
                println!(
                    "{}: frames read once, room for spoiled values full at {:.2}% of the pixels",
                    master_file_name(*kind).trim_end_matches(".fits"),
                    note.full_share * 100.0
                );
            }
        }
    }

    pub fn save_built_masters(&self, prepared: &PreparedCalibration) -> anyhow::Result<()> {
        let Some(dir) = &self.save_masters else {
            return Ok(());
        };
        std::fs::create_dir_all(dir).context("cannot create the masters folder")?;
        for (kind, master) in &prepared.built {
            let path = dir.join(master_file_name(*kind));
            write_master(&path, master, *kind)?;
            println!("saved {}", path.display());
        }
        Ok(())
    }
}

fn master_file_name(kind: StaryKind) -> &'static str {
    match kind {
        StaryKind::MasterBias => "master_bias.fits",
        StaryKind::MasterDark => "master_dark.fits",
        StaryKind::MasterFlat => "master_flat.fits",
        StaryKind::MasterDarkFlat => "master_dark_flat.fits",
        StaryKind::StackResult => "result.fits",
    }
}

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Bias => "bias",
        Stage::DarkFlat => "dark flat",
        Stage::Dark => "dark",
        Stage::Flat => "flat",
        Stage::Lights => "lights",
        Stage::Align => "align",
        Stage::Stack => "stack",
        Stage::Demosaic => "demosaic",
        Stage::Saving => "saving",
    }
}

pub fn progress_printer(last: &Mutex<(Stage, usize)>) -> impl Fn(Progress) + Sync + '_ {
    let started = std::time::Instant::now();
    move |progress| {
        let mut seen = last.lock().expect("progress lock");
        let step = (progress.total / 4).max(1);
        let worth_printing = progress.done % step == 0 || progress.done == progress.total;
        if worth_printing && *seen != (progress.stage, progress.done) {
            println!(
                "  {:<10} {}/{}  [{:.1} s]",
                stage_name(progress.stage),
                progress.done,
                progress.total,
                started.elapsed().as_secs_f64()
            );
        }
        if worth_printing {
            *seen = (progress.stage, progress.done);
        }
    }
}
