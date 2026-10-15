use std::{fs::File, io::BufWriter, path::Path};

use image::{
    ExtendedColorType, ImageEncoder,
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
};
use rayon::prelude::*;
use stary_color::{Conversion, Curve, Profile, Space};
use stary_core::{StaryKind, stary_kind::keyword, stretch::Stretch};
use tiff::{decoder::Decoder, tags::Tag};

use crate::{
    error::IoError,
    fits::{Card, write_fits_rgb_f32, write_fits_rgb_u16},
    rgb_out::{parse_result_note, result_note, tiff_error, write_tiff_data},
};

const FULL_SCALE: f32 = 65535.0;
const MAX_QUALITY: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Tiff,
    Png,
    Jpeg,
    Fits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Eight,
    Sixteen,
    /// 32-bit floating point: the values as they are, with nothing cut off.
    Float,
}

/// How the linear values of a result are turned into the values that are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Linear light as it is.
    Linear,
    /// Encoded with the curve of the color space (gamma): the dark tones get more room.
    Display,
    /// Stretched so that the background is a quarter of the range, then encoded: what a linear
    /// image needs to look right on a screen.
    Stretch,
}

/// What to make of a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Export {
    pub container: Container,
    pub depth: Depth,
    /// `None` keeps the colors the result has (camera colors, or sRGB).
    pub space: Option<Space>,
    pub tone: Tone,
    /// For JPEG, 1 to 100.
    pub quality: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Io(#[from] IoError),
}

impl Export {
    pub fn extension(&self) -> &'static str {
        match self.container {
            Container::Tiff => "tif",
            Container::Png => "png",
            Container::Jpeg => "jpg",
            Container::Fits => "fits",
        }
    }

    /// Whether the file can hold less than the result has: fewer bits, or a lossy format.
    pub fn loses_detail(&self) -> bool {
        self.depth != Depth::Float || self.container == Container::Jpeg
    }

    pub fn check(&self) -> Result<(), ExportError> {
        let invalid = |reason| Err(ExportError::Invalid(reason));
        match (self.container, self.depth) {
            (Container::Jpeg, Depth::Sixteen | Depth::Float) => {
                return invalid("JPEG holds only 8 bits");
            }
            (Container::Png, Depth::Float) => return invalid("PNG has no floating point"),
            (Container::Fits, Depth::Eight) => return invalid("FITS is saved in 16 or 32 bits"),
            _ => {}
        }
        if self.depth == Depth::Float && self.tone != Tone::Linear {
            return invalid("floating point values are saved linear");
        }
        if self.container == Container::Fits && (self.space.is_some() || self.tone != Tone::Linear)
        {
            return invalid("FITS keeps the colors and the linear values of the result");
        }
        if self.container == Container::Jpeg && !(1..=MAX_QUALITY).contains(&self.quality) {
            return invalid("the JPEG quality is from 1 to 100");
        }
        Ok(())
    }
}

struct Source {
    width: usize,
    height: usize,
    samples: Vec<f32>,
    icc: Option<Vec<u8>>,
    note: Option<String>,
}

fn read_source(path: &Path) -> Result<Source, IoError> {
    let file = File::open(path).map_err(IoError::read(path))?;
    let mut decoder = Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| tiff_error(path, e))?
        .with_limits(tiff::decoder::Limits::unlimited());
    let (width, height) = decoder.dimensions().map_err(|e| tiff_error(path, e))?;
    let samples = match decoder.read_image().map_err(|e| tiff_error(path, e))? {
        tiff::decoder::DecodingResult::F32(values) => values,
        tiff::decoder::DecodingResult::U16(values) => values
            .into_iter()
            .map(|value| f32::from(value) / FULL_SCALE)
            .collect(),
        _ => {
            return Err(IoError::unsupported(
                path,
                "only 16-bit and floating point results",
            ));
        }
    };
    if samples.len() != width as usize * height as usize * 3 {
        return Err(IoError::unsupported(
            path,
            "only color results can be saved again",
        ));
    }
    Ok(Source {
        width: width as usize,
        height: height as usize,
        samples,
        icc: decoder.get_tag_u8_vec(Tag::IccProfile).ok(),
        note: decoder.get_tag_ascii_string(Tag::ImageDescription).ok(),
    })
}

