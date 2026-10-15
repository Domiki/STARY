use std::path::Path;

use stary_core::{Frame, LinearRgb16, ResultColor, SourceFormat};
use stary_io::detect::detect_format;
use stary_raw::{DemosaicQuality, OutputColor, camera_to_srgb, demosaic_cfa};

use crate::error::PipelineError;

/// Turns calibrated sensor data into a color image. Frames of a RAW camera are demosaiced by
/// LibRaw, which needs one RAW file of the same camera as a template; everything else is
/// converted by STARY's own code.
pub enum Demosaicer {
    Raw {
        template: Vec<u8>,
        quality: DemosaicQuality,
        color: OutputColor,
    },
    Builtin,
}

impl Demosaicer {
    pub fn open(
        first_light: &Path,
        quality: DemosaicQuality,
        color: OutputColor,
    ) -> Result<Self, PipelineError> {
        if detect_format(first_light)? != SourceFormat::Raw {
            return Ok(Self::Builtin);
        }
        let template = std::fs::read(first_light).map_err(|source| PipelineError::File {
            path: first_light.to_path_buf(),
            source,
        })?;
        Ok(Self::Raw {
            template,
            quality,
            color,
        })
    }

    /// What the three values of the images this makes mean.
    pub fn color(&self) -> ResultColor {
        match self {
            Self::Raw {
                template,
                color: OutputColor::Camera,
                ..
            } => camera_to_srgb(template)
                .ok()
                .flatten()
                .map_or(ResultColor::LinearSrgb, ResultColor::Camera),
            _ => ResultColor::LinearSrgb,
        }
    }

    pub fn run(&self, frame: &Frame, source: &Path) -> Result<LinearRgb16, PipelineError> {
        match self {
            Self::Raw {
                template,
                quality,
                color,
            } => demosaic_cfa(template, &frame.samples, *quality, *color).map_err(|error| {
                PipelineError::Raw {
                    path: source.to_path_buf(),
                    source: error,
                }
            }),
            Self::Builtin => {
                stary_demosaic::to_rgb(frame).map_err(|error| PipelineError::Demosaic {
                    path: source.to_path_buf(),
                    source: error,
                })
            }
        }
    }
}
