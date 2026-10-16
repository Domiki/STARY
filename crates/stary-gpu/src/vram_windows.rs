#![allow(unsafe_code)]

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
    IDXGIAdapter3, IDXGIFactory1,
};
use windows::core::Interface;

/// The budget DXGI gives this process in the local (dedicated) memory of the GPU with these PCI
/// ids. Trying allocations is no substitute: past the video memory WDDM pages instead of failing.
pub(crate) fn local_budget_bytes(vendor: u32, device: u32) -> Option<u64> {
    // SAFETY: plain COM factory creation; the result is an owned interface pointer.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    // SAFETY: enumeration stops at the first index DXGI refuses (DXGI_ERROR_NOT_FOUND).
    let adapters = (0..).map_while(|index| unsafe { factory.EnumAdapters1(index) }.ok());
    for adapter in adapters {
        // SAFETY: `adapter` is a live interface returned by the factory above.
        let Ok(description) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        if description.VendorId != vendor || description.DeviceId != device {
            continue;
        }
        let adapter: IDXGIAdapter3 = adapter.cast().ok()?;
        let mut memory = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
        // SAFETY: node 0 exists on every adapter, and `memory` outlives the call that fills it.
        unsafe { adapter.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut memory) }
            .ok()?;
        return Some(memory.Budget);
    }
    None
}
