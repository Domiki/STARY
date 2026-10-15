use std::path::Path;

use rsraw::RawImage;
use stary_core::{BayerPattern, CfaPattern, PixelLayout, ProbeDetails};

use crate::error::IoError;

const XTRANS_FILTERS: u32 = 9;
const SECOND_GREEN: u32 = 3;

fn bayer_color(filters: u32, row: u32, column: u32) -> u8 {
    let index = filters >> (((row << 1 & 14) | (column & 1)) << 1) & 3;
    (if index == SECOND_GREEN { 1 } else { index }) as u8
}

fn bayer_pattern(filters: u32) -> Option<BayerPattern> {
    BayerPattern::from_colors([
        bayer_color(filters, 0, 0),
        bayer_color(filters, 0, 1),
        bayer_color(filters, 1, 0),
        bayer_color(filters, 1, 1),
    ])
}

fn xtrans_matrix(matrix: [[i8; 6]; 6]) -> [[u8; 6]; 6] {
    matrix.map(|row| row.map(|color| (color as u8).min(2)))
}

fn layout_of(raw: &RawImage, path: &Path) -> Result<PixelLayout, IoError> {
    let data = raw.as_ref();
    match data.idata.filters {
        0 if data.idata.colors == 1 => Ok(PixelLayout::Mono),
        0 => Ok(PixelLayout::Rgb),
        XTRANS_FILTERS => Ok(PixelLayout::Cfa(CfaPattern::XTrans(xtrans_matrix(
            data.idata.xtrans,
        )))),
        filters => bayer_pattern(filters)
            .map(|pattern| PixelLayout::Cfa(CfaPattern::Bayer(pattern)))
            .ok_or_else(|| IoError::unsupported(path, "unsupported color filter array")),
    }
}

fn camera_name(raw: &RawImage) -> Option<String> {
    let make = raw.normalized_make().to_string();
    let model = raw.normalized_model().to_string();
    match (make.is_empty(), model.is_empty()) {
        (true, true) => None,
        (true, false) => Some(model),
        (false, true) => Some(make),
        (false, false) if model.starts_with(&make) => Some(model),
        (false, false) => Some(format!("{make} {model}")),
    }
}

fn details_of(raw: &RawImage, path: &Path) -> Result<ProbeDetails, IoError> {
    Ok(ProbeDetails {
        width: raw.width(),
        height: raw.height(),
        layout: layout_of(raw, path)?,
        bit_depth: u8::try_from(raw.as_ref().color.raw_bps)
            .ok()
            .filter(|bits| *bits > 0),
        camera: camera_name(raw),
        exposure_s: Some(raw.shutter()).filter(|seconds| *seconds > 0.0),
        iso: Some(raw.iso_speed()).filter(|iso| *iso > 0),
        captured_at: raw
            .datetime()
            .map(|time| time.format("%Y-%m-%dT%H:%M:%S").to_string()),
        stary_kind: None,
        stack_count: None,
        calibrated: false,
    })
}

pub fn probe_raw(path: &Path) -> Result<ProbeDetails, IoError> {
    let bytes = std::fs::read(path).map_err(IoError::read(path))?;
    let raw =
        RawImage::open(&bytes).map_err(|error| IoError::decode(path, format!("{error:?}")))?;
    details_of(&raw, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libraw_filter_word_maps_to_rggb() {
        assert_eq!(bayer_pattern(0x94949494), Some(BayerPattern::Rggb));
        assert_eq!(bayer_pattern(0x16161616), Some(BayerPattern::Bggr));
    }
}
