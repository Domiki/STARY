use std::path::Path;

use serde::Serialize;
use stary_core::ProbeInfo;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProbeOutcome {
    Ok {
        info: ProbeInfo,
    },
    Error {
        path: String,
        code: String,
        message: String,
    },
}

impl ProbeOutcome {
    pub fn of(path: &Path) -> Self {
        match stary_io::probe_file(path) {
            Ok(info) => Self::Ok { info },
            Err(error) => Self::Error {
                path: path.display().to_string(),
                code: error.code().into(),
                message: error.to_string(),
            },
        }
    }
}
