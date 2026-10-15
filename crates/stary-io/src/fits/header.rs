use std::{io::Read, path::Path};

use crate::error::IoError;

const BLOCK_LEN: usize = 2880;
const CARD_LEN: usize = 80;
const KEY_LEN: usize = 8;
const MAX_BLOCKS: usize = 1000;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Flag(bool),
    Number(f64),
}

#[derive(Debug, Clone, Default)]
pub struct Header {
    cards: Vec<(String, Value)>,
    history: Vec<String>,
    pub data_offset: u64,
}

impl Header {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.cards
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        match self.get(key)? {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// Whether a HISTORY line mentions `phrase` (ignoring case).
    pub fn history_mentions(&self, phrase: &str) -> bool {
        let phrase = phrase.to_ascii_lowercase();
        self.history
            .iter()
            .any(|line| line.to_ascii_lowercase().contains(&phrase))
    }

    pub fn flag(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            Value::Flag(flag) => Some(*flag),
            _ => None,
        }
    }
}

fn parse_quoted(field: &str) -> Option<String> {
    let mut chars = field.strip_prefix('\'')?.chars().peekable();
    let mut text = String::new();
    while let Some(character) = chars.next() {
        match (character, chars.peek()) {
            ('\'', Some('\'')) => {
                text.push('\'');
                chars.next();
            }
            ('\'', _) => return Some(text.trim_end().to_string()),
            _ => text.push(character),
        }
    }
    None
}

fn parse_value(field: &str) -> Option<Value> {
    let field = field.trim_start();
    if field.starts_with('\'') {
        return parse_quoted(field).map(Value::Text);
    }
    let token = field.split('/').next()?.trim();
    match token {
        "T" => Some(Value::Flag(true)),
        "F" => Some(Value::Flag(false)),
        _ => token
            .replace(['D', 'd'], "E")
            .parse::<f64>()
            .ok()
            .map(Value::Number),
    }
}

fn parse_card(card: &[u8]) -> Option<(String, Value)> {
    let text = String::from_utf8_lossy(card);
    let key = text.get(..KEY_LEN)?.trim_end();
    let has_value = text.get(KEY_LEN..KEY_LEN + 2) == Some("= ");
    if key.is_empty() || !has_value {
        return None;
    }
    parse_value(text.get(KEY_LEN + 2..)?).map(|value| (key.to_string(), value))
}

fn is_end_card(card: &[u8]) -> bool {
    card.starts_with(b"END") && card[3..KEY_LEN].iter().all(|&byte| byte == b' ')
}

fn history_text(card: &[u8]) -> Option<String> {
    card.starts_with(b"HISTORY")
        .then(|| String::from_utf8_lossy(&card[KEY_LEN..]).trim().to_string())
}

pub fn read_header<R: Read>(path: &Path, reader: &mut R) -> Result<Header, IoError> {
    let mut cards = Vec::new();
    let mut history = Vec::new();
    let mut block = [0u8; BLOCK_LEN];
    for index in 1..=MAX_BLOCKS {
        reader.read_exact(&mut block).map_err(IoError::read(path))?;
        let mut parsed = Vec::new();
        for card in block.as_chunks::<CARD_LEN>().0 {
            if is_end_card(card) {
                cards.extend(parsed);
                return Ok(Header {
                    cards,
                    history,
                    data_offset: (index * BLOCK_LEN) as u64,
                });
            }
            history.extend(history_text(card));
            parsed.extend(parse_card(card));
        }
        cards.extend(parsed);
    }
    Err(IoError::decode(path, "FITS header has no END card"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(text: &str) -> Vec<u8> {
        format!("{text:<80}").into_bytes()
    }

    fn header_bytes(cards: &[&str]) -> Vec<u8> {
        let mut bytes: Vec<u8> = cards.iter().flat_map(|text| card(text)).collect();
        bytes.extend(card("END"));
        bytes.resize(bytes.len().div_ceil(BLOCK_LEN) * BLOCK_LEN, b' ');
        bytes
    }

    #[test]
    fn parses_numbers_text_and_flags() {
        let bytes = header_bytes(&[
            "SIMPLE  =                    T",
            "BITPIX  =                   16 / bits",
            "BAYERPAT= 'RGGB    '           / pattern",
            "OBJECT  = 'it''s'",
        ]);
        let header = read_header(Path::new("t.fits"), &mut bytes.as_slice()).unwrap();
        assert_eq!(header.flag("SIMPLE"), Some(true));
        assert_eq!(header.number("BITPIX"), Some(16.0));
        assert_eq!(header.text("BAYERPAT"), Some("RGGB"));
        assert_eq!(header.text("OBJECT"), Some("it's"));
        assert_eq!(header.data_offset, BLOCK_LEN as u64);
    }

    #[test]
    fn history_lines_can_be_searched() {
        let bytes = header_bytes(&[
            "SIMPLE  =                    T",
            "HISTORY Calibrated with a master bias",
            "HISTORY stacked",
        ]);
        let header = read_header(Path::new("t.fits"), &mut bytes.as_slice()).unwrap();
        assert!(header.history_mentions("master BIAS"));
        assert!(!header.history_mentions("master dark"));
    }

    #[test]
    fn missing_end_card_is_an_error() {
        let bytes = vec![b' '; BLOCK_LEN];
        assert!(read_header(Path::new("t.fits"), &mut bytes.as_slice()).is_err());
    }
}
