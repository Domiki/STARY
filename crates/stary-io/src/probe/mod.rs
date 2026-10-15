mod exif_fields;
mod fits_probe;
mod image_probe;
mod raw_probe;

use std::path::Path;

use stary_core::{ProbeDetails, ProbeInfo, SourceFormat};

use crate::{detect::detect_format, error::IoError, fingerprint::fingerprint_of};

fn details_of(path: &Path, format: SourceFormat) -> Result<ProbeDetails, IoError> {
    match format {
        SourceFormat::Raw => raw_probe::probe_raw(path),
        SourceFormat::Fits => fits_probe::probe_fits(path),
        SourceFormat::Jpeg | SourceFormat::Png | SourceFormat::Tiff => {
            image_probe::probe_image(path)
        }
    }
}

pub fn probe_file(path: &Path) -> Result<ProbeInfo, IoError> {
    let format = detect_format(path)?;
    Ok(ProbeInfo {
        path: path.to_path_buf(),
        fingerprint: fingerprint_of(path)?,
        format,
        details: details_of(path, format)?,
    })
}
