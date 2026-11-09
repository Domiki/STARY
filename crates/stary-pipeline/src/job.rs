use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use stary_calib::Master;
use stary_core::{Frame, PixelLayout, ResultColor, SourceFormat, StaryKind};
use stary_io::{
    detect::detect_format,
    master_file::{ExternalBlack, read_master_lenient, write_calibrated_light, write_master},
    rgb_out::{OutputFormat, ResultImage, write_result},
};
use stary_raw::DemosaicQuality;

use crate::{
    budget::MemoryBudget,
    calibrate::{CalibrationInputs, Source, calibrate_lights, prepare_calibration},
    control::{Cancel, Progress, Stage},
    error::PipelineError,
    masking::Masking,
    master_build::{MasterNote, ReadMode, RunOptions},
    stacking::{
        ComputeDevice, Device, Exposure, StackGroup, StackOptions, StackSettings,
        run_stacking_groups,
    },
    trails::{TrailsMethod, TrailsSettings, run_trails},
};

/// What a job saves its result as: the best the program makes, with nothing lost. Other kinds are
/// made from it afterwards.
pub const RESULT_FORMAT: OutputFormat = OutputFormat::Tiff32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Calibration,
    Trails,
    Stacking,
}

#[derive(Debug, Clone, Default)]
pub enum CalSource {
    #[default]
    None,
    Frames(Vec<PathBuf>),
    Master(PathBuf),
}

/// A session that is stacked together with the one of the job: its lights and the calibration
/// files of its own.
#[derive(Debug, Clone)]
pub struct SessionInput {
    pub lights: Vec<PathBuf>,
    pub bias: CalSource,
    pub dark: CalSource,
    pub dark_flat: CalSource,
    pub flat: CalSource,
}

#[derive(Debug, Clone)]
pub struct JobSpec {
    pub kind: JobKind,
    pub lights: Vec<PathBuf>,
    pub bias: CalSource,
    pub dark: CalSource,
    pub dark_flat: CalSource,
    pub flat: CalSource,
    /// The other sessions of a stacking job; the lights above are those of the reference session.
    pub more_sessions: Vec<SessionInput>,
    pub apply_calibration: bool,
    pub masters_folder: Option<PathBuf>,
    pub calibrated_folder: Option<PathBuf>,
    pub result_folder: Option<PathBuf>,
    pub quality: DemosaicQuality,
    pub trails: TrailsMethod,
    pub stacking: StackSettings,
    pub memory_percent: u64,
    pub masking: Option<Masking>,
    pub device: Device,
    /// How the frames of the masters are read.
    pub read: ReadMode,
    /// Stack the sessions of each exposure apart and join them into one picture of a high dynamic
    /// range.
    pub hdr: bool,
}

/// A light that is not in the result, and why (see `SkipReason::code`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    pub path: PathBuf,
    pub reason: &'static str,
}

/// How many of the lights ended up in the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameCounts {
    pub used: u32,
    pub total: u32,
    /// Lights that could not be aligned, and lights left out for their quality.
    pub misaligned: u32,
    pub poor: u32,
    pub left_out: Vec<LeftOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JobReport {
    pub outputs: Vec<JobOutput>,
    pub frames: Option<FrameCounts>,
    /// What laid the frames on the picture, for a job that stacks.
    pub device: Option<ComputeDevice>,
    /// Set when a master was made in one read.
    pub one_read: Option<OneRead>,
}

/// Masters made in one read leave out the largest and smallest values of every pixel; this says
/// how often that room was full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OneRead {
    /// Thousandths of the pixels where every place for a spoiled value was used, so that more
    /// values than there was room for may be spoiled (the worst of the masters).
    pub full_per_thousand: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutput {
    Result(PathBuf),
    Master(PathBuf),
    Calibrated(PathBuf),
}

fn sensor_layout_of(file: Option<&PathBuf>) -> Option<PixelLayout> {
    Some(stary_io::probe_file(file?).ok()?.details.layout)
}

