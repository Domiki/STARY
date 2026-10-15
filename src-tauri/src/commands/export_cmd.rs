use std::path::PathBuf;

use stary_io::export::{ExportError, export_result};

use crate::payload::{command_error::CommandError, export_request::ExportRequest};

impl From<ExportError> for CommandError {
    fn from(error: ExportError) -> Self {
        match error {
            ExportError::Io(error) => error.into(),
            ExportError::Invalid(reason) => Self {
                code: "invalid_export".into(),
                message: reason.into(),
            },
        }
    }
}

/// Makes a file of another kind from a saved result.
#[tauri::command]
pub async fn save_result_as(request: ExportRequest) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let (source, target) = (
            PathBuf::from(&request.source),
            PathBuf::from(&request.target),
        );
        export_result(&source, &target, &request.export()).map_err(CommandError::from)
    })
    .await
    .map_err(CommandError::internal)?
}
