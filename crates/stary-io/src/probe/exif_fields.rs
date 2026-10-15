use std::{fs::File, io::BufReader, path::Path};

use exif::{Exif, In, Reader, Tag, Value};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ExifFields {
    pub camera: Option<String>,
    pub exposure_s: Option<f32>,
    pub iso: Option<u32>,
    pub captured_at: Option<String>,
}

fn ascii(exif: &Exif, tag: Tag) -> Option<String> {
    match &exif.get_field(tag, In::PRIMARY)?.value {
        Value::Ascii(values) => values
            .first()
            .map(|bytes| {
                String::from_utf8_lossy(bytes)
                    .trim_matches(['\0', ' '])
                    .to_string()
            })
            .filter(|text| !text.is_empty()),
        _ => None,
    }
}

fn camera_name(make: Option<String>, model: Option<String>) -> Option<String> {
    match (make, model) {
        (Some(make), Some(model)) if model.starts_with(&make) => Some(model),
        (Some(make), Some(model)) => Some(format!("{make} {model}")),
        (None, model) => model,
        (make, None) => make,
    }
}

fn exposure_seconds(exif: &Exif) -> Option<f32> {
    match &exif.get_field(Tag::ExposureTime, In::PRIMARY)?.value {
        Value::Rational(values) => values.first().map(|value| value.to_f32()),
        _ => None,
    }
}

pub fn normalize_datetime(text: &str) -> String {
    let (date, time) = text.split_once(' ').unwrap_or((text, ""));
    let date = date.replace(':', "-");
    if time.is_empty() {
        date
    } else {
        format!("{date}T{time}")
    }
}

fn fields_of(exif: &Exif) -> ExifFields {
    ExifFields {
        camera: camera_name(ascii(exif, Tag::Make), ascii(exif, Tag::Model)),
        exposure_s: exposure_seconds(exif),
        iso: exif
            .get_field(Tag::PhotographicSensitivity, In::PRIMARY)
            .and_then(|field| field.value.get_uint(0)),
        captured_at: ascii(exif, Tag::DateTimeOriginal).map(|text| normalize_datetime(&text)),
    }
}

pub fn read_exif_fields(path: &Path) -> ExifFields {
    File::open(path)
        .ok()
        .and_then(|file| {
            Reader::new()
                .read_from_container(&mut BufReader::new(file))
                .ok()
        })
        .map(|exif| fields_of(&exif))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datetime_is_iso_like() {
        assert_eq!(
            normalize_datetime("2026:10:01 22:15:03"),
            "2026-10-01T22:15:03"
        );
    }

    #[test]
    fn camera_name_avoids_repeating_make() {
        assert_eq!(
            camera_name(Some("SONY".into()), Some("SONY ILCE-7M3".into())),
            Some("SONY ILCE-7M3".into())
        );
        assert_eq!(
            camera_name(Some("FUJIFILM".into()), Some("X-T3".into())),
            Some("FUJIFILM X-T3".into())
        );
    }
}
