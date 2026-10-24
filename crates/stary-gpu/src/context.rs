use std::sync::{Arc, Mutex};

use crate::error::GpuError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuKind {
    Integrated,
    Discrete,
    Software,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInfo {
    pub name: String,
    pub backend: String,
    pub kind: GpuKind,
}

impl GpuInfo {
    /// Integrated GPUs (Apple silicon among them) work in the memory of the computer, so there is
    /// no separate video memory to run out of and nothing to copy over a bus.
    pub fn shares_memory(&self) -> bool {
        self.kind == GpuKind::Integrated
    }
}

impl std::fmt::Display for GpuInfo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({})", self.name, self.backend)
    }
}

/// An opened GPU. Making one takes noticeable time, so make it once and share it.
pub struct GpuContext {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) limits: wgpu::Limits,
    /// PCI vendor and device ids, to find the same GPU through the operating system.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) pci_ids: (u32, u32),
    info: GpuInfo,
    failure: Arc<Mutex<Option<String>>>,
}

fn kind_of(device_type: wgpu::DeviceType) -> GpuKind {
    match device_type {
        wgpu::DeviceType::IntegratedGpu => GpuKind::Integrated,
        wgpu::DeviceType::DiscreteGpu => GpuKind::Discrete,
        wgpu::DeviceType::Cpu => GpuKind::Software,
        _ => GpuKind::Other,
    }
}

fn most_powerful_adapter() -> Result<wgpu::Adapter, GpuError> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(|_| GpuError::NoAdapter)?;
    if kind_of(adapter.get_info().device_type) == GpuKind::Software {
        return Err(GpuError::NoAdapter);
    }
    Ok(adapter)
}

fn info_of(adapter: &wgpu::Adapter) -> GpuInfo {
    let details = adapter.get_info();
    GpuInfo {
        name: details.name,
        backend: format!("{:?}", details.backend),
        kind: kind_of(details.device_type),
    }
}

/// The GPU that would be used, found without opening it.
pub fn detect() -> Option<GpuInfo> {
    most_powerful_adapter()
        .ok()
        .map(|adapter| info_of(&adapter))
}

impl GpuContext {
    /// Opens the most powerful GPU. Software renderers are refused: they work but are far slower
    /// than the CPU code.
    pub fn new() -> Result<Self, GpuError> {
        let adapter = most_powerful_adapter()?;
        let info = info_of(&adapter);
        let details = adapter.get_info();
        let pci_ids = (details.vendor, details.device);
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("stary"),
            required_features: wgpu::Features::empty(),
            required_limits: limits.clone(),
            ..Default::default()
        }))
        .map_err(|error| GpuError::Device(error.to_string()))?;
        let failure = Arc::new(Mutex::new(None));
        let sink = Arc::clone(&failure);
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            if let Ok(mut slot) = sink.lock() {
                slot.get_or_insert_with(|| error.to_string());
            }
        }));
        Ok(Self {
            device,
            queue,
            limits,
            pci_ids,
            info,
            failure,
        })
    }

    pub fn info(&self) -> &GpuInfo {
        &self.info
    }

    /// The largest buffer a shader may read or write as a whole.
    pub fn max_binding_bytes(&self) -> u64 {
        self.limits
            .max_storage_buffer_binding_size
            .min(self.limits.max_buffer_size)
    }

    /// What the GPU reported since the last check, outside of the calls that make resources: the
    /// work already sent to it is lost when it does.
    pub(crate) fn check(&self) -> Result<(), GpuError> {
        let reported = self.failure.lock().ok().and_then(|mut slot| slot.take());
        reported.map_or(Ok(()), |message| Err(GpuError::Failed(message)))
    }

    /// Makes resources and turns what the GPU says about them into an error.
    pub(crate) fn allocate<T>(&self, make: impl FnOnce() -> T) -> Result<T, GpuError> {
        let memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let made = make();
        let invalid = pollster::block_on(validation.pop());
        let exhausted = pollster::block_on(memory.pop());
        if exhausted.is_some() {
            return Err(GpuError::OutOfMemory);
        }
        match invalid {
            Some(error) => Err(GpuError::Failed(error.to_string())),
            None => Ok(made),
        }
    }
}