/// What the values are when they are written: the profile of the file, and the work on the values.
struct Plan {
    profile: Profile,
    conversion: Option<Conversion>,
    curve: Option<Curve>,
    stretch: bool,
}

fn plan_of(export: &Export, source: &Profile) -> Result<Plan, ExportError> {
    let linear = match export.space {
        Some(space) => Profile::of_space(space, true),
        None => source.with_curve(Curve::Linear, format!("{} linear", source.description)),
    };
    let (profile, curve) = match (export.tone, export.space) {
        (Tone::Linear, _) => (linear, None),
        (_, Some(space)) => (Profile::of_space(space, false), Some(space.curve())),
        (_, None) => {
            let name = linear.description.trim_end_matches(" linear").to_string();
            (
                linear.with_curve(Curve::Srgb, format!("{name} sRGB curve")),
                Some(Curve::Srgb),
            )
        }
    };
    let destination = match export.space {
        Some(space) => Profile::of_space(space, true),
        None => source.with_curve(Curve::Linear, String::new()),
    };
    let unchanged = export.space.is_none() && source.curve == Curve::Linear;
    let conversion = if unchanged {
        None
    } else {
        Some(
            Conversion::between(source, &destination)
                .ok_or(ExportError::Invalid("the color space cannot be converted"))?,
        )
    };
    Ok(Plan {
        profile,
        conversion,
        curve: if export.tone == Tone::Display {
            curve
        } else {
            None
        },
        stretch: export.tone == Tone::Stretch,
    })
}

fn apply(plan: &Plan, samples: &mut [f32]) {
    if let Some(conversion) = &plan.conversion {
        samples.par_chunks_exact_mut(3).for_each(|pixel| {
            let out = conversion.linear([pixel[0], pixel[1], pixel[2]]);
            pixel.copy_from_slice(&out);
        });
    }
    if plan.stretch {
        let stretch = Stretch::fit(samples, 3);
        samples.par_chunks_exact_mut(3).for_each(|pixel| {
            for (channel, value) in pixel.iter_mut().enumerate() {
                *value = stretch.apply(channel, *value);
            }
        });
    } else if let Some(curve) = &plan.curve {
        samples
            .par_iter_mut()
            .for_each(|value| *value = curve.encode(f64::from(*value)) as f32);
    }
}

fn to_u16(samples: &[f32]) -> Vec<u16> {
    samples
        .par_iter()
        .map(|value| (value * FULL_SCALE).round().clamp(0.0, FULL_SCALE) as u16)
        .collect()
}

fn to_u8(samples: &[f32]) -> Vec<u8> {
    samples
        .par_iter()
        .map(|value| (value * 255.0).round().clamp(0.0, 255.0) as u8)
        .collect()
}

fn native_bytes(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_ne_bytes())
        .collect()
}

fn image_error(path: &Path, error: image::ImageError) -> IoError {
    IoError::decode(path, error)
}

fn write_png(
    path: &Path,
    source: &Source,
    depth: Depth,
    samples: &[f32],
    icc: Vec<u8>,
) -> Result<(), IoError> {
    let file = File::create(path).map_err(IoError::read(path))?;
    let mut encoder = PngEncoder::new(BufWriter::new(file));
    encoder
        .set_icc_profile(icc)
        .map_err(|e| IoError::unsupported(path, e.to_string()))?;
    let (width, height) = (source.width as u32, source.height as u32);
    match depth {
        Depth::Sixteen => encoder.write_image(
            &native_bytes(&to_u16(samples)),
            width,
            height,
            ExtendedColorType::Rgb16,
        ),
        _ => encoder.write_image(&to_u8(samples), width, height, ExtendedColorType::Rgb8),
    }
    .map_err(|e| image_error(path, e))
}

fn write_jpeg(
    path: &Path,
    source: &Source,
    samples: &[f32],
    quality: u8,
    icc: Vec<u8>,
) -> Result<(), IoError> {
    let file = File::create(path).map_err(IoError::read(path))?;
    let mut encoder = JpegEncoder::new_with_quality(BufWriter::new(file), quality);
    encoder
        .set_icc_profile(icc)
        .map_err(|e| IoError::unsupported(path, e.to_string()))?;
    encoder
        .write_image(
            &to_u8(samples),
            source.width as u32,
            source.height as u32,
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| image_error(path, e))
}

