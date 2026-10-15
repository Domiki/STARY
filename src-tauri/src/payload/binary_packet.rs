use serde::Serialize;

use super::command_error::CommandError;

pub const HEADER_LEN_BYTES: usize = 4;

/// A JSON header and then bytes, the way the screen reads them: the length of the header (4 bytes,
/// little endian), the header, the bytes.
pub fn pack(header: &impl Serialize, body: &[u8]) -> Result<Vec<u8>, CommandError> {
    let header_json = serde_json::to_vec(header).map_err(CommandError::internal)?;
    let header_len = u32::try_from(header_json.len()).map_err(CommandError::internal)?;
    let mut bytes = Vec::with_capacity(HEADER_LEN_BYTES + header_json.len() + body.len());
    bytes.extend_from_slice(&header_len.to_le_bytes());
    bytes.extend_from_slice(&header_json);
    bytes.extend_from_slice(body);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Header {
        width: u32,
    }

    #[test]
    fn the_packet_starts_with_the_length_of_the_header() {
        let bytes = pack(&Header { width: 3 }, &[9, 8]).unwrap();
        let header_len = u32::from_le_bytes(bytes[..HEADER_LEN_BYTES].try_into().unwrap()) as usize;
        assert_eq!(
            &bytes[HEADER_LEN_BYTES..HEADER_LEN_BYTES + header_len],
            br#"{"width":3}"#
        );
        assert_eq!(&bytes[bytes.len() - 2..], &[9, 8]);
    }
}
