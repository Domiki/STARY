#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SkyError {
    #[error("the picture has no pixels or its data does not match its size")]
    BadPicture,
    #[error("the picture is too small to tell the sky from the ground")]
    TooSmall,
}