fn is_fits(path: &Path) -> bool {
    detect_format(path).is_ok_and(|format| format == SourceFormat::Fits)
}

fn resolve(
    source: &CalSource,
    layout: Option<PixelLayout>,
    external_black: ExternalBlack,
) -> Result<Source, PipelineError> {
    Ok(match source {
        CalSource::None => Source::None,
        CalSource::Frames(paths) => Source::Frames(paths.clone()),
        CalSource::Master(path) if is_fits(path) => {
            Source::Master(read_master_lenient(path, layout, external_black)?.0)
        }
        CalSource::Master(path) => Source::Frames(vec![path.clone()]),
    })
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

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("light")
        .to_string()
}

fn create_folder(folder: &Path) -> Result<(), PipelineError> {
    std::fs::create_dir_all(folder).map_err(|source| PipelineError::File {
        path: folder.to_path_buf(),
        source,
    })
}

/// `name.ext`, or `name_2.ext`, `name_3.ext` … when the file already exists.
pub fn unused_file_name(folder: &Path, stem: &str, extension: &str) -> PathBuf {
    let numbered = |number: u32| folder.join(format!("{stem}_{number}.{extension}"));
    let first = folder.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(numbered)
        .find(|candidate| !candidate.exists())
        .expect("an unused number exists")
}

fn save_masters(
    built: &[(StaryKind, Master)],
    folder: &Path,
    outputs: &mut Vec<JobOutput>,
) -> Result<(), PipelineError> {
    create_folder(folder)?;
    for (kind, master) in built {
        let path = folder.join(master_file_name(*kind));
        write_master(&path, master, *kind)?;
        outputs.push(JobOutput::Master(path));
    }
    Ok(())
}

struct CalibrationFiles<'a> {
    first_light: Option<&'a PathBuf>,
    bias: &'a CalSource,
    dark: &'a CalSource,
    dark_flat: &'a CalSource,
    flat: &'a CalSource,
}

fn calibration_of(
    files: &CalibrationFiles,
    apply: bool,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    notes: &mut Vec<MasterNote>,
) -> Result<crate::calibrate::PreparedCalibration, PipelineError> {
    if !apply {
        return Ok(crate::calibrate::PreparedCalibration {
            set: None,
            built: Vec::new(),
            notes: Vec::new(),
        });
    }
    let layout = sensor_layout_of(files.first_light);
    let inputs = CalibrationInputs {
        bias: resolve(files.bias, layout, ExternalBlack::Median)?,
        dark_flat: resolve(files.dark_flat, layout, ExternalBlack::Median)?,
        dark: resolve(files.dark, layout, ExternalBlack::Median)?,
        flat: resolve(files.flat, layout, ExternalBlack::Zero)?,
    };
    let prepared = prepare_calibration(&inputs, options, cancel, progress)?;
    notes.extend(prepared.notes.iter().copied());
    Ok(prepared)
}

fn calibration_for(
    spec: &JobSpec,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    notes: &mut Vec<MasterNote>,
) -> Result<crate::calibrate::PreparedCalibration, PipelineError> {
    let files = CalibrationFiles {
        first_light: spec.lights.first(),
        bias: &spec.bias,
        dark: &spec.dark,
        dark_flat: &spec.dark_flat,
        flat: &spec.flat,
    };
    calibration_of(
        &files,
        spec.apply_calibration,
        options,
        cancel,
        progress,
        notes,
    )
}

fn run_calibration_only(
    spec: &JobSpec,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    outputs: &mut Vec<JobOutput>,
    notes: &mut Vec<MasterNote>,
) -> Result<(), PipelineError> {
    let Some(folder) = spec
        .calibrated_folder
        .as_deref()
        .filter(|_| !spec.lights.is_empty())
    else {
        return Ok(());
    };
    let prepared = calibration_for(spec, options, cancel, progress, notes)?;
    if let Some(masters) = &spec.masters_folder {
        save_masters(&prepared.built, masters, outputs)?;
    }
    create_folder(folder)?;
    let saved = Mutex::new(Vec::new());
    let observer = calibrated_saver(spec, &saved);
    calibrate_lights(
        &spec.lights,
        prepared.set.as_ref(),
        options,
        cancel,
        progress,
        &observer,
        |_, _| Ok(()),
    )?;
    take_saved(&saved, outputs);
    Ok(())
}

