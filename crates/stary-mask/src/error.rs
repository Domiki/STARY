#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MaskError {
    #[error("a mask needs at least one pixel")]
    NoPixels,
    #[error("the mask data does not match its size")]
    SizeMismatch,
}
