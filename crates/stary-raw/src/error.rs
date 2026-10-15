#[derive(Debug, thiserror::Error)]
pub enum RawError {
    #[error("cannot open RAW data: {0}")]
    Open(String),
    #[error("cannot unpack RAW data: {0}")]
    Unpack(String),
    #[error("unsupported color filter array")]
    UnsupportedCfa,
    #[error("this file has no raw sensor buffer")]
    NoSensorBuffer,
    #[error("sensor buffer geometry is invalid")]
    BadGeometry,
}
