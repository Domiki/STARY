use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::{error::IoError, fits::RowOrder};

const BLOCK_LEN: usize = 2880;
const CARD_LEN: usize = 80;
const U16_ZERO: i32 = 32768;

pub enum Pixels<'a> {
    U16(&'a [u16]),
    F32(&'a [f32]),
}

#[derive(Clone)]
pub enum Card {
    Number(&'static str, f64),
    Text(&'static str, String),
    Flag(&'static str, bool),
}

fn render(card: &Card) -> String {
    match card {
        Card::Number(key, value) => format!("{key:<8}= {value:>20}"),
        Card::Text(key, value) => format!("{key:<8}= '{:<8}'", value.replace('\'', "''")),
        Card::Flag(key, value) => format!("{key:<8}= {:>20}", if *value { "T" } else { "F" }),
    }
}

fn header_bytes(
    width: usize,
    height: usize,
    planes: usize,
    bitpix: i32,
    extra: &[Card],
) -> Vec<u8> {
    let axes = if planes > 1 { 3 } else { 2 };
    let mut cards = vec![
        "SIMPLE  =                    T".to_string(),
        format!("BITPIX  = {bitpix:>20}"),
        format!("NAXIS   = {axes:>20}"),
        format!("NAXIS1  = {width:>20}"),
        format!("NAXIS2  = {height:>20}"),
    ];
    if planes > 1 {
        cards.push(format!("NAXIS3  = {planes:>20}"));
    }
    if bitpix == 16 {
        cards.push(format!("BZERO   = {U16_ZERO:>20}"));
        cards.push("BSCALE  =                    1".to_string());
    }
    cards.extend(extra.iter().map(render));
    cards.push("END".to_string());
    let mut bytes: Vec<u8> = cards
        .iter()
        .flat_map(|card| format!("{card:<CARD_LEN$}").into_bytes())
        .collect();
    bytes.resize(bytes.len().div_ceil(BLOCK_LEN) * BLOCK_LEN, b' ');
    bytes
}

fn write_pixels(writer: &mut impl Write, pixels: &Pixels) -> std::io::Result<usize> {
    const CHUNK: usize = 1 << 16;
    let mut buffer = Vec::with_capacity(CHUNK * 4);
    let mut written = 0;
    match pixels {
        Pixels::U16(samples) => {
            for chunk in samples.chunks(CHUNK) {
                buffer.clear();
                buffer.extend(
                    chunk
                        .iter()
                        .flat_map(|value| ((i32::from(*value) - U16_ZERO) as i16).to_be_bytes()),
                );
                writer.write_all(&buffer)?;
                written += buffer.len();
            }
        }
        Pixels::F32(samples) => {
            for chunk in samples.chunks(CHUNK) {
                buffer.clear();
                buffer.extend(chunk.iter().flat_map(|value| value.to_be_bytes()));
                writer.write_all(&buffer)?;
                written += buffer.len();
            }
        }
    }
    Ok(written)
}

fn with_row_order(order: RowOrder, extra: &[Card]) -> Vec<Card> {
    let mut cards = vec![Card::Text("ROWORDER", order.keyword_value().to_string())];
    cards.extend(extra.iter().cloned());
    cards
}

/// Writes top-down rows. `pixels` hold `planes` planes one after the other.
pub fn write_fits(
    path: &Path,
    width: usize,
    height: usize,
    planes: usize,
    pixels: &Pixels,
    extra: &[Card],
) -> Result<(), IoError> {
    let bitpix = match pixels {
        Pixels::U16(_) => 16,
        Pixels::F32(_) => -32,
    };
    let cards = with_row_order(RowOrder::TopDown, extra);
    let file = File::create(path).map_err(IoError::read(path))?;
    let mut writer = BufWriter::new(file);
    let result = (|| {
        writer.write_all(&header_bytes(width, height, planes, bitpix, &cards))?;
        let written = write_pixels(&mut writer, pixels)?;
        writer.write_all(&vec![0u8; (BLOCK_LEN - written % BLOCK_LEN) % BLOCK_LEN])?;
        writer.flush()
    })();
    result.map_err(IoError::read(path))
}

/// Writes interleaved RGB as three planes of 32-bit floats. FITS stores the bottom row first, so
/// the rows are written in reverse and the image appears upright in other programs.
pub fn write_fits_rgb_f32(
    path: &Path,
    width: usize,
    height: usize,
    interleaved: &[f32],
    extra: &[Card],
) -> Result<(), IoError> {
    let cards = with_row_order(RowOrder::BottomUp, extra);
    let file = File::create(path).map_err(IoError::read(path))?;
    let mut writer = BufWriter::new(file);
    let result = (|| {
        writer.write_all(&header_bytes(width, height, 3, -32, &cards))?;
        let mut written = 0;
        for plane in 0..3 {
            for row in (0..height).rev() {
                let line: Vec<u8> = (0..width)
                    .flat_map(|column| {
                        interleaved[(row * width + column) * 3 + plane].to_be_bytes()
                    })
                    .collect();
                writer.write_all(&line)?;
                written += line.len();
            }
        }
        writer.write_all(&vec![0u8; (BLOCK_LEN - written % BLOCK_LEN) % BLOCK_LEN])?;
        writer.flush()
    })();
    result.map_err(IoError::read(path))
}

/// Writes interleaved RGB as three planes of unsigned 16-bit integers, bottom row first.
pub fn write_fits_rgb_u16(
    path: &Path,
    width: usize,
    height: usize,
    interleaved: &[u16],
    extra: &[Card],
) -> Result<(), IoError> {
    let cards = with_row_order(RowOrder::BottomUp, extra);
    let file = File::create(path).map_err(IoError::read(path))?;
    let mut writer = BufWriter::new(file);
    let result = (|| {
        writer.write_all(&header_bytes(width, height, 3, 16, &cards))?;
        let mut written = 0;
        for plane in 0..3 {
            for row in (0..height).rev() {
                let line: Vec<u8> = (0..width)
                    .flat_map(|column| {
                        let value = interleaved[(row * width + column) * 3 + plane];
                        ((i32::from(value) - U16_ZERO) as i16).to_be_bytes()
                    })
                    .collect();
                writer.write_all(&line)?;
                written += line.len();
            }
        }
        writer.write_all(&vec![0u8; (BLOCK_LEN - written % BLOCK_LEN) % BLOCK_LEN])?;
        writer.flush()
    })();
    result.map_err(IoError::read(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_padded_to_whole_blocks_and_ends_with_end() {
        let bytes = header_bytes(
            10,
            5,
            1,
            -32,
            &[Card::Text("STRYTYPE", "MASTER_DARK".into())],
        );
        assert_eq!(bytes.len() % BLOCK_LEN, 0);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("STRYTYPE= 'MASTER_DARK'"));
        assert!(text.contains("END"));
    }

    #[test]
    fn unsigned_16_bit_uses_the_standard_offset() {
        let bytes = header_bytes(2, 2, 1, 16, &[]);
        assert!(
            String::from_utf8(bytes)
                .unwrap()
                .contains("BZERO   =                32768")
        );
    }
}
