use tauri::State;

use crate::launch_args::LaunchContext;

#[tauri::command]
pub fn launch_context(context: State<'_, LaunchContext>) -> LaunchContext {
    context.inner().clone()
}
