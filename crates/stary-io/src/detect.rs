use std::{fs::File, io::Read, path::Path};

use stary_core::SourceFormat;

use crate::error::IoError;

const RAW_EXTENSIONS: &[&str] = &[
    "arw", "sr2", "srf", "raf", "nef", "nrw", "cr2", "cr3", "crw", "dng", "orf", "rw2", "pef",
    "srw", "3fr", "erf", "kdc", "mrw", "mef", "mos", "iiq", "rwl",
];

const OTHER_EXTENSIONS: &[&str] = &["fit", "fits", "fts", "jpg", "jpeg", "png", "tif", "tiff"];

const MAGIC_LEN: usize = 16;

pub fn has_supported_extension(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    extension.is_some_and(|extension| {
        RAW_EXTENSIONS.contains(&extension.as_str())
            || OTHER_EXTENSIONS.contains(&extension.as_str())
    })
}

fn has_raw_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| RAW_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
}

fn format_from_magic(magic: &[u8]) -> Option<SourceFormat> {
    match magic {
        [b'S', b'I', b'M', b'P', b'L', b'E', b' ', b' ', b'=', ..] => Some(SourceFormat::Fits),
        [0xFF, 0xD8, 0xFF, ..] => Some(SourceFormat::Jpeg),
        [0x89, b'P', b'N', b'G', ..] => Some(SourceFormat::Png),
        [b'I', b'I', 0x2A, 0x00, ..] | [b'M', b'M', 0x00, 0x2A, ..] => Some(SourceFormat::Tiff),
        [b'I', b'I', 0x2B, 0x00, ..] | [b'M', b'M', 0x00, 0x2B, ..] => Some(SourceFormat::Tiff),
        _ => None,
    }
}

fn read_magic(path: &Path) -> Result<Vec<u8>, IoError> {
    let mut buffer = Vec::with_capacity(MAGIC_LEN);
    File::open(path)
        .and_then(|file| file.take(MAGIC_LEN as u64).read_to_end(&mut buffer))
        .map_err(IoError::read(path))?;
    Ok(buffer)
}

pub fn detect_format(path: &Path) -> Result<SourceFormat, IoError> {
    if has_raw_extension(path) {
        return Ok(SourceFormat::Raw);
    }
    format_from_magic(&read_magic(path)?)
        .ok_or_else(|| IoError::unsupported(path, "unrecognized file signature"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_signatures() {
        assert_eq!(
            format_from_magic(b"SIMPLE  =                    T"),
            Some(SourceFormat::Fits)
        );
        assert_eq!(
            format_from_magic(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(SourceFormat::Jpeg)
        );
        assert_eq!(
            format_from_magic(&[0x89, b'P', b'N', b'G', 0x0D]),
            Some(SourceFormat::Png)
        );
        assert_eq!(format_from_magic(b"II*\0rest"), Some(SourceFormat::Tiff));
        assert_eq!(format_from_magic(b"plain text"), None);
    }

    #[test]
    fn supported_extensions_cover_raw_fits_and_images() {
        for name in ["a.RAF", "b.fit", "c.FITS", "d.jpg", "e.tiff", "f.dng"] {
            assert!(has_supported_extension(Path::new(name)), "{name}");
        }
        assert!(!has_supported_extension(Path::new("notes.txt")));
        assert!(!has_supported_extension(Path::new("noextension")));
    }

    #[test]
    fn raw_extension_is_case_insensitive() {
        assert!(has_raw_extension(Path::new("/a/DSC0001.ARW")));
        assert!(has_raw_extension(Path::new("/a/DSCF0001.raf")));
        assert!(!has_raw_extension(Path::new("/a/photo.jpg")));
    }
}
