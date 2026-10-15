use serde::Serialize;
use stary_io::IoError;

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn internal(error: impl ToString) -> Self {
        Self {
            code: "internal".into(),
            message: error.to_string(),
        }
    }
}

impl From<IoError> for CommandError {
    fn from(error: IoError) -> Self {
        Self {
            code: error.code().into(),
            message: error.to_string(),
        }
    }
}

impl From<stary_pipeline::PipelineError> for CommandError {
    fn from(error: stary_pipeline::PipelineError) -> Self {
        Self {
            code: match error {
                stary_pipeline::PipelineError::NoInput => "no_input",
                _ => "internal",
            }
            .into(),
            message: error.to_string(),
        }
    }
}
