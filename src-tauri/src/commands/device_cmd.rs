use std::sync::OnceLock;

use stary_pipeline::detect_gpu;

use crate::payload::gpu_payload::GpuPayload;

static GPU: OnceLock<Option<GpuPayload>> = OnceLock::new();

#[tauri::command]
pub async fn gpu_info() -> Option<GpuPayload> {
    tauri::async_runtime::spawn_blocking(|| {
        GPU.get_or_init(|| detect_gpu().map(GpuPayload::from))
            .clone()
    })
    .await
    .ok()
    .flatten()
}
