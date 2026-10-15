use stary_mask::Mask;
use tauri::{
    State,
    ipc::{InvokeBody, Request},
};

use crate::{mask_store::MaskStore, payload::command_error::CommandError};

fn header(request: &Request<'_>, name: &str) -> Result<String, CommandError> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .ok_or_else(|| CommandError {
            code: "bad_request".into(),
            message: format!("the request has no {name}"),
        })
}

/// Keeps the mask of a session (gray bytes, 255 for sky) for the jobs that follow.
#[tauri::command]
pub fn set_mask(request: Request<'_>, masks: State<'_, MaskStore>) -> Result<(), CommandError> {
    let session = header(&request, "x-session")?;
    let parse = |name: &str| -> Result<usize, CommandError> {
        header(&request, name)?
            .parse()
            .map_err(CommandError::internal)
    };
    let (width, height) = (parse("x-width")?, parse("x-height")?);
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(CommandError {
            code: "bad_request".into(),
            message: "the request has no bytes".into(),
        });
    };
    let mask = Mask::from_bytes(width, height, bytes).map_err(CommandError::internal)?;
    masks.set(session, mask);
    Ok(())
}

#[tauri::command]
pub fn clear_mask(session: String, masks: State<'_, MaskStore>) {
    masks.remove(&session);
}
