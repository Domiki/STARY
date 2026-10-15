use std::sync::Arc;

use stary_gpu::{GpuContext, GpuError, GpuInfo, GpuMemory, StackGpu, StackLayout, stack_bytes};
use stary_stack::Kernel;

use super::plan::{Accumulation, StackShape};
use crate::{budget::MemoryBudget, error::PipelineError};

/// The GPU that would lay the frames, found without opening it.
pub fn detect_gpu() -> Option<GpuInfo> {
    stary_gpu::detect()
}

/// What the user asks to do the work of laying frames on the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Device {
    /// The GPU when there is one that can hold the picture, the processor otherwise.
    #[default]
    Auto,
    Cpu,
    Gpu,
}

/// What did the work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeDevice {
    Cpu,
    Gpu(GpuInfo),
}

pub enum Backend {
    Cpu,
    Gpu {
        stack: Box<StackGpu>,
        info: GpuInfo,
        bytes: u64,
        shares_memory: bool,
    },
}

impl Backend {
    pub fn device(&self) -> ComputeDevice {
        match self {
            Self::Cpu => ComputeDevice::Cpu,
            Self::Gpu { info, .. } => ComputeDevice::Gpu(info.clone()),
        }
    }

    pub fn accumulation(&self) -> Accumulation {
        match self {
            Self::Cpu => Accumulation::Host,
            Self::Gpu {
                bytes,
                shares_memory,
                ..
            } => Accumulation::Gpu {
                bytes: *bytes,
                shared: *shares_memory,
            },
        }
    }
}

fn open_gpu(shape: &StackShape, kernel: Kernel, budget: MemoryBudget) -> Result<Backend, GpuError> {
    let context = Arc::new(GpuContext::new()?);
    let memory = GpuMemory::detect(&context, budget.bytes);
    let bytes = stack_bytes(shape.width, shape.height, shape.strip_layers);
    if !memory.fits(bytes) {
        return Err(GpuError::TooLarge {
            needed: bytes,
            allowed: memory.budget_bytes,
        });
    }
    let info = context.info().clone();
    let stack = StackGpu::new(
        context,
        StackLayout {
            width: shape.width,
            height: shape.height,
            layers: shape.strip_layers,
            kernel,
        },
    )?;
    Ok(Backend::Gpu {
        stack: Box::new(stack),
        info,
        bytes,
        shares_memory: memory.shared,
    })
}

/// Picks what lays the frames on the picture. When the GPU is only a preference, anything that
/// keeps it from being used leaves the work to the processor.
pub fn choose_backend(
    device: Device,
    shape: &StackShape,
    kernel: Kernel,
    budget: MemoryBudget,
) -> Result<Backend, PipelineError> {
    match device {
        Device::Cpu => Ok(Backend::Cpu),
        Device::Gpu => Ok(open_gpu(shape, kernel, budget)?),
        Device::Auto => Ok(open_gpu(shape, kernel, budget).unwrap_or(Backend::Cpu)),
    }
}
