use tauri::{AppHandle, State};

use crate::jobs::JobRegistry;

/// Stops what is running and closes the program (after the user agreed to lose it).
#[tauri::command]
pub fn quit_app(app: AppHandle, registry: State<'_, JobRegistry>) {
    registry.cancel();
    app.exit(0);
}
