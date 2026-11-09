use tauri::State;

use crate::{
    app_menu::{MenuHandles, MenuLabels},
    payload::command_error::CommandError,
};

/// Puts the words of the menu bar in the language of the screen.
#[tauri::command]
pub fn set_menu_text(
    handles: State<'_, MenuHandles>,
    labels: MenuLabels,
) -> Result<(), CommandError> {
    handles.set_text(&labels).map_err(CommandError::internal)
}
