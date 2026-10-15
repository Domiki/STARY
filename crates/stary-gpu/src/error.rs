#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GpuError {
    #[error("no usable GPU was found")]
    NoAdapter,
    #[error("the GPU could not be opened: {0}")]
    Device(String),
    #[error("the picture is too large for this GPU ({needed} bytes needed, {allowed} allowed)")]
    TooLarge { needed: u64, allowed: u64 },
    #[error("the GPU ran out of memory")]
    OutOfMemory,
    #[error("the GPU failed: {0}")]
    Failed(String),
}
