use std::path::PathBuf;

use stary_calib::GeometryError;
use stary_demosaic::DemosaicError;
use stary_gpu::GpuError;
use stary_io::IoError;
use stary_raw::RawError;

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("cancelled")]
    Cancelled,
    #[error("no input files")]
    NoInput,
    #[error("fewer than two frames could be aligned")]
    NotEnoughFrames,
    #[error("the sky of the mask has too few stars to align on")]
    NotEnoughSkyStars,
    #[error("{path}: {source}")]
    Raw { path: PathBuf, source: RawError },
    #[error("{path}: {source}")]
    Demosaic {
        path: PathBuf,
        source: DemosaicError,
    },
    #[error("{path}: {source}")]
    File {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Geometry {
        path: PathBuf,
        source: GeometryError,
    },
    #[error(transparent)]
    Io(#[from] IoError),
    #[error(transparent)]
    Gpu(#[from] GpuError),
}
