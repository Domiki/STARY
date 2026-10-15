use std::path::PathBuf;

use serde::Deserialize;
use stary_pipeline::{
    CalSource, Combine, Device, Ground, JobKind, JobSpec, Masking, ReadMode, Reference,
    StackSettings, TrailsMethod,
};
use stary_raw::DemosaicQuality;
use stary_register::Model;
use stary_stack::Kernel;

use super::command_error::CommandError;
use crate::mask_store::MaskStore;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KindWire {
    Calibration,
    Trails,
    Stacking,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelWire {
    Similarity,
    Affine,
    Homography,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResamplerWire {
    Bilinear,
    Bicubic,
    Lanczos3,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombineWire {
    Average,
    Sigma,
    Winsorized,
}

/// How the frames of the masters are read.
#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadWire {
    #[default]
    Exact,
    Fast,
}

impl From<ReadWire> for ReadMode {
    fn from(wire: ReadWire) -> Self {
        match wire {
            ReadWire::Exact => Self::Exact,
            ReadWire::Fast => Self::Fast,
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceWire {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

impl From<DeviceWire> for Device {
    fn from(wire: DeviceWire) -> Self {
        match wire {
            DeviceWire::Auto => Self::Auto,
            DeviceWire::Cpu => Self::Cpu,
            DeviceWire::Gpu => Self::Gpu,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrailsMethodWire {
    Lighten,
    Comet,
    GapFilling,
}

#[derive(Debug, Deserialize)]
pub struct TrailsWire {
    pub method: TrailsMethodWire,
    /// For a comet: how many frames its tail takes to fade.
    pub tail_frames: u32,
    /// For a comet: read the lights from the last to the first.
    #[serde(default)]
    pub reverse_order: bool,
}

impl From<TrailsWire> for TrailsMethod {
    fn from(wire: TrailsWire) -> Self {
        match wire.method {
            TrailsMethodWire::Lighten => Self::Lighten,
            TrailsMethodWire::Comet => Self::Comet {
                tail_frames: wire.tail_frames.max(1),
                reverse: wire.reverse_order,
            },
            TrailsMethodWire::GapFilling => Self::GapFilling,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StackingWire {
    pub model: ModelWire,
    pub resampler: ResamplerWire,
    pub combine: CombineWire,
    pub low: f32,
    pub high: f32,
    /// Position of the reference frame in the list of lights.
    pub reference_index: usize,
    pub skip_poor_frames: bool,
}

impl From<StackingWire> for StackSettings {
    fn from(wire: StackingWire) -> Self {
        let (low, high) = (wire.low, wire.high);
        Self {
            model: match wire.model {
                ModelWire::Similarity => Model::Similarity,
                ModelWire::Affine => Model::Affine,
                ModelWire::Homography => Model::Homography,
            },
            kernel: match wire.resampler {
                ResamplerWire::Bilinear => Kernel::Bilinear,
                ResamplerWire::Bicubic => Kernel::Bicubic,
                ResamplerWire::Lanczos3 => Kernel::Lanczos3,
            },
            combine: match wire.combine {
                CombineWire::Average => Combine::Average,
                CombineWire::Sigma => Combine::Sigma { low, high },
                CombineWire::Winsorized => Combine::Winsorized { low, high },
            },
            reference: Reference::Index(wire.reference_index),
            skip_poor_frames: wire.skip_poor_frames,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityWire {
    Fast,
    High,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlotWire {
    None,
    Frames { paths: Vec<String> },
    Master { path: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundWire {
    WithSky,
    Separate,
}

impl From<GroundWire> for Ground {
    fn from(wire: GroundWire) -> Self {
        match wire {
            GroundWire::WithSky => Self::WithSky,
            GroundWire::Separate => Self::Separate,
        }
    }
}

/// The sky painted for a session (see `set_mask`) and what to do with the ground.
#[derive(Debug, Deserialize)]
pub struct MaskingWire {
    pub session_id: String,
    pub ground: GroundWire,
}

#[derive(Debug, Deserialize)]
pub struct JobRequest {
    pub kind: KindWire,
    pub lights: Vec<String>,
    pub bias: SlotWire,
    pub dark: SlotWire,
    pub dark_flat: SlotWire,
    pub flat: SlotWire,
    pub apply_calibration: bool,
    pub masters_folder: Option<String>,
    pub calibrated_folder: Option<String>,
    pub result_folder: Option<String>,
    pub quality: QualityWire,
    pub trails: TrailsWire,
    pub stacking: StackingWire,
    pub memory_percent: u64,
    #[serde(default)]
    pub device: DeviceWire,
    #[serde(default)]
    pub read: ReadWire,
    #[serde(default)]
    pub masking: Option<MaskingWire>,
}

fn paths(values: Vec<String>) -> Vec<PathBuf> {
    values.into_iter().map(PathBuf::from).collect()
}

fn slot(wire: SlotWire) -> CalSource {
    match wire {
        SlotWire::None => CalSource::None,
        SlotWire::Frames { paths: values } => CalSource::Frames(paths(values)),
        SlotWire::Master { path } => CalSource::Master(PathBuf::from(path)),
    }
}

fn masking_of(
    wire: Option<MaskingWire>,
    masks: &MaskStore,
) -> Result<Option<Masking>, CommandError> {
    let Some(wire) = wire else {
        return Ok(None);
    };
    let mask = masks.get(&wire.session_id).ok_or_else(|| CommandError {
        code: "no_mask".into(),
        message: "the session has no mask".into(),
    })?;
    Ok(Some(Masking {
        mask,
        ground: wire.ground.into(),
    }))
}

impl JobRequest {
    pub fn into_spec(self, masks: &MaskStore) -> Result<JobSpec, CommandError> {
        let masking = masking_of(self.masking, masks)?;
        Ok(JobSpec {
            kind: match self.kind {
                KindWire::Calibration => JobKind::Calibration,
                KindWire::Trails => JobKind::Trails,
                KindWire::Stacking => JobKind::Stacking,
            },
            lights: paths(self.lights),
            bias: slot(self.bias),
            dark: slot(self.dark),
            dark_flat: slot(self.dark_flat),
            flat: slot(self.flat),
            apply_calibration: self.apply_calibration,
            masters_folder: self.masters_folder.map(PathBuf::from),
            calibrated_folder: self.calibrated_folder.map(PathBuf::from),
            result_folder: self.result_folder.map(PathBuf::from),
            quality: match self.quality {
                QualityWire::Fast => DemosaicQuality::Fast,
                QualityWire::High => DemosaicQuality::High,
            },
            trails: self.trails.into(),
            stacking: self.stacking.into(),
            memory_percent: self.memory_percent,
            masking,
            device: self.device.into(),
            read: self.read.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_from_the_ui_becomes_a_job_spec() {
        let json = r#"{
            "kind": "trails", "lights": ["/a/1.raf", "/a/2.raf"],
            "bias": {"kind": "none"},
            "dark": {"kind": "frames", "paths": ["/d/1.raf"]},
            "dark_flat": {"kind": "none"},
            "flat": {"kind": "master", "path": "/m/flat.fits"},
            "apply_calibration": true, "masters_folder": null, "calibrated_folder": "/c",
            "result_folder": null, "quality": "fast", "memory_percent": 50,
            "trails": {"method": "comet", "tail_frames": 12, "reverse_order": true},
            "stacking": {"model": "affine", "resampler": "lanczos3", "combine": "sigma",
                         "low": 2.5, "high": 3.5, "reference_index": 3,
                         "skip_poor_frames": false}
        }"#;
        let request: JobRequest = serde_json::from_str(json).unwrap();
        let spec = request.into_spec(&MaskStore::default()).unwrap();
        assert!(spec.masking.is_none());
        assert_eq!(spec.read, ReadMode::Exact);
        assert_eq!(spec.memory_percent, 50);
        assert_eq!(spec.device, Device::Auto);
        assert_eq!(
            spec.trails,
            TrailsMethod::Comet {
                tail_frames: 12,
                reverse: true
            }
        );
        assert_eq!(spec.stacking.model, Model::Affine);
        assert_eq!(spec.stacking.kernel, Kernel::Lanczos3);
        assert_eq!(
            spec.stacking.combine,
            Combine::Sigma {
                low: 2.5,
                high: 3.5
            }
        );
        assert_eq!(spec.stacking.reference, Reference::Index(3));
        assert!(!spec.stacking.skip_poor_frames);
        assert_eq!(spec.kind, JobKind::Trails);
        assert_eq!(spec.lights.len(), 2);
        assert!(matches!(spec.dark, CalSource::Frames(ref list) if list.len() == 1));
        assert!(matches!(spec.flat, CalSource::Master(_)));
        assert_eq!(spec.calibrated_folder, Some(PathBuf::from("/c")));
    }

    fn request_with(masking: &str) -> JobRequest {
        serde_json::from_str(&format!(
            r#"{{
            "kind": "stacking", "lights": ["/a/1.raf", "/a/2.raf"],
            "bias": {{"kind": "none"}}, "dark": {{"kind": "none"}},
            "dark_flat": {{"kind": "none"}}, "flat": {{"kind": "none"}},
            "apply_calibration": false, "masters_folder": null, "calibrated_folder": null,
            "result_folder": null, "quality": "fast", "memory_percent": 50,
            "trails": {{"method": "lighten", "tail_frames": 10, "reverse_order": false}},
            "stacking": {{"model": "similarity", "resampler": "bicubic", "combine": "sigma",
                         "low": 3, "high": 3, "reference_index": 0, "skip_poor_frames": true}},
            "masking": {masking}
        }}"#
        ))
        .unwrap()
    }

    #[test]
    fn the_mask_of_the_session_goes_into_the_job() {
        let masks = MaskStore::default();
        masks.set(
            "s1".into(),
            stary_mask::Mask::new(2, 1, vec![1.0, 0.0]).unwrap(),
        );
        let request = request_with(r#"{"session_id": "s1", "ground": "separate"}"#);
        let masking = request.into_spec(&masks).unwrap().masking.unwrap();
        assert!(masking.separates_ground());
        assert_eq!(masking.mask.width(), 2);
    }

    #[test]
    fn a_job_that_asks_for_a_mask_the_session_does_not_have_is_refused() {
        let request = request_with(r#"{"session_id": "gone", "ground": "with_sky"}"#);
        let error = request.into_spec(&MaskStore::default()).unwrap_err();
        assert_eq!(error.code, "no_mask");
    }

    #[test]
    fn the_read_mode_of_the_masters_comes_from_the_request() {
        let json = r#"{"kind": "calibration", "lights": [], "bias": {"kind": "none"},
            "dark": {"kind": "none"}, "dark_flat": {"kind": "none"}, "flat": {"kind": "none"},
            "apply_calibration": true, "masters_folder": null, "calibrated_folder": null,
            "result_folder": null, "quality": "fast", "memory_percent": 50,
            "trails": {"method": "lighten", "tail_frames": 10, "reverse_order": false},
            "stacking": {"model": "similarity", "resampler": "bicubic", "combine": "sigma",
                         "low": 3, "high": 3, "reference_index": 0, "skip_poor_frames": true},
            "read": "fast"}"#;
        let request: JobRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.into_spec(&MaskStore::default()).unwrap().read,
            ReadMode::Fast
        );
    }
}
