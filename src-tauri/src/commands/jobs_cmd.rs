use tauri::{AppHandle, State};

use crate::{
    jobs::{JobRegistry, spawn_job},
    mask_store::MaskStore,
    payload::{command_error::CommandError, job_request::JobRequest},
};

#[tauri::command]
pub fn start_job(
    app: AppHandle,
    registry: State<'_, JobRegistry>,
    masks: State<'_, MaskStore>,
    request: JobRequest,
) -> Result<(), CommandError> {
    let spec = request.into_spec(&masks)?;
    let cancel = registry.begin().ok_or(CommandError {
        code: "busy".into(),
        message: "another job is already running".into(),
    })?;
    spawn_job(app, spec, cancel);
    Ok(())
}

#[tauri::command]
pub fn cancel_job(registry: State<'_, JobRegistry>) {
    registry.cancel();
}