/// Where each light is saved after calibration, in the order of all the lights of the job: the
/// other sessions have a folder of their own, their files may have the same names.
fn calibrated_destinations(spec: &JobSpec) -> Vec<PathBuf> {
    let Some(folder) = &spec.calibrated_folder else {
        return Vec::new();
    };
    let named = |folder: PathBuf, lights: &[PathBuf]| -> Vec<PathBuf> {
        lights
            .iter()
            .map(|light| folder.join(format!("{}.fits", stem_of(light))))
            .collect()
    };
    let mut destinations = named(folder.clone(), &spec.lights);
    for (number, session) in spec.more_sessions.iter().enumerate() {
        destinations.extend(named(session_folder(folder, number), &session.lights));
    }
    destinations
}

/// The folder of the session after the reference one, which is the `number`th (from 0) of them.
fn session_folder(folder: &Path, number: usize) -> PathBuf {
    folder.join(format!("session_{}", number + 2))
}

/// Saves each light after calibration when the job asks for it; the paths are collected in `saved`.
fn calibrated_saver<'a>(
    spec: &JobSpec,
    saved: &'a Mutex<Vec<JobOutput>>,
) -> impl Fn(usize, &Frame) -> Result<(), PipelineError> + Sync + 'a {
    let destinations = calibrated_destinations(spec);
    move |index, frame| {
        if let Some(path) = destinations.get(index) {
            if let Some(folder) = path.parent() {
                create_folder(folder)?;
            }
            write_calibrated_light(path, frame)?;
            saved
                .lock()
                .expect("calibrated list lock")
                .push(JobOutput::Calibrated(path.clone()));
        }
        Ok(())
    }
}

fn take_saved(saved: &Mutex<Vec<JobOutput>>, outputs: &mut Vec<JobOutput>) {
    let mut list = std::mem::take(&mut *saved.lock().expect("calibrated list lock"));
    list.sort_by_key(|output| format!("{output:?}"));
    outputs.append(&mut list);
}

struct Finished<'a> {
    color: ResultColor,
    tag: &'a str,
    mode: &'a str,
    frames: u32,
    /// How many lights the job was given.
    total: u32,
    skipped: (u32, u32),
    left_out: Vec<LeftOut>,
}

fn save_result<'a>(
    spec: &JobSpec,
    image: impl Into<ResultImage<'a>>,
    finished: &Finished,
    progress: &(dyn Fn(Progress) + Sync),
    outputs: &mut Vec<JobOutput>,
) -> Result<FrameCounts, PipelineError> {
    let first = spec.lights.first().ok_or(PipelineError::NoInput)?;
    progress(Progress {
        stage: Stage::Saving,
        done: 0,
        total: 1,
    });
    let path = result_path(spec, first, finished.tag, finished.frames)?;
    write_result(
        &path,
        image,
        RESULT_FORMAT,
        finished.mode,
        finished.frames,
        &finished.color,
    )?;
    progress(Progress {
        stage: Stage::Saving,
        done: 1,
        total: 1,
    });
    outputs.push(JobOutput::Result(path));
    Ok(FrameCounts {
        used: finished.frames,
        total: finished.total,
        misaligned: finished.skipped.0,
        poor: finished.skipped.1,
        left_out: finished.left_out.clone(),
    })
}

