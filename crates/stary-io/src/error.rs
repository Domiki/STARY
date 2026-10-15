use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("unsupported file {path}: {reason}")]
    Unsupported { path: PathBuf, reason: String },
    #[error("cannot decode {path}: {reason}")]
    Decode { path: PathBuf, reason: String },
}

impl IoError {
    pub fn read(path: &Path) -> impl Fn(std::io::Error) -> IoError {
        let path = path.to_path_buf();
        move |source| IoError::Read {
            path: path.clone(),
            source,
        }
    }

    pub fn unsupported(path: &Path, reason: impl Into<String>) -> IoError {
        IoError::Unsupported {
            path: path.to_path_buf(),
            reason: reason.into(),
        }
    }

    pub fn decode(path: &Path, reason: impl ToString) -> IoError {
        IoError::Decode {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        }
    }

    pub const fn code(&self) -> &'static str {
        match self {
            IoError::Read { .. } => "read_failed",
            IoError::Unsupported { .. } => "unsupported_file",
            IoError::Decode { .. } => "decode_failed",
        }
    }
}
