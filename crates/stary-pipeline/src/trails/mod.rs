mod gap_fill;
mod merge;

use std::path::PathBuf;

use stary_calib::{CalibrationSet, SensorGeometry};
use stary_core::{Frame, LinearRgb16};
use stary_raw::{DemosaicQuality, OutputColor};

pub use self::gap_fill::GapFillTuning;
use self::{
    gap_fill::{GapContext, frame_stars, light_needs_stars},
    merge::{GroundSum, Merge, Merged},
};
use crate::{
    calibrate::{LightInputs, Observer, calibrate_lights_with},
    control::{Cancel, Progress, Stage},
    demosaicer::Demosaicer,
    error::PipelineError,
    masking::{Masking, Rows, blend_rows, mix_u16},
    master_build::{RunOptions, geometry_of},
};

/// How the lights of a star trails run are put together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailsMethod {
    /// The brightest value of every sample.
    Lighten,
    /// Like Lighten, but older frames fade out over `tail_frames` frames, which leaves a tail.
    /// With `reverse` the list of lights is read from the last one to the first.
    Comet { tail_frames: u32, reverse: bool },
    /// Lighten with the gaps between the trails of consecutive frames filled in.
    GapFilling,
}

impl TrailsMethod {
    /// A word for the name of the result file.
    pub const fn file_tag(self) -> &'static str {
        match self {
            Self::Lighten => "trails",
            Self::Comet { .. } => "comet",
            Self::GapFilling => "gapfill",
        }
    }
}

/// What a star trails run does with the lights.
#[derive(Clone)]
pub struct TrailsSettings {
    pub quality: DemosaicQuality,
    pub method: TrailsMethod,
    pub masking: Option<Masking>,
    pub gap_fill: GapFillTuning,
}

impl TrailsSettings {
    pub fn new(quality: DemosaicQuality, method: TrailsMethod) -> Self {
        Self {
            quality,
            method,
            masking: None,
            gap_fill: GapFillTuning::default(),
        }
    }
}

/// How the gaps between the trails were filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapStats {
    /// Pairs of consecutive lights whose stars matched, and those whose stars did not.
    pub filled: u32,
    pub skipped: u32,
}

#[derive(Debug)]
pub struct TrailsResult {
    pub image: LinearRgb16,
    pub frames: u32,
    pub gaps: Option<GapStats>,
}

struct Accumulation {
    geometry: SensorGeometry,
    shape: Frame,
    merge: Merge,
    ground: Option<GroundSum>,
    frames: u32,
}

fn ground_sum_for(settings: &TrailsSettings, len: usize) -> Option<GroundSum> {
    let masking = settings.masking.as_ref().filter(|m| m.separates_ground())?;
    Some(GroundSum::new(masking.mask.clone(), len))
}

/// Star trails: every calibrated light is merged by keeping the brightest value of each sensor
/// sample (for a comet, the brightest of the faded values), and the color image is made once at the
/// end by demosaicing the merged sensor data. For gap filling the lights are merged by stretches,
/// the motion of the stars between consecutive lights is found from the stars of each light, and
/// the picture of every stretch is filled along that motion.
pub fn run_trails(
    lights: &[PathBuf],
    calibration: Option<&CalibrationSet>,
    settings: TrailsSettings,
    options: &RunOptions,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
    observer: Observer,
) -> Result<TrailsResult, PipelineError> {
    let first = lights.first().ok_or(PipelineError::NoInput)?;
    let filling = settings.method == TrailsMethod::GapFilling;
    if filling && lights.len() < 2 {
        return Err(PipelineError::NotEnoughFrames);
    }
    let wanted = if filling {
        light_needs_stars(lights.len(), &settings.gap_fill)
    } else {
        Vec::new()
    };
    let mut state: Option<Accumulation> = None;
    calibrate_lights_with(
        LightInputs {
            paths: lights,
            set: calibration,
        },
        options,
        cancel,
        progress,
        observer,
        |index, frame| {
            if wanted.get(index).copied().unwrap_or(false) {
                frame_stars(frame, &settings.gap_fill.detect)
            } else {
                Vec::new()
            }
        },
        |index, frame, stars| {
            let accumulation = state.get_or_insert_with(|| Accumulation {
                geometry: geometry_of(&frame),
                shape: Frame {
                    samples: Vec::new(),
                    ..frame
                },
                merge: Merge::new(&settings, frame.samples.len(), lights.len()),
                ground: ground_sum_for(&settings, frame.samples.len()),
                frames: 0,
            });
            accumulation
                .geometry
                .check_matches(&geometry_of(&frame))
                .map_err(|source| PipelineError::Geometry {
                    path: lights[index].clone(),
                    source,
                })?;
            accumulation.merge.add(&frame, index, lights.len(), stars)?;
            if let Some(ground) = &mut accumulation.ground {
                ground.add(&frame.samples);
            }
            accumulation.frames += 1;
            Ok(())
        },
    )?;
    let accumulation = state.ok_or(PipelineError::NoInput)?;
    if cancel.is_requested() {
        return Err(PipelineError::Cancelled);
    }
    let frames = accumulation.frames;
    let demosaicer = Demosaicer::open(first, settings.quality, OutputColor::Srgb)?;
    let ground = accumulation.ground.as_ref().map(|ground| Frame {
        samples: ground.mean(frames),
        ..accumulation.shape.clone()
    });
    let (mut image, plain, gaps) = match accumulation.merge.finish(accumulation.shape.black) {
        Merged::Sensor(samples) => {
            progress(Progress {
                stage: Stage::Demosaic,
                done: 0,
                total: 1,
            });
            let merged = Frame {
                samples,
                ..accumulation.shape.clone()
            };
            let image = demosaicer.run(&merged, first)?;
            progress(Progress {
                stage: Stage::Demosaic,
                done: 1,
                total: 1,
            });
            (image, None, None)
        }
        Merged::Gaps(filler) => {
            let picture = filler.finish(&GapContext {
                shape: &accumulation.shape,
                demosaicer: &demosaicer,
                first_light: first,
                masking: settings.masking.as_ref(),
                cancel,
                progress,
            })?;
            let gaps = GapStats {
                filled: picture.gaps_filled,
                skipped: picture.gaps_skipped,
            };
            (picture.image, picture.plain, Some(gaps))
        }
    };
    let placed = Rows {
        width: image.width,
        height: image.height,
        channels: 3,
        first: 0,
    };
    if let (Some(ground), Some(sums)) = (ground, &accumulation.ground) {
        let under = demosaicer.run(&ground, first)?;
        blend_rows(
            &mut image.samples,
            &under.samples,
            placed,
            &sums.mask,
            mix_u16,
        );
    } else if let (Some(plain), Some(masking)) = (plain, &settings.masking) {
        blend_rows(
            &mut image.samples,
            &plain.samples,
            placed,
            &masking.mask,
            mix_u16,
        );
    }
    Ok(TrailsResult {
        image,
        frames,
        gaps,
    })
}