fn run_trails_job(
    spec: &JobSpec,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    outputs: &mut Vec<JobOutput>,
    notes: &mut Vec<MasterNote>,
) -> Result<FrameCounts, PipelineError> {
    spec.lights.first().ok_or(PipelineError::NoInput)?;
    let prepared = calibration_for(spec, options, cancel, progress, notes)?;
    if let Some(masters) = &spec.masters_folder {
        save_masters(&prepared.built, masters, outputs)?;
    }
    if let Some(folder) = &spec.calibrated_folder {
        create_folder(folder)?;
    }
    let saved = Mutex::new(Vec::new());
    let observer = calibrated_saver(spec, &saved);
    let mut settings = TrailsSettings::new(spec.quality, spec.trails);
    settings.masking = spec.masking.clone();
    let result = run_trails(
        &spec.lights,
        prepared.set.as_ref(),
        settings,
        options,
        cancel,
        progress,
        &observer,
    )?;
    let finished = Finished {
        color: ResultColor::LinearSrgb,
        tag: spec.trails.file_tag(),
        mode: match spec.trails {
            TrailsMethod::Lighten => "Trails",
            TrailsMethod::Comet { .. } => "Comet",
            TrailsMethod::GapFilling => "Trails (gap filling)",
        },
        frames: result.frames,
        total: spec.lights.len() as u32,
        skipped: (0, 0),
        left_out: Vec::new(),
    };
    take_saved(&saved, outputs);
    save_result(spec, &result.image, &finished, progress, outputs)
}

fn result_path(
    spec: &JobSpec,
    first: &Path,
    mode: &str,
    frames: u32,
) -> Result<PathBuf, PipelineError> {
    let folder = spec.result_folder.clone().unwrap_or_else(|| {
        first
            .parent()
            .map_or_else(|| PathBuf::from("."), |parent| parent.join("stary-output"))
    });
    create_folder(&folder)?;
    let stem = format!("{}_{mode}_{frames}f", stem_of(first));
    Ok(unused_file_name(&folder, &stem, RESULT_FORMAT.extension()))
}

/// What the first light of a session was exposed with.
fn exposure_of(lights: &[PathBuf]) -> Option<Exposure> {
    let details = stary_io::probe_file(lights.first()?).ok()?.details;
    Some(Exposure {
        seconds: f64::from(details.exposure_s?),
        iso: f64::from(details.iso.unwrap_or(1)),
    })
}

fn run_stacking_job(
    spec: &JobSpec,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    outputs: &mut Vec<JobOutput>,
    notes: &mut Vec<MasterNote>,
) -> Result<(FrameCounts, ComputeDevice), PipelineError> {
    spec.lights.first().ok_or(PipelineError::NoInput)?;
    let all_lights: Vec<PathBuf> = spec
        .lights
        .iter()
        .chain(
            spec.more_sessions
                .iter()
                .flat_map(|session| &session.lights),
        )
        .cloned()
        .collect();
    if all_lights.len() < 2 {
        return Err(PipelineError::NotEnoughFrames);
    }
    let prepared = calibration_for(spec, options, cancel, progress, notes)?;
    let more: Vec<_> = spec
        .more_sessions
        .iter()
        .map(|session| {
            let files = CalibrationFiles {
                first_light: session.lights.first(),
                bias: &session.bias,
                dark: &session.dark,
                dark_flat: &session.dark_flat,
                flat: &session.flat,
            };
            calibration_of(
                &files,
                spec.apply_calibration,
                options,
                cancel,
                progress,
                notes,
            )
        })
        .collect::<Result<_, _>>()?;
    if let Some(masters) = &spec.masters_folder {
        save_masters(&prepared.built, masters, outputs)?;
        for (number, session) in more.iter().enumerate() {
            save_masters(&session.built, &session_folder(masters, number), outputs)?;
        }
    }
    if let Some(folder) = &spec.calibrated_folder {
        create_folder(folder)?;
    }
    let saved = Mutex::new(Vec::new());
    let observer = calibrated_saver(spec, &saved);
    let mut stack_options = StackOptions::from_settings(
        spec.stacking,
        spec.quality,
        MemoryBudget::detect(Some(spec.memory_percent)),
    );
    stack_options.masking = spec.masking.clone();
    stack_options.device = spec.device;
    stack_options.hdr = spec.hdr;
    let groups: Vec<StackGroup> = std::iter::once(StackGroup {
        lights: &spec.lights,
        calibration: prepared.set.as_ref(),
        exposure: exposure_of(&spec.lights),
    })
    .chain(
        spec.more_sessions
            .iter()
            .zip(&more)
            .map(|(session, calibration)| StackGroup {
                lights: &session.lights,
                calibration: calibration.set.as_ref(),
                exposure: exposure_of(&session.lights),
            }),
    )
    .collect();
    let result = run_stacking_groups(
        &groups,
        &stack_options,
        options,
        &observer,
        cancel,
        progress,
    )?;
    take_saved(&saved, outputs);
    let finished = Finished {
        color: result.color,
        tag: "stack",
        mode: "Stacking",
        frames: result.frames,
        total: all_lights.len() as u32,
        skipped: result.skipped_counts(),
        left_out: result
            .skipped
            .iter()
            .filter_map(|skipped| {
                Some(LeftOut {
                    path: all_lights.get(skipped.index)?.clone(),
                    reason: skipped.reason.code(),
                })
            })
            .collect(),
    };
    let counts = save_result(spec, &result.image, &finished, progress, outputs)?;
    Ok((counts, result.device))
}