fn fits_cards(source: &Source) -> Vec<Card> {
    let (mode, frames) = source
        .note
        .as_deref()
        .and_then(parse_result_note)
        .unwrap_or_else(|| ("STARY".to_string(), 0));
    vec![
        Card::Text(
            "STRYTYPE",
            StaryKind::StackResult.keyword_value().to_string(),
        ),
        Card::Number(keyword::STACK_COUNT, f64::from(frames)),
        Card::Text("STRYMODE", mode),
    ]
}

/// Makes a file of another kind from a result (a TIFF of 32-bit floats the program saved), keeping
/// the result as it is.
pub fn export_result(
    source_path: &Path,
    target: &Path,
    export: &Export,
) -> Result<(), ExportError> {
    export.check()?;
    let mut source = read_source(source_path)?;
    let source_profile = source
        .icc
        .as_deref()
        .and_then(|bytes| Profile::from_icc(bytes).ok())
        .unwrap_or_else(|| Profile::of_space(Space::Srgb, true));
    let plan = plan_of(export, &source_profile)?;
    let mut samples = std::mem::take(&mut source.samples);
    apply(&plan, &mut samples);
    let icc = plan.profile.to_icc();
    let note = source
        .note
        .clone()
        .unwrap_or_else(|| result_note("STARY", 0));
    let (width, height) = (source.width, source.height);
    match (export.container, export.depth) {
        (Container::Tiff, Depth::Float) => {
            write_tiff_data::<tiff::encoder::colortype::RGB32Float>(
                target, width, height, &samples, &note, &icc,
            )?;
        }
        (Container::Tiff, Depth::Sixteen) => {
            write_tiff_data::<tiff::encoder::colortype::RGB16>(
                target,
                width,
                height,
                &to_u16(&samples),
                &note,
                &icc,
            )?;
        }
        (Container::Tiff, _) => {
            write_tiff_data::<tiff::encoder::colortype::RGB8>(
                target,
                width,
                height,
                &to_u8(&samples),
                &note,
                &icc,
            )?;
        }
        (Container::Png, depth) => write_png(target, &source, depth, &samples, icc)?,
        (Container::Jpeg, _) => write_jpeg(target, &source, &samples, export.quality, icc)?,
        (Container::Fits, Depth::Float) => {
            write_fits_rgb_f32(target, width, height, &samples, &fits_cards(&source))?;
        }
        (Container::Fits, _) => {
            write_fits_rgb_u16(
                target,
                width,
                height,
                &to_u16(&samples),
                &fits_cards(&source),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use stary_core::{LinearRgbF32, ResultColor};

    use super::*;
    use crate::{
        fits::read_normalized,
        rgb_out::{OutputFormat, write_result},
    };

    const WIDTH: usize = 24;
    const HEIGHT: usize = 16;

    fn samples() -> Vec<f32> {
        (0..WIDTH * HEIGHT * 3)
            .map(|index| {
                0.01 + 0.0003 * (index % 977) as f32 + if index % 301 == 0 { 0.7 } else { 0.0 }
            })
            .collect()
    }

    fn result(dir: &Path, color: ResultColor) -> std::path::PathBuf {
        let path = dir.join("result.tif");
        let image = LinearRgbF32 {
            width: WIDTH,
            height: HEIGHT,
            samples: samples(),
        };
        write_result(&path, &image, OutputFormat::Tiff32, "Stacking", 12, &color).unwrap();
        path
    }

    fn export(container: Container, depth: Depth) -> Export {
        Export {
            container,
            depth,
            space: None,
            tone: Tone::Linear,
            quality: 90,
        }
    }

    const CAMERA: [[f32; 3]; 3] = [
        [1.2582287, 0.052999634, -0.3112283],
        [-0.11689235, 1.5896847, -0.4727924],
        [-0.09838579, -0.50626373, 1.6046495],
    ];

    #[test]
    fn a_float_tiff_goes_to_fits_and_back_to_tiff_with_every_value_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::Camera(CAMERA));
        let fits = dir.path().join("a.fits");
        export_result(&source, &fits, &export(Container::Fits, Depth::Float)).unwrap();
        let tiff = dir.path().join("b.tif");
        export_result(&source, &tiff, &export(Container::Tiff, Depth::Float)).unwrap();
        let original = samples();
        let from_fits = read_normalized(&fits).unwrap().samples;
        let from_tiff = read_source(&tiff).unwrap().samples;
        assert_eq!(from_fits.len(), original.len());
        assert!(
            original
                .iter()
                .zip(&from_fits)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert!(
            original
                .iter()
                .zip(&from_tiff)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }

    #[test]
    fn what_the_note_says_goes_into_the_fits_header() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::LinearSrgb);
        let fits = dir.path().join("a.fits");
        export_result(&source, &fits, &export(Container::Fits, Depth::Float)).unwrap();
        let header = read_normalized(&fits).unwrap().header;
        assert_eq!(header.text("STRYMODE"), Some("Stacking"));
        assert_eq!(header.number(keyword::STACK_COUNT), Some(12.0));
    }

    #[test]
    fn the_color_profile_of_the_result_is_kept_when_the_colors_are() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::Camera(CAMERA));
        let target = dir.path().join("b.tif");
        export_result(&source, &target, &export(Container::Tiff, Depth::Float)).unwrap();
        let (before, after) = (
            read_source(&source).unwrap().icc.unwrap(),
            read_source(&target).unwrap().icc.unwrap(),
        );
        let (a, b) = (
            Profile::from_icc(&before).unwrap(),
            Profile::from_icc(&after).unwrap(),
        );
        assert_eq!(a.description, "STARY camera RGB linear");
        assert_eq!(a.to_xyz, b.to_xyz);
    }

    #[test]
    fn camera_colors_become_srgb_with_a_profile_that_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::Camera(CAMERA));
        let target = dir.path().join("srgb.tif");
        let wanted = Export {
            space: Some(Space::Srgb),
            ..export(Container::Tiff, Depth::Float)
        };
        export_result(&source, &target, &wanted).unwrap();
        let converted = read_source(&target).unwrap();
        // what the camera matrix says: linear sRGB = matrix x camera
        let original = samples();
        for pixel in 0..4 {
            let camera = &original[pixel * 3..pixel * 3 + 3];
            for (row, matrix_row) in CAMERA.iter().enumerate() {
                let expected: f32 = matrix_row.iter().zip(camera).map(|(m, c)| m * c).sum();
                assert!((converted.samples[pixel * 3 + row] - expected).abs() < 1e-4);
            }
        }
        assert_eq!(
            Profile::from_icc(&converted.icc.unwrap())
                .unwrap()
                .description,
            "STARY sRGB linear"
        );
    }

    #[test]
    fn a_wider_space_makes_the_numbers_of_saturated_colors_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pure.tif");
        let image = LinearRgbF32 {
            width: 2,
            height: 1,
            samples: vec![0.0, 0.8, 0.0, 0.2, 0.2, 0.2],
        };
        write_result(
            &path,
            &image,
            OutputFormat::Tiff32,
            "Trails",
            1,
            &ResultColor::LinearSrgb,
        )
        .unwrap();
        let target = dir.path().join("p3.tif");
        let wanted = Export {
            space: Some(Space::DisplayP3),
            ..export(Container::Tiff, Depth::Float)
        };
        export_result(&path, &target, &wanted).unwrap();
        let out = read_source(&target).unwrap().samples;
        assert!(out[1] < 0.8 && out[0] > 0.0, "{out:?}");
        assert!(
            (out[3] - 0.2).abs() < 1e-3
                && (out[4] - 0.2).abs() < 1e-3
                && (out[5] - 0.2).abs() < 1e-3
        );
    }

    #[test]
    fn the_display_curve_lifts_the_shadows_and_the_stretch_puts_the_background_near_a_quarter() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::LinearSrgb);
        let linear = dir.path().join("lin.tif");
        let encoded = dir.path().join("enc.tif");
        let stretched = dir.path().join("str.tif");
        let base = Export {
            space: Some(Space::Srgb),
            ..export(Container::Tiff, Depth::Sixteen)
        };
        export_result(&source, &linear, &base).unwrap();
        export_result(
            &source,
            &encoded,
            &Export {
                tone: Tone::Display,
                ..base
            },
        )
        .unwrap();
        export_result(
            &source,
            &stretched,
            &Export {
                tone: Tone::Stretch,
                depth: Depth::Eight,
                ..base
            },
        )
        .unwrap();
        let read = |path: &Path| image::ImageReader::open(path).unwrap().decode().unwrap();
        let (a, b) = (read(&linear).into_rgb16(), read(&encoded).into_rgb16());
        assert!(b.as_raw()[100] > a.as_raw()[100]);
        let eight = read(&stretched).into_rgb8();
        let mut values: Vec<u8> = eight.as_raw().clone();
        values.sort_unstable();
        let median = f32::from(values[values.len() / 2]) / 255.0;
        assert!((median - 0.25).abs() < 0.1, "{median}");
    }

    #[test]
    fn png_and_jpeg_carry_the_profile_and_have_the_size_of_the_result() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::LinearSrgb);
        for (container, depth, name) in [
            (Container::Png, Depth::Sixteen, "a.png"),
            (Container::Png, Depth::Eight, "b.png"),
            (Container::Jpeg, Depth::Eight, "c.jpg"),
        ] {
            let target = dir.path().join(name);
            let wanted = Export {
                space: Some(Space::AdobeRgb),
                tone: Tone::Display,
                ..export(container, depth)
            };
            export_result(&source, &target, &wanted).unwrap();
            let mut decoder = image::ImageReader::open(&target)
                .unwrap()
                .into_decoder()
                .unwrap();
            let icc = image::ImageDecoder::icc_profile(&mut decoder)
                .unwrap()
                .unwrap();
            assert_eq!(
                Profile::from_icc(&icc).unwrap().description,
                "STARY Adobe RGB"
            );
            assert_eq!(
                image::ImageDecoder::dimensions(&decoder),
                (WIDTH as u32, HEIGHT as u32)
            );
        }
    }

    #[test]
    fn eight_bit_and_sixteen_bit_tiffs_have_their_depth() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::LinearSrgb);
        for (depth, bits) in [(Depth::Eight, 8), (Depth::Sixteen, 16)] {
            let target = dir.path().join(format!("t{bits}.tif"));
            export_result(&source, &target, &export(Container::Tiff, depth)).unwrap();
            assert_eq!(
                crate::probe_file(&target).unwrap().details.bit_depth,
                Some(bits)
            );
        }
    }

    #[test]
    fn a_sixteen_bit_fits_keeps_what_sixteen_bits_can() {
        let dir = tempfile::tempdir().unwrap();
        let source = result(dir.path(), ResultColor::LinearSrgb);
        let target = dir.path().join("s.fits");
        export_result(&source, &target, &export(Container::Fits, Depth::Sixteen)).unwrap();
        let read = read_normalized(&target).unwrap().samples;
        for (written, original) in read.iter().zip(samples()) {
            assert!((written / FULL_SCALE - original.clamp(0.0, 1.0)).abs() < 1.0 / FULL_SCALE);
        }
    }

    #[test]
    fn what_a_format_cannot_hold_is_refused() {
        let ok = export(Container::Tiff, Depth::Float);
        assert!(ok.check().is_ok());
        assert!(export(Container::Jpeg, Depth::Sixteen).check().is_err());
        assert!(export(Container::Png, Depth::Float).check().is_err());
        assert!(export(Container::Fits, Depth::Eight).check().is_err());
        assert!(
            Export {
                tone: Tone::Display,
                ..ok
            }
            .check()
            .is_err()
        );
        assert!(
            Export {
                space: Some(Space::Srgb),
                ..export(Container::Fits, Depth::Float)
            }
            .check()
            .is_err()
        );
        assert!(
            Export {
                quality: 0,
                ..export(Container::Jpeg, Depth::Eight)
            }
            .check()
            .is_err()
        );
        assert!(export(Container::Jpeg, Depth::Eight).loses_detail());
        assert!(!ok.loses_detail());
    }
}
