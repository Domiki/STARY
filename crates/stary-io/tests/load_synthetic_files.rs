use std::path::{Path, PathBuf};

use image::{ImageBuffer, Luma, Rgb};
use stary_core::{BayerPattern, CfaPattern, PixelLayout, SourceFormat, StaryKind};
use stary_io::{probe_file, read_preview};
use tempfile::TempDir;

mod common;

use common::fits_bytes;

fn u16_fits_data(width: usize, height: usize, value_at: impl Fn(usize, usize) -> u16) -> Vec<u8> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| ((i32::from(value_at(x, y)) - 32768) as i16).to_be_bytes())
        .collect()
}

fn write_fits(dir: &TempDir, name: &str, extra: &[&str], width: usize, height: usize) -> PathBuf {
    let data = u16_fits_data(width, height, |x, y| 1000 + ((x * 7 + y * 13) % 200) as u16);
    let mut cards = vec![
        "SIMPLE  =                    T".to_string(),
        "BITPIX  =                   16".to_string(),
        "NAXIS   =                    2".to_string(),
        format!("NAXIS1  = {width:>20}"),
        format!("NAXIS2  = {height:>20}"),
        "BZERO   =                32768".to_string(),
        "BSCALE  =                    1".to_string(),
    ];
    cards.extend(extra.iter().map(|text| text.to_string()));
    let path = dir.path().join(name);
    std::fs::write(&path, fits_bytes(&cards, &data)).unwrap();
    path
}

fn assert_preview_is_valid(path: &Path, max_edge: u32) -> (u32, u32) {
    let preview = read_preview(path, max_edge).unwrap();
    assert_eq!(
        preview.rgb8.len(),
        preview.width as usize * preview.height as usize * 3
    );
    assert!(preview.width.max(preview.height) <= max_edge);
    (preview.width, preview.height)
}

#[test]
fn probes_mono_fits_with_metadata() {
    let dir = TempDir::new().unwrap();
    let path = write_fits(
        &dir,
        "mono.fits",
        &[
            "EXPTIME =                 30.0",
            "DATE-OBS= '2026-10-01T22:15:03'",
            "GAIN    =                  100",
        ],
        64,
        48,
    );
    let info = probe_file(&path).unwrap();
    assert_eq!(info.format, SourceFormat::Fits);
    assert_eq!((info.details.width, info.details.height), (64, 48));
    assert_eq!(info.details.layout, PixelLayout::Mono);
    assert_eq!(info.details.bit_depth, Some(16));
    assert_eq!(info.details.exposure_s, Some(30.0));
    assert_eq!(info.details.iso, Some(100));
    assert_eq!(
        info.details.captured_at.as_deref(),
        Some("2026-10-01T22:15:03")
    );
    assert_eq!(info.details.stary_kind, None);
}

#[test]
fn probes_bayer_fits_and_reads_stary_keywords() {
    let dir = TempDir::new().unwrap();
    let path = write_fits(
        &dir,
        "master_dark.fits",
        &[
            "BAYERPAT= 'RGGB    '",
            "STRYTYPE= 'MASTER_DARK'",
            "STACKCNT=                   20",
            "STRYCAL =                    T",
        ],
        32,
        32,
    );
    let info = probe_file(&path).unwrap();
    assert_eq!(
        info.details.layout,
        PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb))
    );
    assert_eq!(info.details.stary_kind, Some(StaryKind::MasterDark));
    assert_eq!(info.details.stack_count, Some(20));
    assert!(info.details.calibrated);
}

#[test]
fn fits_preview_halves_bayer_resolution_and_fits_max_edge() {
    let dir = TempDir::new().unwrap();
    let bayer = write_fits(&dir, "bayer.fits", &["BAYERPAT= 'RGGB    '"], 400, 200);
    assert_eq!(assert_preview_is_valid(&bayer, 1000), (200, 100));
    let mono = write_fits(&dir, "mono.fits", &[], 400, 200);
    assert_eq!(assert_preview_is_valid(&mono, 100), (100, 50));
}

#[test]
fn probes_and_previews_png_and_jpeg() {
    let dir = TempDir::new().unwrap();
    let png = dir.path().join("frame.png");
    ImageBuffer::from_fn(120, 80, |x, y| {
        Rgb([(x % 256) as u8, (y % 256) as u8, 40u8])
    })
    .save(&png)
    .unwrap();
    let jpeg = dir.path().join("frame.jpg");
    ImageBuffer::from_fn(120, 80, |x, _| Rgb([(x * 2) as u8, 90u8, 30u8]))
        .save(&jpeg)
        .unwrap();

    let png_info = probe_file(&png).unwrap();
    assert_eq!(png_info.format, SourceFormat::Png);
    assert_eq!((png_info.details.width, png_info.details.height), (120, 80));
    assert_eq!(png_info.details.layout, PixelLayout::Rgb);
    assert_eq!(png_info.details.bit_depth, Some(8));
    assert_eq!(probe_file(&jpeg).unwrap().format, SourceFormat::Jpeg);
    assert_eq!(assert_preview_is_valid(&png, 60), (60, 40));
    assert_eq!(assert_preview_is_valid(&jpeg, 500), (120, 80));
}

#[test]
fn probes_sixteen_bit_gray_tiff() {
    let dir = TempDir::new().unwrap();
    let tiff = dir.path().join("gray.tiff");
    ImageBuffer::<Luma<u16>, _>::from_fn(50, 30, |x, y| Luma([((x + y) * 500) as u16]))
        .save(&tiff)
        .unwrap();
    let info = probe_file(&tiff).unwrap();
    assert_eq!(info.format, SourceFormat::Tiff);
    assert_eq!(info.details.layout, PixelLayout::Mono);
    assert_eq!(info.details.bit_depth, Some(16));
}

#[test]
fn rejects_unknown_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, "just text, not an image").unwrap();
    let error = probe_file(&path).unwrap_err();
    assert_eq!(error.code(), "unsupported_file");
}
