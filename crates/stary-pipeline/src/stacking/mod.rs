mod device;
mod hdr;
mod integrate;
mod plan;
mod prepare;
mod quality;
mod registration;
mod streaming;

use std::path::{Path, PathBuf};

use stary_calib::CalibrationSet;
use stary_core::{COLORS, Frame, LinearRgbF32, ResultColor};
use stary_raw::{DemosaicQuality, OutputColor};
use stary_register::{AlignError, AlignParams, Alignment, DetectParams, Model};
use stary_stack::{Kernel, Matching, Rejection, SkyLevel};

use self::{
    device::{Backend, choose_backend},
    integrate::{Integration, Member, integrate},
    plan::{plan_stacking, plan_streaming},
    prepare::{Observer, PrepareContext, Prepared, prepare_frame},
    registration::{SkyView, register},
    streaming::{Clipping, StreamSetup, integrate_streaming},
};
use crate::{
    budget::MemoryBudget,
    control::{Cancel, Progress, Stage},
    decode_stream::for_each_processed,
    demosaicer::Demosaicer,
    error::PipelineError,
    masking::Masking,
    master_build::RunOptions,
};

pub use device::{ComputeDevice, Device, detect_gpu};
pub use hdr::Exposure;
pub use plan::{StackPlan, StackShape, plan_stacking as plan};
pub use quality::{FrameQuality, Poor};
pub use registration::Reference;

#[derive(Debug, Clone)]
pub struct StackOptions {
    pub quality: DemosaicQuality,
    pub model: Model,
    pub kernel: Kernel,
    pub rejection: Rejection,
    pub reference: Reference,
    pub normalize: bool,
    pub skip_poor_frames: bool,
    pub detect: DetectParams,
    pub align: AlignParams,
    pub budget: MemoryBudget,
    pub masking: Option<Masking>,
    pub device: Device,
    /// Stacks the sessions of each exposure apart and joins them (see `hdr`).
    pub hdr: bool,
}

/// How the aligned frames are combined.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combine {
    Average,
    Sigma { low: f32, high: f32 },
    Winsorized { low: f32, high: f32 },
}

impl From<Combine> for Rejection {
    fn from(combine: Combine) -> Self {
        match combine {
            Combine::Average => Self::None,
            Combine::Sigma { low, high } => Self::Sigma { low, high },
            Combine::Winsorized { low, high } => Self::Winsorized { low, high },
        }
    }
}

/// The choices of a stacking job that are not about demosaicing or memory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackSettings {
    pub model: Model,
    pub kernel: Kernel,
    pub combine: Combine,
    pub reference: Reference,
    /// Leave out frames that are much worse than the rest (clouds, blur, trailing).
    pub skip_poor_frames: bool,
}

impl Default for StackSettings {
    fn default() -> Self {
        Self {
            model: Model::Similarity,
            kernel: Kernel::Bicubic,
            combine: Combine::Sigma {
                low: 3.0,
                high: 3.0,
            },
            reference: Reference::Index(0),
            skip_poor_frames: true,
        }
    }
}

impl StackOptions {
    pub fn from_settings(
        settings: StackSettings,
        quality: DemosaicQuality,
        budget: MemoryBudget,
    ) -> Self {
        Self {
            quality,
            model: settings.model,
            kernel: settings.kernel,
            rejection: settings.combine.into(),
            reference: settings.reference,
            skip_poor_frames: settings.skip_poor_frames,
            ..Self::new(budget)
        }
    }

    pub fn new(budget: MemoryBudget) -> Self {
        Self {
            quality: DemosaicQuality::Fast,
            model: Model::Similarity,
            kernel: Kernel::Bicubic,
            rejection: Rejection::Sigma {
                low: 3.0,
                high: 3.0,
            },
            reference: Reference::Index(0),
            normalize: true,
            skip_poor_frames: true,
            detect: DetectParams::default(),
            align: AlignParams::default(),
            budget,
            masking: None,
            device: Device::Auto,
            hdr: false,
        }
    }
}

/// Why a light is not in the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Its stars did not match those of the reference frame.
    Misaligned(AlignError),
    Poor(Poor),
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Misaligned(error) => error.fmt(formatter),
            Self::Poor(poor) => poor.fmt(formatter),
        }
    }
}

impl SkipReason {
    /// A short name for the reason that stays the same between versions.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Misaligned(AlignError::TooFewStars) => "too_few_stars_to_align",
            Self::Misaligned(AlignError::NoMatch) => "no_match",
            Self::Poor(Poor::BrightSky) => "bright_sky",
            Self::Poor(Poor::Noisy) => "noisy",
            Self::Poor(Poor::FewStars) => "few_stars",
            Self::Poor(Poor::Blurry) => "blurry",
            Self::Poor(Poor::Trailed) => "trailed",
        }
    }
}

