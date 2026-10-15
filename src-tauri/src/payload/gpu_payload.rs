use serde::Serialize;
use stary_pipeline::{ComputeDevice, GpuInfo, GpuKind};

#[derive(Debug, Clone, Serialize)]
pub struct GpuPayload {
    pub name: String,
    pub backend: String,
    pub shares_memory: bool,
}

impl From<GpuInfo> for GpuPayload {
    fn from(info: GpuInfo) -> Self {
        Self {
            shares_memory: info.kind == GpuKind::Integrated,
            name: info.name,
            backend: info.backend,
        }
    }
}

/// What laid the frames of a finished job on the picture.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeviceEvent {
    Cpu,
    Gpu { name: String },
}

impl From<ComputeDevice> for DeviceEvent {
    fn from(device: ComputeDevice) -> Self {
        match device {
            ComputeDevice::Cpu => Self::Cpu,
            ComputeDevice::Gpu(info) => Self::Gpu { name: info.name },
        }
    }
}
