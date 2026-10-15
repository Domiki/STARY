use std::{fs::File, io::BufWriter, path::Path};

use stary_color::{Profile, Space};
use stary_core::{LinearRgb16, LinearRgbF32, ResultColor, StaryKind, stary_kind::keyword};
use tiff::{
    encoder::{TiffEncoder, colortype},
    tags::Tag,
};

use crate::{
    error::IoError,
    fits::{Card, write_fits_rgb_f32},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Tiff32,
    Tiff16,
    Fits32,
}

impl OutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Tiff32 | Self::Tiff16 => "tif",
            Self::Fits32 => "fits",
        }
    }
}

const FULL_SCALE: f32 = 65535.0;

/// A color result to write: 16-bit integers (star trails) or floats (stacks).
#[derive(Clone, Copy)]
pub enum ResultImage<'a> {
    U16(&'a LinearRgb16),
    F32(&'a LinearRgbF32),
}

impl<'a> From<&'a LinearRgb16> for ResultImage<'a> {
    fn from(image: &'a LinearRgb16) -> Self {
        Self::U16(image)
    }
}

impl<'a> From<&'a LinearRgbF32> for ResultImage<'a> {
    fn from(image: &'a LinearRgbF32) -> Self {
        Self::F32(image)
    }
}

impl ResultImage<'_> {
    const fn size(&self) -> (usize, usize) {
        match self {
            Self::U16(image) => (image.width, image.height),
            Self::F32(image) => (image.width, image.height),
        }
    }

    fn floats(&self) -> std::borrow::Cow<'_, [f32]> {
        match self {
            Self::U16(image) => image
                .samples
                .iter()
                .map(|value| f32::from(*value) / FULL_SCALE)
                .collect(),
            Self::F32(image) => std::borrow::Cow::Borrowed(&image.samples),
        }
    }

    fn integers(&self) -> std::borrow::Cow<'_, [u16]> {
        match self {
            Self::U16(image) => std::borrow::Cow::Borrowed(&image.samples),
            Self::F32(image) => image
                .samples
                .iter()
                .map(|value| (value * FULL_SCALE).round().clamp(0.0, FULL_SCALE) as u16)
                .collect(),
        }
    }
}

pub(crate) fn tiff_error(path: &Path, error: tiff::TiffError) -> IoError {
    IoError::decode(path, error)
}

/// The ICC profile that says what the three values of a result mean.
pub fn profile_of(color: &ResultColor) -> Profile {
    match color {
        ResultColor::LinearSrgb => Profile::of_space(Space::Srgb, true),
        ResultColor::Camera(matrix) => Profile::camera(matrix),
    }
}

/// What a result file says about itself.
pub fn result_note(mode: &str, frames: u32) -> String {
    format!("STARY {mode}, {frames} frames, linear")
}

/// The mode and the number of frames of a note made by `result_note`.
pub fn parse_result_note(note: &str) -> Option<(String, u32)> {
    let rest = note.strip_prefix("STARY ")?;
    let (mode, rest) = rest.split_once(", ")?;
    let (frames, _) = rest.split_once(" frames")?;
    Some((mode.to_string(), frames.parse().ok()?))
}

/// Writes one image into a TIFF with a note and a color profile.
pub(crate) fn write_tiff_data<C>(
    path: &Path,
    width: usize,
    height: usize,
    data: &[C::Inner],
    note: &str,
    icc: &[u8],
) -> Result<(), IoError>
where
    C: colortype::ColorType,
    [C::Inner]: tiff::encoder::TiffValue,
{
    let file = File::create(path).map_err(IoError::read(path))?;
    let mut encoder = TiffEncoder::new(BufWriter::new(file)).map_err(|e| tiff_error(path, e))?;
    let mut page = encoder
        .new_image::<C>(width as u32, height as u32)
        .map_err(|e| tiff_error(path, e))?;
    page.encoder()
        .write_tag(Tag::ImageDescription, note)
        .map_err(|e| tiff_error(path, e))?;
    page.encoder()
        .write_tag(Tag::IccProfile, icc)
        .map_err(|e| tiff_error(path, e))?;
    page.write_data(data).map_err(|e| tiff_error(path, e))
}

