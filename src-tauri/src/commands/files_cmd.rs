use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tauri::ipc::Response;

use crate::payload::{
    command_error::CommandError, image_packet::image_packet, probe_outcome::ProbeOutcome,
};

#[tauri::command]
pub async fn probe_files(paths: Vec<String>) -> Result<Vec<ProbeOutcome>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .par_iter()
            .map(|path| ProbeOutcome::of(Path::new(path)))
            .collect()
    })
    .await
    .map_err(CommandError::internal)
}

#[tauri::command]
pub async fn open_image(path: String, max_edge: u32) -> Result<Response, CommandError> {
    let bytes =
        tauri::async_runtime::spawn_blocking(move || image_packet(Path::new(&path), max_edge))
            .await
            .map_err(CommandError::internal)??;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn expand_paths(paths: Vec<String>) -> Result<Vec<String>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let inputs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        stary_io::collect_files(&inputs)
            .into_iter()
            .map(|path| path.display().to_string())
            .collect()
    })
    .await
    .map_err(CommandError::internal)
}
