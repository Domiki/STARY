use crate::context::GpuContext;

const DEFAULT_DEDICATED_BYTES: u64 = 2 << 30;
const USABLE_SHARE_PERCENT: u64 = 80;

/// What the GPU may be asked to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMemory {
    pub budget_bytes: u64,
    /// The budget is a guess (nothing told the real size of the video memory).
    pub guessed: bool,
    /// The GPU works in the memory of the computer: what it holds counts against the memory budget
    /// of the whole run.
    pub shared: bool,
}

/// The size of the video memory of the primary GPU, from the operating system, when it can be told.
///
/// Not done yet on Windows (DXGI `QueryVideoMemoryInfo`) and Linux (Vulkan `VK_EXT_memory_budget`,
/// or `mem_info_vram_total` in sysfs for AMD); the guide for those systems says how.
pub fn platform_vram_bytes() -> Option<u64> {
    None
}

impl GpuMemory {
    /// `ram_budget_bytes` is the memory budget of the whole run, which an integrated GPU shares.
    pub fn detect(context: &GpuContext, ram_budget_bytes: u64) -> Self {
        if context.info().shares_memory() {
            return Self {
                budget_bytes: ram_budget_bytes,
                guessed: false,
                shared: true,
            };
        }
        if let Some(total) = platform_vram_bytes() {
            return Self {
                budget_bytes: total * USABLE_SHARE_PERCENT / 100,
                guessed: false,
                shared: false,
            };
        }
        Self {
            budget_bytes: DEFAULT_DEDICATED_BYTES,
            guessed: true,
            shared: false,
        }
    }

    pub const fn fits(&self, bytes: u64) -> bool {
        bytes <= self.budget_bytes
    }
}
