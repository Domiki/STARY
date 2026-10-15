use std::path::PathBuf;

use stary_calib::{
    ClippedAccumulator, MIN_FRAMES_FOR_CLIPPING, Master, SensorGeometry, StatsAccumulator,
    TrimmedAccumulator, trim_count,
};
use stary_core::Frame;

use crate::{
    control::{Cancel, Progress, Stage},
    decode_stream::{DecodeFn, decode_file, for_each_decoded},
    error::PipelineError,
};

/// How the frames of a master are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadMode {
    /// Twice: the first read finds the mean and the spread of every pixel and the second adds up
    /// what is within them. Exact, and slower when the files are slow to read.
    #[default]
    Exact,
    /// Once: the largest and smallest values of every pixel are kept on the side and left out of
    /// the mean. Spoiled values beyond what fits there stay in.
    Fast,
}

#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    pub workers: usize,
    pub kappa: f32,
    pub decode: DecodeFn,
    pub read: ReadMode,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            workers: std::thread::available_parallelism().map_or(1, usize::from),
            kappa: 3.0,
            decode: decode_file,
            read: ReadMode::Exact,
        }
    }
}

/// How a master was made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasterNote {
    /// How many times the frames were read.
    pub reads: u8,
    /// Reading once: the share of pixels where all the room for spoiled values was used, which
    /// says that more values than that may be spoiled. 0 when read twice.
    pub full_share: f32,
}

pub fn geometry_of(frame: &Frame) -> SensorGeometry {
    SensorGeometry {
        width: frame.width,
        height: frame.height,
        layout: frame.layout,
    }
}

struct Read<A> {
    geometry: SensorGeometry,
    accumulator: A,
    black_total: u64,
}

fn check_geometry(
    expected: &SensorGeometry,
    frame: &Frame,
    path: &std::path::Path,
) -> Result<(), PipelineError> {
    expected
        .check_matches(&geometry_of(frame))
        .map_err(|source| PipelineError::Geometry {
            path: path.to_path_buf(),
            source,
        })
}

/// The frames of a master, the stage they are shown under and how many reads the stage counts.
#[derive(Clone, Copy)]
struct Frames<'a> {
    paths: &'a [PathBuf],
    stage: Stage,
    passes: usize,
}

/// Reads every frame once and adds it to an accumulator made from the first one.
fn read_frames<A>(
    frames: Frames,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    make: impl Fn(usize) -> A,
    add: impl Fn(&mut A, &[u16]) -> Result<(), stary_calib::GeometryError>,
) -> Result<Read<A>, PipelineError> {
    let Frames {
        paths,
        stage,
        passes,
    } = frames;
    let mut state: Option<Read<A>> = None;
    let mut done = 0;
    for_each_decoded(
        paths,
        options.workers,
        options.decode,
        cancel,
        |index, frame| {
            let read = state.get_or_insert_with(|| Read {
                geometry: geometry_of(&frame),
                accumulator: make(frame.samples.len()),
                black_total: 0,
            });
            check_geometry(&read.geometry, &frame, &paths[index])?;
            add(&mut read.accumulator, &frame.samples).map_err(|source| {
                PipelineError::Geometry {
                    path: paths[index].clone(),
                    source,
                }
            })?;
            read.black_total += u64::from(frame.black);
            done += 1;
            progress(Progress {
                stage,
                done,
                total: paths.len() * passes,
            });
            Ok(())
        },
    )?;
    state.ok_or(PipelineError::NoInput)
}

fn second_pass(
    paths: &[PathBuf],
    stage: Stage,
    first: &Read<StatsAccumulator>,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Vec<f32>, PipelineError> {
    let mut clipped = ClippedAccumulator::new(first.accumulator.len());
    let mut done = paths.len();
    for_each_decoded(
        paths,
        options.workers,
        options.decode,
        cancel,
        |index, frame| {
            clipped
                .add(&first.accumulator, &frame.samples, options.kappa)
                .map_err(|source| PipelineError::Geometry {
                    path: paths[index].clone(),
                    source,
                })?;
            done += 1;
            progress(Progress {
                stage,
                done,
                total: paths.len() * 2,
            });
            Ok(())
        },
    )?;
    Ok(clipped.finish(&first.accumulator))
}

fn finished(
    paths: &[PathBuf],
    geometry: SensorGeometry,
    black_total: u64,
    samples: Vec<f32>,
) -> Result<Master, PipelineError> {
    let black = (black_total as f64 / paths.len() as f64).round() as u16;
    Master::new(geometry, black, paths.len() as u32, samples).map_err(|source| {
        PipelineError::Geometry {
            path: paths[0].clone(),
            source,
        }
    })
}

/// Integrates the frames into a master, and tells how. With enough frames the result rejects
/// outliers: by reading the files twice (sigma clipping), so that memory does not grow with the
/// number of frames, or, when asked to be fast and when the frames are not too many for it, by
/// reading them once and leaving out the largest and smallest values of every pixel.
pub fn integrate_master(
    paths: &[PathBuf],
    stage: Stage,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<(Master, MasterNote), PipelineError> {
    if paths.is_empty() {
        return Err(PipelineError::NoInput);
    }
    let frames = paths.len() as u32;
    let keep = (options.read == ReadMode::Fast)
        .then(|| trim_count(frames, options.kappa))
        .flatten();
    if let Some(keep) = keep {
        let read = read_frames(
            Frames {
                paths,
                stage,
                passes: 1,
            },
            options,
            cancel,
            progress,
            |len| TrimmedAccumulator::new(len, keep),
            |accumulator, samples| accumulator.add(samples),
        )?;
        let trimmed = read.accumulator.finish(options.kappa);
        let note = MasterNote {
            reads: 1,
            full_share: trimmed.full as f32 / trimmed.samples.len().max(1) as f32,
        };
        let master = finished(paths, read.geometry, read.black_total, trimmed.samples)?;
        return Ok((master, note));
    }
    let clipping = frames >= MIN_FRAMES_FOR_CLIPPING;
    let first = read_frames(
        Frames {
            paths,
            stage,
            passes: if clipping { 2 } else { 1 },
        },
        options,
        cancel,
        progress,
        StatsAccumulator::new,
        |accumulator, samples| accumulator.add(samples),
    )?;
    let samples = if clipping {
        second_pass(paths, stage, &first, options, cancel, progress)?
    } else {
        first.accumulator.mean()
    };
    let note = MasterNote {
        reads: if clipping { 2 } else { 1 },
        full_share: 0.0,
    };
    let master = finished(paths, first.geometry, first.black_total, samples)?;
    Ok((master, note))
}

pub fn build_master(
    paths: &[PathBuf],
    stage: Stage,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Master, PipelineError> {
    integrate_master(paths, stage, options, cancel, progress).map(|(master, _)| master)
}
