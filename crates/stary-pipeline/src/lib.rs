mod budget;
mod calibrate;
mod control;
mod decode_stream;
mod demosaicer;
mod error;
mod job;
mod masking;
mod master_build;
mod planes;
mod stacking;
mod trails;

pub use budget::{MemoryBudget, RunPlan, estimate_costs, plan_in_flight, plan_run};
pub use calibrate::{
    CalibrationInputs, LightInputs, Observer, PreparedCalibration, Source, calibrate_lights,
    calibrate_lights_with, prepare_calibration,
};
pub use control::{Cancel, Progress, Stage};
pub use demosaicer::Demosaicer;
pub use error::PipelineError;
pub use job::{
    CalSource, FrameCounts, JobKind, JobOutput, JobReport, JobSpec, LeftOut, OneRead, run_job,
    unused_file_name,
};
pub use masking::{Ground, Masking};
pub use master_build::{MasterNote, ReadMode, RunOptions, build_master, integrate_master};
pub use stacking::{
    Combine, ComputeDevice, Device, FrameQuality, FrameReport, Poor, Reference, SkipReason,
    Skipped, StackOptions, StackPlan, StackResult, StackSettings, detect_gpu, no_observer,
    plan as plan_stack, run_stacking,
};
pub use stary_gpu::{GpuInfo, GpuKind};
pub use trails::{GapFillTuning, GapStats, TrailsMethod, TrailsResult, TrailsSettings, run_trails};

#[cfg(test)]
mod tests;
