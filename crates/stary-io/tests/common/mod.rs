#![allow(dead_code)]

use std::path::PathBuf;

use tempfile::TempDir;

const BLOCK_LEN: usize = 2880;

fn card(text: &str) -> Vec<u8> {
    format!("{text:<80}").into_bytes()
}

pub fn fits_bytes(cards: &[String], data: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = cards.iter().flat_map(|text| card(text)).collect();
    bytes.extend(card("END"));
    bytes.resize(bytes.len().div_ceil(BLOCK_LEN) * BLOCK_LEN, b' ');
    bytes.extend_from_slice(data);
    bytes.resize(bytes.len().div_ceil(BLOCK_LEN) * BLOCK_LEN, 0);
    bytes
}

pub fn header_cards(bitpix: i32, axes: &[usize], extra: &[&str]) -> Vec<String> {
    let mut cards = vec![
        "SIMPLE  =                    T".to_string(),
        format!("BITPIX  = {bitpix:>20}"),
        format!("NAXIS   = {:>20}", axes.len()),
    ];
    cards.extend(
        axes.iter()
            .enumerate()
            .map(|(index, length)| format!("NAXIS{}  = {length:>20}", index + 1)),
    );
    if bitpix == 16 {
        cards.push("BZERO   =                32768".to_string());
        cards.push("BSCALE  =                    1".to_string());
    }
    cards.extend(extra.iter().map(|text| text.to_string()));
    cards
}

pub fn write_file(dir: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}
