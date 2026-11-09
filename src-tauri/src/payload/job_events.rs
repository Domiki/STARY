use serde::Serialize;
use stary_pipeline::{FrameCounts, JobOutput, JobReport, PipelineError, Progress, Stage};

use super::gpu_payload::DeviceEvent;

pub const PROGRESS_EVENT: &str = "job-progress";
pub const FINISHED_EVENT: &str = "job-finished";

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub stage: &'static str,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputEvent {
    pub kind: &'static str,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeftOutEvent {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FramesEvent {
    pub used: u32,
    pub total: u32,
    pub misaligned: u32,
    pub poor: u32,
    pub left_out: Vec<LeftOutEvent>,
}

impl From<FrameCounts> for FramesEvent {
    fn from(counts: FrameCounts) -> Self {
        Self {
            used: counts.used,
            total: counts.total,
            misaligned: counts.misaligned,
            poor: counts.poor,
            left_out: counts
                .left_out
                .into_iter()
                .map(|left| LeftOutEvent {
                    path: left.path.to_string_lossy().into_owned(),
                    reason: left.reason,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FailureEvent {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FinishedEvent {
    Done {
        outputs: Vec<OutputEvent>,
        frames: Option<FramesEvent>,
        device: Option<DeviceEvent>,
        /// Thousandths of the pixels of a master made in one read whose room for spoiled values
        /// was full; `None` when no master was made that way.
        one_read_full: Option<u32>,
        seconds: f64,
    },
    Cancelled,
    Error {
        failure: FailureEvent,
    },
}

pub const fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Bias => "bias",
        Stage::DarkFlat => "dark_flat",
        Stage::Dark => "dark",
        Stage::Flat => "flat",
        Stage::Lights => "lights",
        Stage::Align => "align",
        Stage::Stack => "stack",
        Stage::Demosaic => "demosaic",
        Stage::Saving => "saving",
    }
}

impl From<Progress> for ProgressEvent {
    fn from(progress: Progress) -> Self {
        Self {
            stage: stage_name(progress.stage),
            done: progress.done,
            total: progress.total,
        }
    }
}

fn output_event(output: &JobOutput) -> OutputEvent {
    match output {
        JobOutput::Result(path) => OutputEvent {
            kind: "result",
            path: path.display().to_string(),
        },
        JobOutput::Master(path) => OutputEvent {
            kind: "master",
            path: path.display().to_string(),
        },
        JobOutput::Calibrated(path) => OutputEvent {
            kind: "calibrated",
            path: path.display().to_string(),
        },
    }
}

fn failure_code(error: &PipelineError) -> &'static str {
    match error {
        PipelineError::Cancelled => "cancelled",
        PipelineError::NoInput => "no_input",
        PipelineError::NotEnoughFrames => "not_enough_frames",
        PipelineError::NotEnoughSkyStars => "not_enough_sky_stars",
        PipelineError::OtherSensor(_) => "other_sensor",
        PipelineError::Raw { .. } => "raw",
        PipelineError::Demosaic { .. } => "demosaic",
        PipelineError::File { .. } => "file",
        PipelineError::Geometry { .. } => "geometry",
        PipelineError::Io(_) => "io",
        PipelineError::Gpu(_) => "gpu",
    }
}

impl FinishedEvent {
    pub fn from_result(result: Result<JobReport, PipelineError>, seconds: f64) -> Self {
        match result {
            Ok(report) => Self::Done {
                outputs: report.outputs.iter().map(output_event).collect(),
                frames: report.frames.map(FramesEvent::from),
                device: report.device.map(DeviceEvent::from),
                one_read_full: report.one_read.map(|read| read.full_per_thousand),
                seconds,
            },
            Err(PipelineError::Cancelled) => Self::Cancelled,
            Err(error) => Self::Error {
                failure: FailureEvent {
                    code: failure_code(&error),
                    message: error.to_string(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn a_cancelled_run_is_reported_as_cancelled_not_as_an_error() {
        let event = FinishedEvent::from_result(Err(PipelineError::Cancelled), 1.0);
        assert!(matches!(event, FinishedEvent::Cancelled));
    }

    #[test]
    fn outputs_keep_their_kind() {
        let outputs = vec![
            JobOutput::Result(PathBuf::from("/r/a.tif")),
            JobOutput::Master(PathBuf::from("/m/m.fits")),
        ];
        let report = JobReport {
            outputs,
            frames: Some(FrameCounts {
                used: 38,
                total: 40,
                misaligned: 1,
                poor: 1,
                left_out: Vec::new(),
            }),
            device: None,
            one_read: None,
        };
        let FinishedEvent::Done {
            outputs, frames, ..
        } = FinishedEvent::from_result(Ok(report), 2.0)
        else {
            panic!("expected Done")
        };
        assert_eq!(outputs[0].kind, "result");
        assert_eq!(outputs[1].kind, "master");
        assert_eq!(
            frames.map(|counts| (counts.used, counts.total, counts.misaligned, counts.poor)),
            Some((38, 40, 1, 1))
        );
    }

    #[test]
    fn failures_carry_a_code_for_the_ui_to_translate() {
        let event = FinishedEvent::from_result(Err(PipelineError::NoInput), 0.1);
        let FinishedEvent::Error { failure } = event else {
            panic!("expected Error")
        };
        assert_eq!(failure.code, "no_input");
    }
}