#[derive(Debug)]
pub struct Skipped {
    pub index: usize,
    pub reason: SkipReason,
}

/// What was found for one light.
#[derive(Debug)]
pub struct FrameReport {
    pub quality: FrameQuality,
    /// Set when the frame was left out for its quality.
    pub poor: Option<Poor>,
    pub alignment: Result<Alignment, AlignError>,
}

#[derive(Debug)]
pub struct StackResult {
    pub image: LinearRgbF32,
    pub frames: u32,
    pub skipped: Vec<Skipped>,
    pub reference: usize,
    pub reports: Vec<FrameReport>,
    /// What laid the frames on the picture.
    pub device: ComputeDevice,
    /// What the three values of a pixel of `image` mean.
    pub color: ResultColor,
}

impl StackResult {
    /// How many lights were left out because they could not be aligned, and for their quality.
    pub fn skipped_counts(&self) -> (u32, u32) {
        let poor = self
            .skipped
            .iter()
            .filter(|skipped| matches!(skipped.reason, SkipReason::Poor(_)))
            .count() as u32;
        (self.skipped.len() as u32 - poor, poor)
    }
}

pub fn no_observer(_: usize, _: &Frame) -> Result<(), PipelineError> {
    Ok(())
}

/// The exposures to stack apart, the longest first, and the exposure of each light's session
/// (an index into them). One exposure when the sessions are not to be joined.
fn exposure_classes(
    groups: &[StackGroup],
    ranges: &[std::ops::Range<usize>],
    hdr: bool,
) -> (Vec<Exposure>, Vec<usize>) {
    let exposures: Option<Vec<Exposure>> = groups.iter().map(|group| group.exposure).collect();
    let mut distinct: Vec<Exposure> = Vec::new();
    for exposure in exposures.iter().flatten().filter(|_| hdr) {
        if !distinct.iter().any(|known| known.same_as(*exposure)) {
            distinct.push(*exposure);
        }
    }
    let (Some(exposures), true) = (exposures, distinct.len() >= 2) else {
        let total = ranges.last().map_or(0, |range| range.end);
        return (Vec::new(), vec![0; total]);
    };
    distinct.sort_by(|a, b| b.ratio_to(*a).total_cmp(&1.0));
    let mut class_of_light = Vec::new();
    for (exposure, range) in exposures.iter().zip(ranges) {
        let class = distinct
            .iter()
            .position(|known| known.same_as(*exposure))
            .unwrap_or(0);
        class_of_light.extend(range.clone().map(|_| class));
    }
    (distinct, class_of_light)
}

/// The sky every frame of an exposure is matched to: that of the reference frame for its own
/// exposure, else of the frame with the most stars.
fn class_skies(
    prepared: &[Prepared],
    class_of_light: &[usize],
    classes: usize,
    reference: usize,
) -> Vec<[SkyLevel; COLORS]> {
    (0..classes.max(1))
        .map(|class| {
            let chosen = if class_of_light[reference] == class {
                reference
            } else {
                (0..prepared.len())
                    .filter(|&index| class_of_light[index] == class)
                    .max_by_key(|&index| prepared[index].stars.len())
                    .unwrap_or(reference)
            };
            prepared[chosen].sky
        })
        .collect()
}

fn group_ranges(groups: &[StackGroup]) -> Vec<std::ops::Range<usize>> {
    let mut start = 0;
    groups
        .iter()
        .map(|group| {
            let range = start..start + group.lights.len();
            start = range.end;
            range
        })
        .collect()
}

/// Every session is judged by its own frames: the nights differ in how bright their sky is, and
/// judging them together would throw a whole night away.
fn assess_by_session(
    qualities: &[FrameQuality],
    ranges: &[std::ops::Range<usize>],
    reference: usize,
) -> Vec<Option<Poor>> {
    ranges
        .iter()
        .flat_map(|range| {
            let keep = reference.checked_sub(range.start).unwrap_or(usize::MAX);
            quality::assess(&qualities[range.clone()], keep)
        })
        .collect()
}

