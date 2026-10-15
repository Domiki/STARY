mod common;

use common::{fits_bytes, header_cards, write_file};
use image::{ImageBuffer, Luma, Rgb};
use stary_core::{BayerPattern, CfaPattern, PixelLayout};
use stary_io::decode_frame;
use tempfile::TempDir;

fn u16_data(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| ((i32::from(*value) - 32768) as i16).to_be_bytes())
        .collect()
}

fn f32_data(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

#[test]
fn a_bottom_up_fits_is_turned_upright_and_keeps_the_sensor_pattern() {
    let dir = TempDir::new().unwrap();
    // stored rows: 10,11 / 20,21 / 30,31 / 40,41 (first stored row is the bottom row)
    let values = [10, 11, 20, 21, 30, 31, 40, 41];
    let cards = header_cards(16, &[2, 4], &["BAYERPAT= 'RGGB    '"]);
    let path = write_file(&dir, "bayer.fits", &fits_bytes(&cards, &u16_data(&values)));
    let frame = decode_frame(&path).unwrap();
    assert_eq!(frame.samples, vec![40, 41, 30, 31, 20, 21, 10, 11]);
    // BAYERPAT describes the sensor as seen from the top, whatever the order of the rows
    assert_eq!(
        frame.layout,
        PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb))
    );
}

#[test]
fn an_xtrans_pattern_from_its_thirty_six_letters_is_used_as_written() {
    let dir = TempDir::new().unwrap();
    let pattern = "GGRGGBGGBGGRBRGRBGGGBGGRGGRGGBRBGBRG";
    let cards = header_cards(16, &[6, 6], &[&format!("BAYERPAT= '{pattern}'")]);
    let values: Vec<u16> = (0..36).collect();
    let path = write_file(&dir, "xtrans.fits", &fits_bytes(&cards, &u16_data(&values)));
    let frame = decode_frame(&path).unwrap();
    let PixelLayout::Cfa(CfaPattern::XTrans(matrix)) = frame.layout else {
        panic!("expected an X-Trans layout, got {:?}", frame.layout)
    };
    assert_eq!(matrix[0], [1, 1, 0, 1, 1, 2]);
    assert_eq!(matrix[5], [0, 2, 1, 2, 0, 1]);
    // the rows were stored bottom first
    assert_eq!(&frame.samples[..6], &[30, 31, 32, 33, 34, 35]);
}

#[test]
fn a_top_down_file_is_left_alone() {
    let dir = TempDir::new().unwrap();
    let cards = header_cards(
        16,
        &[2, 2],
        &["BAYERPAT= 'RGGB    '", "ROWORDER= 'TOP-DOWN'"],
    );
    let path = write_file(
        &dir,
        "top.fits",
        &fits_bytes(&cards, &u16_data(&[1, 2, 3, 4])),
    );
    let frame = decode_frame(&path).unwrap();
    assert_eq!(frame.samples, vec![1, 2, 3, 4]);
    assert_eq!(
        frame.layout,
        PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb))
    );
    assert_eq!((frame.black, frame.maximum), (0, 65535));
}

#[test]
fn a_float_color_fits_becomes_interleaved_sixteen_bit() {
    let dir = TempDir::new().unwrap();
    // 2x1 pixels, planes R, G, B, normalised floats
    let cards = header_cards(-32, &[2, 1, 3], &["ROWORDER= 'TOP-DOWN'"]);
    let data = f32_data(&[0.0, 1.0, 0.5, 0.5, 1.0, 0.0]);
    let path = write_file(&dir, "rgb.fits", &fits_bytes(&cards, &data));
    let frame = decode_frame(&path).unwrap();
    assert_eq!(frame.layout, PixelLayout::Rgb);
    assert_eq!(frame.samples, vec![0, 32768, 65535, 65535, 32768, 0]);
}

#[test]
fn a_plain_two_dimensional_fits_is_mono() {
    let dir = TempDir::new().unwrap();
    let cards = header_cards(16, &[2, 1], &["ROWORDER= 'TOP-DOWN'"]);
    let path = write_file(
        &dir,
        "mono.fits",
        &fits_bytes(&cards, &u16_data(&[100, 60000])),
    );
    let frame = decode_frame(&path).unwrap();
    assert_eq!(frame.layout, PixelLayout::Mono);
    assert_eq!(frame.samples, vec![100, 60000]);
}

#[test]
fn a_fits_with_an_odd_number_of_planes_is_refused() {
    let dir = TempDir::new().unwrap();
    let cards = header_cards(16, &[1, 1, 2], &[]);
    let path = write_file(&dir, "two.fits", &fits_bytes(&cards, &u16_data(&[1, 2])));
    assert_eq!(decode_frame(&path).unwrap_err().code(), "unsupported_file");
}

#[test]
fn a_png_is_made_linear() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("frame.png");
    ImageBuffer::from_fn(2, 1, |x, _| Rgb([if x == 0 { 0u8 } else { 255 }, 128, 0]))
        .save(&path)
        .unwrap();
    let frame = decode_frame(&path).unwrap();
    assert_eq!(frame.layout, PixelLayout::Rgb);
    assert_eq!((frame.width, frame.height), (2, 1));
    assert_eq!(&frame.samples[..3], &[0, frame.samples[1], 0]);
    // sRGB 128/255 is about 21.6% linear
    assert!((i32::from(frame.samples[1]) - 14160).abs() < 100);
    assert_eq!(frame.samples[3], 65535);
}

#[test]
fn a_sixteen_bit_gray_tiff_is_mono() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("gray.tiff");
    ImageBuffer::<Luma<u16>, _>::from_fn(3, 2, |x, y| Luma([((x + y) * 10000) as u16]))
        .save(&path)
        .unwrap();
    let frame = decode_frame(&path).unwrap();
    assert_eq!(frame.layout, PixelLayout::Mono);
    assert_eq!(frame.samples.len(), 6);
    assert_eq!(frame.samples[0], 0);
    assert!(frame.samples[5] > frame.samples[1]);
}