/// Writes a linear color result. `frames` and `mode` are recorded so the file explains itself, and
/// `color` as an ICC profile in a TIFF.
pub fn write_result<'a>(
    path: &Path,
    image: impl Into<ResultImage<'a>>,
    format: OutputFormat,
    mode: &str,
    frames: u32,
    color: &ResultColor,
) -> Result<(), IoError> {
    let image = image.into();
    let note = result_note(mode, frames);
    let icc = profile_of(color).to_icc();
    let (width, height) = image.size();
    match format {
        OutputFormat::Tiff32 => write_tiff_data::<colortype::RGB32Float>(
            path,
            width,
            height,
            &image.floats(),
            &note,
            &icc,
        ),
        OutputFormat::Tiff16 => {
            write_tiff_data::<colortype::RGB16>(path, width, height, &image.integers(), &note, &icc)
        }
        OutputFormat::Fits32 => {
            let cards = [
                Card::Text(
                    "STRYTYPE",
                    StaryKind::StackResult.keyword_value().to_string(),
                ),
                Card::Number(keyword::STACK_COUNT, f64::from(frames)),
                Card::Text("STRYMODE", mode.to_string()),
            ];
            write_fits_rgb_f32(path, width, height, &image.floats(), &cards)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{probe_file, read_preview};

    fn gradient() -> LinearRgb16 {
        let (width, height) = (8, 4);
        let samples = (0..width * height * 3).map(|i| (i * 600) as u16).collect();
        LinearRgb16 {
            width,
            height,
            samples,
        }
    }

    #[test]
    fn the_note_of_a_result_can_be_read_back() {
        assert_eq!(
            parse_result_note(&result_note("Trails (gap filling)", 102)),
            Some(("Trails (gap filling)".to_string(), 102))
        );
        assert_eq!(parse_result_note("something else"), None);
    }

    #[test]
    fn a_float_tiff_can_be_probed_and_previewed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("result.tif");
        write_result(
            &path,
            &gradient(),
            OutputFormat::Tiff32,
            "Trails",
            5,
            &ResultColor::LinearSrgb,
        )
        .unwrap();
        let info = probe_file(&path).unwrap();
        assert_eq!((info.details.width, info.details.height), (8, 4));
        assert_eq!(info.details.bit_depth, Some(32));
        let preview = read_preview(&path, 100).unwrap();
        assert_eq!(
            preview.rgb8.len(),
            preview.width as usize * preview.height as usize * 3
        );
        assert!(
            preview.rgb8.iter().any(|value| *value > 20),
            "float preview should be stretched"
        );
    }

    #[test]
    fn a_float_result_can_be_written_in_every_format() {
        let dir = tempfile::tempdir().unwrap();
        let image = LinearRgbF32 {
            width: 4,
            height: 2,
            samples: (0..24).map(|i| i as f32 / 30.0 - 0.1).collect(),
        };
        for (name, format, bits) in [
            ("a.tif", OutputFormat::Tiff32, 32),
            ("b.tif", OutputFormat::Tiff16, 16),
            ("c.fits", OutputFormat::Fits32, 32),
        ] {
            let path = dir.path().join(name);
            write_result(
                &path,
                &image,
                format,
                "Stacking",
                3,
                &ResultColor::LinearSrgb,
            )
            .unwrap();
            let info = probe_file(&path).unwrap();
            assert_eq!((info.details.width, info.details.height), (4, 2));
            assert_eq!(info.details.bit_depth, Some(bits));
        }
    }

    #[test]
    fn a_sixteen_bit_tiff_keeps_the_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("result16.tif");
        write_result(
            &path,
            &gradient(),
            OutputFormat::Tiff16,
            "Trails",
            5,
            &ResultColor::LinearSrgb,
        )
        .unwrap();
        assert_eq!(probe_file(&path).unwrap().details.bit_depth, Some(16));
    }

    #[test]
    fn a_fits_result_is_recognised_as_a_stack_result() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("result.fits");
        write_result(
            &path,
            &gradient(),
            OutputFormat::Fits32,
            "Trails",
            7,
            &ResultColor::LinearSrgb,
        )
        .unwrap();
        let info = probe_file(&path).unwrap();
        assert_eq!(info.details.stary_kind, Some(StaryKind::StackResult));
        assert_eq!(info.details.stack_count, Some(7));
        assert_eq!((info.details.width, info.details.height), (8, 4));
        let preview = read_preview(&path, 100).unwrap();
        assert_eq!(
            preview.rgb8.len(),
            preview.width as usize * preview.height as usize * 3
        );
    }
}