/// The sessions are laid on one picture, so their frames must have the size and the pixel layout
/// of the first light.
fn check_same_sensor(groups: &[StackGroup], first: &Path) -> Result<(), PipelineError> {
    let reference = stary_io::probe_file(first)?.details;
    for light in groups
        .iter()
        .skip(1)
        .filter_map(|group| group.lights.first())
    {
        let details = stary_io::probe_file(light)?.details;
        if (details.width, details.height, details.layout)
            != (reference.width, reference.height, reference.layout)
        {
            return Err(PipelineError::OtherSensor(light.clone()));
        }
    }
    Ok(())
}

fn prepare_all(
    context: &PrepareContext,
    workers: usize,
    cached: usize,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Vec<Prepared>, PipelineError> {
    let total = context.lights.len();
    let mut slots: Vec<Option<Prepared>> = (0..total).map(|_| None).collect();
    let mut done = 0;
    for_each_processed(
        total,
        workers,
        |index| prepare_frame(context, index, index < cached),
        cancel,
        |index, prepared| {
            slots[index] = Some(prepared);
            done += 1;
            progress(Progress {
                stage: Stage::Lights,
                done,
                total,
            });
            Ok(())
        },
    )?;
    slots
        .into_iter()
        .map(|slot| slot.ok_or(PipelineError::NoInput))
        .collect()
}

fn matching_for(frame: &Prepared, reference: &Prepared, normalize: bool) -> [Matching; COLORS] {
    std::array::from_fn(|channel| {
        if normalize {
            Matching::between(frame.sky[channel], reference.sky[channel])
        } else {
            Matching::NONE
        }
    })
}

/// The lights of one session with the calibration that is made for them.
#[derive(Clone, Copy)]
pub struct StackGroup<'a> {
    pub lights: &'a [PathBuf],
    pub calibration: Option<&'a CalibrationSet>,
    /// What the lights were exposed with; needed to join exposures.
    pub exposure: Option<Exposure>,
}

