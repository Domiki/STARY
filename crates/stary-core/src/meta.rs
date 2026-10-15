use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{cfa::CfaPattern, format::SourceFormat, stary_kind::StaryKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub size_bytes: u64,
    pub modified_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PixelLayout {
    Cfa(CfaPattern),
    Rgb,
    Mono,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeDetails {
    pub width: u32,
    pub height: u32,
    pub layout: PixelLayout,
    pub bit_depth: Option<u8>,
    pub camera: Option<String>,
    pub exposure_s: Option<f32>,
    pub iso: Option<u32>,
    pub captured_at: Option<String>,
    pub stary_kind: Option<StaryKind>,
    pub stack_count: Option<u32>,
    pub calibrated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeInfo {
    pub path: PathBuf,
    pub fingerprint: Fingerprint,
    pub format: SourceFormat,
    #[serde(flatten)]
    pub details: ProbeDetails,
}
