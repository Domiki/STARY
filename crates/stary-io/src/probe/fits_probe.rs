use std::{fs::File, io::BufReader, path::Path};

use stary_core::{ProbeDetails, StaryKind, stary_kind::keyword};

use crate::{
    error::IoError,
    fits::{Header, geometry_of, layout_of, read_header},
};

fn first_number(header: &Header, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| header.number(key))
}

fn first_text(header: &Header, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| header.text(key))
        .map(str::to_string)
}

pub fn probe_fits(path: &Path) -> Result<ProbeDetails, IoError> {
    let file = File::open(path).map_err(IoError::read(path))?;
    let header = read_header(path, &mut BufReader::new(file))?;
    let geometry = geometry_of(path, &header)?;
    Ok(ProbeDetails {
        width: geometry.width as u32,
        height: geometry.height as u32,
        layout: layout_of(&header, geometry.planes),
        bit_depth: u8::try_from(geometry.bitpix.unsigned_abs()).ok(),
        camera: first_text(&header, &["INSTRUME", "CAMERA"]),
        exposure_s: first_number(&header, &["EXPTIME", "EXPOSURE"]).map(|value| value as f32),
        iso: first_number(&header, &["ISOSPEED", "ISO", "GAIN"]).map(|value| value as u32),
        captured_at: first_text(&header, &["DATE-OBS"]),
        stary_kind: header.text(keyword::KIND).and_then(StaryKind::parse),
        stack_count: header
            .number(keyword::STACK_COUNT)
            .map(|value| value as u32),
        calibrated: header.flag(keyword::CALIBRATED).unwrap_or(false),
    })
}