/// Aligns the calibrated lights on one of them and combines them with outlier rejection.
pub fn run_stacking(
    lights: &[PathBuf],
    calibration: Option<&CalibrationSet>,
    options: &StackOptions,
    run: &RunOptions,
    observer: Observer,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<StackResult, PipelineError> {
    let group = StackGroup {
        lights,
        calibration,
        exposure: None,
    };
    run_stacking_groups(&[group], options, run, observer, cancel, progress)
}

/// The sessions are told apart only for calibration, the quality of their frames and the ground.
/// The first group is the reference session: the lights are numbered group after group, the size
/// of the result and the ground are those of that session.
pub fn run_stacking_groups(
    groups: &[StackGroup],
    options: &StackOptions,
    run: &RunOptions,
    observer: Observer,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<StackResult, PipelineError> {
    let lights: Vec<PathBuf> = groups
        .iter()
        .flat_map(|group| group.lights.iter().cloned())
        .collect();
    let calibrations: Vec<Option<&CalibrationSet>> = groups
        .iter()
        .flat_map(|group| group.lights.iter().map(|_| group.calibration))
        .collect();
    let ranges = group_ranges(groups);
    let (exposures, class_of_light) = exposure_classes(groups, &ranges, options.hdr);
    let first = lights.first().ok_or(PipelineError::NoInput)?;
    check_same_sensor(groups, first)?;
    let info = stary_io::probe_file(first)?;
    let file_bytes = std::fs::metadata(first).map_or(0, |metadata| metadata.len());
    let (width, height) = (info.details.width as usize, info.details.height as usize);
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    let shape = StackShape {
        width,
        height,
        frames: lights.len(),
        file_bytes,
        strip_layers: 1 + usize::from(
            options
                .masking
                .as_ref()
                .is_some_and(Masking::separates_ground),
        ),
    };
    let exact = matches!(options.rejection, Rejection::Winsorized { .. });
    let backend = if exact {
        Backend::Cpu
    } else {
        choose_backend(options.device, &shape, options.kernel, options.budget)?
    };
    let device = backend.device();
    let plan = if exact {
        plan_stacking(&shape, options.budget, cores)
    } else {
        plan_streaming(&shape, options.budget, cores, backend.accumulation())
    };
    let workers = plan.workers.min(run.workers.max(1));
    let demosaicer = Demosaicer::open(first, options.quality, OutputColor::Camera)?;
    let color = demosaicer.color();
    let context = PrepareContext {
        lights: &lights,
        calibrations: &calibrations,
        demosaicer: &demosaicer,
        decode: run.decode,
        detect: options.detect,
        observer,
    };
    let mut prepared = prepare_all(&context, workers, plan.cached_frames, cancel, progress)?;
    if cancel.is_requested() {
        return Err(PipelineError::Cancelled);
    }

    progress(Progress {
        stage: Stage::Align,
        done: 0,
        total: lights.len(),
    });
    let align = AlignParams {
        model: options.model,
        ..options.align
    };
    let sky = options.masking.as_ref().map(|masking| SkyView {
        mask: &masking.mask,
        width,
        height,
    });
    let registration = register(&prepared, options.reference, &align, sky.as_ref())?;
    progress(Progress {
        stage: Stage::Align,
        done: lights.len(),
        total: lights.len(),
    });

    let skies = class_skies(
        &prepared,
        &class_of_light,
        exposures.len(),
        registration.reference,
    );
    let mut members = Vec::new();
    let mut skipped = Vec::new();
    let qualities: Vec<FrameQuality> = prepared.iter().map(quality::measure).collect();
    let poor = if options.skip_poor_frames {
        assess_by_session(&qualities, &ranges, registration.reference)
    } else {
        vec![None; qualities.len()]
    };
    let reports: Vec<FrameReport> = registration
        .outcomes
        .iter()
        .zip(qualities.iter().zip(&poor))
        .map(|(outcome, (quality, poor))| FrameReport {
            quality: *quality,
            poor: *poor,
            alignment: *outcome,
        })
        .collect();
    for (index, outcome) in registration.outcomes.iter().enumerate() {
        if let Some(reason) = poor[index] {
            prepared[index].planes = None;
            skipped.push(Skipped {
                index,
                reason: SkipReason::Poor(reason),
            });
            continue;
        }
        match outcome
            .as_ref()
            .map(|alignment| alignment.transform.inverse())
        {
            Ok(Some(to_source)) => {
                let frame = &mut prepared[index];
                let reference = Prepared {
                    stars: Vec::new(),
                    sky: skies[class_of_light[index]],
                    planes: None,
                };
                members.push(Member {
                    index,
                    to_source,
                    matching: matching_for(frame, &reference, options.normalize),
                    planes: frame.planes.take(),
                    with_ground: ranges[0].contains(&index),
                });
            }
            Ok(None) => skipped.push(Skipped {
                index,
                reason: SkipReason::Misaligned(AlignError::NoMatch),
            }),
            Err(reason) => skipped.push(Skipped {
                index,
                reason: SkipReason::Misaligned(*reason),
            }),
        }
    }
    drop(prepared);
    if members.len() < 2 {
        return Err(PipelineError::NotEnoughFrames);
    }

    let frame_count = members.len() as u32;
    let joined = !exposures.is_empty();
    let ground = options
        .masking
        .as_ref()
        .filter(|masking| masking.separates_ground() && !joined)
        .map(|masking| &*masking.mask);
    let mut first_backend = Some(backend);
    let mut integrate_members = |members: &[Member]| -> Result<LinearRgbF32, PipelineError> {
        if exact {
            let setup = Integration {
                width,
                height,
                ground,
                kernel: options.kernel,
                rejection: options.rejection,
                strip_rows: plan.strip_rows,
                workers,
                context: &context,
            };
            return integrate(members, &setup, cancel, progress);
        }
        let backend = match first_backend.take() {
            Some(backend) => backend,
            None => choose_backend(options.device, &shape, options.kernel, options.budget)?,
        };
        let clipping = match options.rejection {
            Rejection::Sigma { low, high } => Some(Clipping { low, high }),
            _ => None,
        };
        let setup = StreamSetup {
            width,
            height,
            kernel: options.kernel,
            clipping,
            workers,
            context: &context,
            ground,
        };
        integrate_streaming(members, &setup, backend, cancel, progress)
    };
    let image = if joined {
        let mut layered: Option<(hdr::Layered, Exposure)> = None;
        let mut by_class: Vec<Vec<Member>> = exposures.iter().map(|_| Vec::new()).collect();
        for member in members {
            by_class[class_of_light[member.index]].push(member);
        }
        for (exposure, of_class) in exposures.iter().zip(by_class) {
            if of_class.is_empty() {
                continue;
            }
            let stack = integrate_members(&of_class)?;
            layered = Some(match layered {
                None => (hdr::Layered::start(stack), *exposure),
                Some((picture, base)) => (
                    hdr::add_shorter(picture, &stack, base.ratio_to(*exposure)),
                    base,
                ),
            });
        }
        layered
            .map(|(picture, _)| picture.image)
            .ok_or(PipelineError::NotEnoughFrames)?
    } else {
        integrate_members(&members)?
    };
    Ok(StackResult {
        image,
        frames: frame_count,
        skipped,
        reference: registration.reference,
        reports,
        device,
        color,
    })
}
