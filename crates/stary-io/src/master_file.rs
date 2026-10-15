use std::path::Path;

use stary_calib::{Master, SensorGeometry};
use stary_core::{Frame, PixelLayout, StaryKind, stary_kind::keyword};

use crate::{
    error::IoError,
    fits::{Card, Pixels, read_normalized, stary_cfa::layout_cards, write_fits},
};

const KEY_BLACK: &str = "STRYBLK";
const KEY_MAXIMUM: &str = "STRYMAX";
const FORMAT_VERSION: f64 = 1.0;
const COLOR_PLANES: usize = 3;
const FULL_SCALE: f32 = 65535.0;

fn layout_card_list(layout: &PixelLayout) -> Vec<Card> {
    match layout {
        PixelLayout::Cfa(pattern) => layout_cards(pattern)
            .into_iter()
            .map(|(key, value)| Card::Text(key, value))
            .collect(),
        PixelLayout::Rgb | PixelLayout::Mono => Vec::new(),
    }
}

fn into_planes<T: Copy>(interleaved: &[T], channels: usize) -> Vec<T> {
    if channels == 1 {
        return interleaved.to_vec();
    }
    (0..channels)
        .flat_map(|plane| interleaved.iter().skip(plane).step_by(channels).copied())
        .collect()
}

pub fn write_master(path: &Path, master: &Master, kind: StaryKind) -> Result<(), IoError> {
    let mut cards = vec![
        Card::Text("STRYTYPE", kind.keyword_value().to_string()),
        Card::Number("STRYVER", FORMAT_VERSION),
        Card::Number(keyword::STACK_COUNT, f64::from(master.count)),
        Card::Number(KEY_BLACK, f64::from(master.black)),
    ];
    cards.extend(layout_card_list(&master.geometry.layout));
    let geometry = &master.geometry;
    let channels = geometry.layout.channels();
    write_fits(
        path,
        geometry.width,
        geometry.height,
        channels,
        &Pixels::F32(&into_planes(&master.samples, channels)),
        &cards,
    )
}

pub fn write_calibrated_light(path: &Path, frame: &Frame) -> Result<(), IoError> {
    let mut cards = vec![
        Card::Flag(keyword::CALIBRATED, true),
        Card::Number("STRYVER", FORMAT_VERSION),
        Card::Number(KEY_BLACK, f64::from(frame.black)),
        Card::Number(KEY_MAXIMUM, f64::from(frame.maximum)),
    ];
    cards.extend(layout_card_list(&frame.layout));
    let channels = frame.layout.channels();
    write_fits(
        path,
        frame.width,
        frame.height,
        channels,
        &Pixels::U16(&into_planes(&frame.samples, channels)),
        &cards,
    )
}

/// What to take as the black level of a master that STARY did not write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalBlack {
    /// The master has had its offset removed already (a flat minus its bias).
    Zero,
    /// The master still holds its offset (a bias or a dark), which is about its median.
    Median,
}

fn median_of(samples: &[f32]) -> f32 {
    let mut sample: Vec<f32> = samples.iter().step_by(61).copied().collect();
    if sample.is_empty() {
        return 0.0;
    }
    let middle = sample.len() / 2;
    let (_, value, _) = sample.select_nth_unstable_by(middle, f32::total_cmp);
    *value
}

/// Floating point masters from other programs are often normalised to 0..1. They are brought to
/// the 16-bit scale the frames use.
fn bring_to_frame_scale(samples: &mut [f32]) {
    let highest = samples
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(0.0, f32::max);
    if highest <= 1.0 {
        samples.iter_mut().for_each(|value| *value *= FULL_SCALE);
    }
}

/// Reads a master that may not come from STARY. When the header does not describe the sensor
/// layout, `fallback_layout` (taken from the lights) is used. A master without STARY's black level
/// gets the one `external_black` says.
pub fn read_master_lenient(
    path: &Path,
    fallback_layout: Option<PixelLayout>,
    external_black: ExternalBlack,
) -> Result<(Master, Option<StaryKind>), IoError> {
    let mut fits = read_normalized(path)?;
    let layout = fits.layout.or(fallback_layout).unwrap_or(PixelLayout::Mono);
    let geometry = SensorGeometry {
        width: fits.geometry.width,
        height: fits.geometry.height,
        layout,
    };
    let stated_black = fits.header.number(KEY_BLACK);
    if stated_black.is_none() && fits.geometry.bitpix < 0 {
        bring_to_frame_scale(&mut fits.samples);
    }
    let black = match (stated_black, external_black) {
        (Some(value), _) => value as u16,
        (None, ExternalBlack::Zero) => 0,
        (None, ExternalBlack::Median) => median_of(&fits.samples).round() as u16,
    };
    let count = fits
        .header
        .number(keyword::STACK_COUNT)
        .map_or(1, |value| value as u32);
    let kind = fits.header.text(keyword::KIND).and_then(StaryKind::parse);
    let offset_removed = stated_black.is_none()
        && (fits
            .header
            .history_mentions("calibrated with a master bias")
            || fits
                .header
                .history_mentions("calibrated with a master dark"));
    let master = Master::new(geometry, black, count, fits.samples)
        .map_err(|error| IoError::decode(path, error))?
        .with_offset_removed(offset_removed);
    Ok((master, kind))
}

const _: () = assert!(COLOR_PLANES == 3);

#[cfg(test)]
mod tests {
    use super::*;

