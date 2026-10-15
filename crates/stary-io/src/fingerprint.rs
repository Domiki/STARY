use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use stary_core::Fingerprint;

use crate::error::IoError;

fn unix_millis(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_millis() as u64)
}

pub fn fingerprint_of(path: &Path) -> Result<Fingerprint, IoError> {
    let metadata = std::fs::metadata(path).map_err(IoError::read(path))?;
    Ok(Fingerprint {
        size_bytes: metadata.len(),
        modified_unix_ms: metadata.modified().ok().and_then(unix_millis),
    })
}
