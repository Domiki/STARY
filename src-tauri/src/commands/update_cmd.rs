use crate::{payload::command_error::CommandError, update};

/// The newest release on GitHub that is newer than `current`, if there is one. A failure (no
/// network, GitHub unreachable) is an error the screen may ignore.
#[tauri::command]
pub async fn check_update(current: String) -> Result<Option<update::NewerRelease>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || update::check(&current))
        .await
        .map_err(CommandError::internal)?
        .map_err(|message| CommandError {
            code: "update_check_failed".into(),
            message,
        })
}