pub fn run_job(
    spec: &JobSpec,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<JobReport, PipelineError> {
    let options = &RunOptions {
        read: spec.read,
        ..*options
    };
    let mut outputs = Vec::new();
    let mut notes = Vec::new();
    let (frames, device) = match spec.kind {
        JobKind::Calibration => {
            run_calibration_only(spec, options, cancel, progress, &mut outputs, &mut notes)?;
            (None, None)
        }
        JobKind::Trails => (
            Some(run_trails_job(
                spec,
                options,
                cancel,
                progress,
                &mut outputs,
                &mut notes,
            )?),
            None,
        ),
        JobKind::Stacking => {
            let (counts, device) =
                run_stacking_job(spec, options, cancel, progress, &mut outputs, &mut notes)?;
            (Some(counts), Some(device))
        }
    };
    Ok(JobReport {
        outputs,
        frames,
        device,
        one_read: one_read_of(&notes),
    })
}

/// What to tell about masters that were made by reading their frames once: the largest share of
/// pixels (in thousandths) whose room for spoiled values was full.
fn one_read_of(notes: &[MasterNote]) -> Option<OneRead> {
    notes
        .iter()
        .filter(|note| note.reads == 1)
        .map(|note| (note.full_share * 1000.0).round() as u32)
        .max()
        .map(|full_per_thousand| OneRead { full_per_thousand })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_name_gets_a_number() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            unused_file_name(dir.path(), "a_trails", "tif"),
            dir.path().join("a_trails.tif")
        );
        std::fs::write(dir.path().join("a_trails.tif"), b"x").unwrap();
        assert_eq!(
            unused_file_name(dir.path(), "a_trails", "tif"),
            dir.path().join("a_trails_2.tif")
        );
        std::fs::write(dir.path().join("a_trails_2.tif"), b"x").unwrap();
        assert_eq!(
            unused_file_name(dir.path(), "a_trails", "tif"),
            dir.path().join("a_trails_3.tif")
        );
    }

    #[test]
    fn a_trails_job_without_lights_is_an_error() {
        let spec = JobSpec {
            kind: JobKind::Trails,
            lights: Vec::new(),
            bias: CalSource::None,
            dark: CalSource::None,
            dark_flat: CalSource::None,
            flat: CalSource::None,
            more_sessions: Vec::new(),
            apply_calibration: true,
            masters_folder: None,
            calibrated_folder: None,
            result_folder: None,
            quality: DemosaicQuality::Fast,
            trails: TrailsMethod::Lighten,
            stacking: StackSettings::default(),
            memory_percent: 50,
            masking: None,
            device: Device::Auto,
            read: ReadMode::Exact,
            hdr: false,
        };
        let result = run_job(&spec, &RunOptions::default(), &Cancel::default(), &|_| {});
        assert!(matches!(result, Err(PipelineError::NoInput)));
    }
}
