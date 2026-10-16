mod context;
mod error;
mod memory;
mod stack;
#[cfg(windows)]
mod vram_windows;

pub use context::{GpuContext, GpuInfo, GpuKind, detect};
pub use error::GpuError;
pub use memory::{GpuMemory, platform_vram_bytes};
pub use stack::{Pass, Placement, StackGpu, StackLayout, stack_bytes};
