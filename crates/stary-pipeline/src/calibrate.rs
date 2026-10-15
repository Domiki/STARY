use std::path::PathBuf;

use stary_calib::{CalibrationSet, Master, flat_gain};
use stary_core::{Frame, StaryKind};

use crate::{
    control::{Cancel, Progress, Stage},
    decode_stream::for_each_processed,
    error::PipelineError,
    master_build::{MasterNote, RunOptions, geometry_of, integrate_master},
};

#[derive(Debug, Clone, Default)]
pub enum Source {
    #[default]
    None,
    Frames(Vec<PathBuf>),
    Master(Master),
}

#[derive(Debug, Clone, Default)]
pub struct CalibrationInputs {
    pub bias: Source,
    pub dark_flat: Source,
    pub dark: Source,
    pub flat: Source,
}

pub struct PreparedCalibration {
    pub set: Option<CalibrationSet>,
    pub built: Vec<(StaryKind, Master)>,
    /// How each of the masters in `built` was made.
    pub notes: Vec<MasterNote>,
}

impl PreparedCalibration {
    fn of(set: Option<CalibrationSet>, built: Vec<(StaryKind, Master, MasterNote)>) -> Self {
        let notes = built.iter().map(|(_, _, note)| *note).collect();
        Self {
            set,
            built: built
                .into_iter()
                .map(|(kind, master, _)| (kind, master))
                .collect(),
            notes,
        }
    }
}

fn resolve(
    source: &Source,
    stage: Stage,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    built: &mut Vec<(StaryKind, Master, MasterNote)>,
    kind: StaryKind,
) -> Result<Option<Master>, PipelineError> {
    match source {
        Source::None => Ok(None),
        Source::Master(master) => Ok(Some(master.clone())),
        Source::Frames(paths) => {
            let (master, note) = integrate_master(paths, stage, options, cancel, progress)?;
            built.push((kind, master.clone(), note));
            Ok(Some(master))
        }
    }
}

/// Builds the masters in dependency order (bias, dark flat, dark, flat) and combines them into
/// one `CalibrationSet`. Masters that were built from frames are returned so they can be saved.
pub fn prepare_calibration(
    inputs: &CalibrationInputs,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<PreparedCalibration, PipelineError> {
    let mut built = Vec::new();
    let mut run =
        |source, stage, kind| resolve(source, stage, options, cancel, progress, &mut built, kind);
    let bias = run(&inputs.bias, Stage::Bias, StaryKind::MasterBias)?;
    let dark_flat = run(
        &inputs.dark_flat,
        Stage::DarkFlat,
        StaryKind::MasterDarkFlat,
    )?;
    let dark = run(&inputs.dark, Stage::Dark, StaryKind::MasterDark)?;
    let flat = run(&inputs.flat, Stage::Flat, StaryKind::MasterFlat)?;

    let Some(geometry) = [&flat, &dark, &bias, &dark_flat]
        .into_iter()
        .flatten()
        .map(|master| master.geometry)
        .next()
    else {
        return Ok(PreparedCalibration::of(None, built));
    };
    let gain = flat
        .as_ref()
        .map(|flat| flat_gain(flat, dark_flat.as_ref().or(bias.as_ref())))
        .transpose()
        .map_err(|source| PipelineError::Geometry {
            path: PathBuf::new(),
            source,
        })?;
    let set =
        CalibrationSet::new(geometry, dark.as_ref(), bias.as_ref(), gain).map_err(|source| {
            PipelineError::Geometry {
                path: PathBuf::new(),
                source,
            }
        })?;
    Ok(PreparedCalibration::of(Some(set), built))
}

/// Sees each light once it is calibrated, on the worker thread that made it (to save it, say).
pub type Observer<'a> = &'a (dyn Fn(usize, &Frame) -> Result<(), PipelineError> + Sync);

/// The lights of a run and what is taken off them.
#[derive(Clone, Copy)]
pub struct LightInputs<'a> {
    pub paths: &'a [PathBuf],
    pub set: Option<&'a CalibrationSet>,
}

/// Decodes and calibrates the lights in parallel, lets `observer` see each one on its worker, and
/// hands it to `consume` on the calling thread.
pub fn calibrate_lights<F>(
    paths: &[PathBuf],
    set: Option<&CalibrationSet>,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    observer: Observer,
    mut consume: F,
) -> Result<(), PipelineError>
where
    F: FnMut(usize, Frame) -> Result<(), PipelineError>,
{
    calibrate_lights_with(
        LightInputs { paths, set },
        options,
        cancel,
        progress,
        observer,
        |_, _| (),
        |index, frame, ()| consume(index, frame),
    )
}

/// Like `calibrate_lights`, and `analyze` looks at each calibrated light on its worker, so that
/// what it finds comes with the frame.
pub fn calibrate_lights_with<T, A, F>(
    inputs: LightInputs,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    observer: Observer,
    analyze: A,
    mut consume: F,
) -> Result<(), PipelineError>
where
    T: Send,
    A: Fn(usize, &Frame) -> T + Sync,
    F: FnMut(usize, Frame, T) -> Result<(), PipelineError>,
{
    let LightInputs { paths, set } = inputs;
    let mut done = 0;
    for_each_processed(
        paths.len(),
        options.workers,
        |index| {
            let mut frame = (options.decode)(&paths[index])?;
            if let Some(set) = set {
                set.apply(&mut frame.samples, frame.black, frame.maximum)
                    .map_err(|source| PipelineError::Geometry {
                        path: paths[index].clone(),
                        source,
                    })?;
            }
            debug_assert_eq!(geometry_of(&frame).len(), frame.samples.len());
            observer(index, &frame)?;
            let found = analyze(index, &frame);
            Ok((frame, found))
        },
        cancel,
        |index, (frame, found)| {
            consume(index, frame, found)?;
            done += 1;
            progress(Progress {
                stage: Stage::Lights,
                done,
                total: paths.len(),
            });
            Ok(())
        },
    )
}
