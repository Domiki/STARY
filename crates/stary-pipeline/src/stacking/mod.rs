mod device;
mod integrate;
mod plan;
mod prepare;
mod quality;
mod registration;
mod streaming;

use std::path::PathBuf;

use stary_calib::CalibrationSet;
use stary_core::{COLORS, Frame, LinearRgbF32, ResultColor};
use stary_raw::{DemosaicQuality, OutputColor};
use stary_register::{AlignError, AlignParams, Alignment, DetectParams, Model};
use stary_stack::{Kernel, Matching, Rejection};

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
    let first = lights.first().ok_or(PipelineError::NoInput)?;
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
        lights,
        calibration,
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

    let reference_sky = prepared[registration.reference].sky;
    let mut members = Vec::new();
    let mut skipped = Vec::new();
    let qualities: Vec<FrameQuality> = prepared.iter().map(quality::measure).collect();
    let poor = if options.skip_poor_frames {
        quality::assess(&qualities, registration.reference)
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
                    sky: reference_sky,
                    planes: None,
                };
                members.push(Member {
                    index,
                    to_source,
                    matching: matching_for(frame, &reference, options.normalize),
                    planes: frame.planes.take(),
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

    let ground = options
        .masking
        .as_ref()
        .filter(|masking| masking.separates_ground())
        .map(|masking| &*masking.mask);
    let image = if exact {
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
        integrate(&members, &setup, cancel, progress)?
    } else {
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
        integrate_streaming(&members, &setup, backend, cancel, progress)?
    };
    Ok(StackResult {
        image,
        frames: members.len() as u32,
        skipped,
        reference: registration.reference,
        reports,
        device,
        color,
    })
}