    fn read_master(path: &Path) -> Result<(Master, Option<StaryKind>), IoError> {
        read_master_lenient(path, None, ExternalBlack::Zero)
    }
    use stary_core::{BayerPattern, CfaPattern};

    fn geometry(layout: PixelLayout) -> SensorGeometry {
        SensorGeometry {
            width: 6,
            height: 6,
            layout,
        }
    }

    fn bayer(pattern: BayerPattern) -> PixelLayout {
        PixelLayout::Cfa(CfaPattern::Bayer(pattern))
    }

    #[test]
    fn a_bayer_master_survives_a_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master_dark.fits");
        let samples: Vec<f32> = (0..36).map(|index| 1000.0 + index as f32 * 0.5).collect();
        let master = Master::new(geometry(bayer(BayerPattern::Grbg)), 1022, 20, samples).unwrap();
        write_master(&path, &master, StaryKind::MasterDark).unwrap();
        let (read, kind) = read_master(&path).unwrap();
        assert_eq!(read, master);
        assert_eq!(kind, Some(StaryKind::MasterDark));
    }

    #[test]
    fn a_foreign_master_without_a_layout_is_mono_unless_the_lights_say_otherwise() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foreign.fits");
        let samples = vec![500.0f32; 36];
        crate::fits::write_fits(&path, 6, 6, 1, &Pixels::F32(&samples), &[]).unwrap();
        assert_eq!(
            read_master(&path).unwrap().0.geometry.layout,
            PixelLayout::Mono
        );
        let layout = bayer(BayerPattern::Rggb);
        let (master, kind) = read_master_lenient(&path, Some(layout), ExternalBlack::Zero).unwrap();
        assert_eq!(master.geometry.layout, layout);
        assert_eq!((master.black, kind), (0, None));
    }

    #[test]
    fn an_xtrans_master_keeps_its_matrix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master_flat.fits");
        let mut matrix = [[1u8; 6]; 6];
        matrix[0][0] = 2;
        matrix[3][4] = 0;
        let layout = PixelLayout::Cfa(CfaPattern::XTrans(matrix));
        let master = Master::new(geometry(layout), 1024, 7, vec![2000.0; 36]).unwrap();
        write_master(&path, &master, StaryKind::MasterFlat).unwrap();
        assert_eq!(read_master(&path).unwrap().0.geometry.layout, layout);
    }

    #[test]
    fn a_color_master_is_stored_in_planes_and_comes_back_interleaved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master_rgb.fits");
        let samples: Vec<f32> = (0..108).map(|index| index as f32).collect();
        let master = Master::new(geometry(PixelLayout::Rgb), 0, 3, samples).unwrap();
        write_master(&path, &master, StaryKind::MasterFlat).unwrap();
        assert_eq!(read_master(&path).unwrap().0, master);
    }

    #[test]
    fn a_calibrated_light_is_read_back_as_the_same_frame() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("light.fits");
        let frame = Frame {
            width: 6,
            height: 6,
            layout: bayer(BayerPattern::Rggb),
            black: 1024,
            maximum: 16383,
            samples: (0..36).map(|index| 1024 + index as u16).collect(),
        };
        write_calibrated_light(&path, &frame).unwrap();
        let info = crate::probe_file(&path).unwrap();
        assert!(info.details.calibrated);
        assert_eq!((info.details.width, info.details.height), (6, 6));
        assert_eq!(crate::decode_frame(&path).unwrap(), frame);
    }

    #[test]
    fn a_calibrated_color_light_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("light_rgb.fits");
        let frame = Frame {
            width: 4,
            height: 2,
            layout: PixelLayout::Rgb,
            black: 0,
            maximum: 65535,
            samples: (0..24).map(|index| index * 1000).collect(),
        };
        write_calibrated_light(&path, &frame).unwrap();
        assert_eq!(crate::decode_frame(&path).unwrap(), frame);
    }

    #[test]
    fn a_normalised_float_master_from_another_program_is_brought_to_the_frame_scale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("siril_dark.fits");
        let samples: Vec<f32> = (0..36).map(|index| 0.0155 + index as f32 * 1e-6).collect();
        crate::fits::write_fits(&path, 6, 6, 1, &Pixels::F32(&samples), &[]).unwrap();
        let (master, _) = read_master_lenient(&path, None, ExternalBlack::Median).unwrap();
        assert!(
            (master.samples[0] - 1015.8).abs() < 0.5,
            "{}",
            master.samples[0]
        );
        assert_eq!(master.black, 1016);
        let (flat, _) = read_master_lenient(&path, None, ExternalBlack::Zero).unwrap();
        assert_eq!(flat.black, 0);
    }

    #[test]
    fn a_master_in_counts_is_not_scaled_and_stary_masters_keep_their_black() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("counts.fits");
        crate::fits::write_fits(&path, 6, 6, 1, &Pixels::F32(&[1500.0; 36]), &[]).unwrap();
        let (counts, _) = read_master_lenient(&path, None, ExternalBlack::Median).unwrap();
        assert_eq!((counts.samples[0], counts.black), (1500.0, 1500));

        let own = dir.path().join("own.fits");
        let master = Master::new(geometry(PixelLayout::Mono), 1022, 4, vec![0.5; 36]).unwrap();
        write_master(&own, &master, StaryKind::MasterDark).unwrap();
        let (read, _) = read_master_lenient(&own, None, ExternalBlack::Median).unwrap();
        assert_eq!((read.samples[0], read.black), (0.5, 1022));
    }
}
