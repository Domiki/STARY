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

/// How much video memory of the opened GPU the operating system lets this program use, when it
/// can be told.
///
/// Not done yet on Linux (Vulkan `VK_EXT_memory_budget`, or `mem_info_vram_total` in sysfs for
/// AMD).
pub fn platform_vram_bytes(context: &GpuContext) -> Option<u64> {
    #[cfg(windows)]
    {
        let (vendor, device) = context.pci_ids;
        crate::vram_windows::local_budget_bytes(vendor, device)
    }
    #[cfg(not(windows))]
    {
        let _ = context;
        None
    }
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
        if let Some(available) = platform_vram_bytes(context) {
            return Self {
                budget_bytes: available * USABLE_SHARE_PERCENT / 100,
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::GpuKind;

    #[test]
    fn a_discrete_gpu_tells_its_video_memory_instead_of_a_guess() {
        let Ok(context) = GpuContext::new() else {
            return;
        };
        if context.info().kind != GpuKind::Discrete {
            return;
        }
        let available = platform_vram_bytes(&context).expect("DXGI knows the GPU wgpu opened");
        assert!(available >= 256 << 20, "{available} bytes");
        assert!(!GpuMemory::detect(&context, 0).guessed);
    }
}
