use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use crate::{error::IoError, fits::header::Header};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub width: usize,
    pub height: usize,
    pub planes: usize,
    pub bitpix: i32,
    pub bzero: f64,
    pub bscale: f64,
}

impl Geometry {
    pub const fn bytes_per_sample(&self) -> usize {
        self.bitpix.unsigned_abs() as usize / 8
    }

    pub const fn sample_count(&self) -> usize {
        self.width * self.height * self.planes
    }
}

fn dimension(header: &Header, key: &str) -> Option<usize> {
    header
        .number(key)
        .filter(|value| *value >= 1.0)
        .map(|value| value as usize)
}

pub fn geometry_of(path: &Path, header: &Header) -> Result<Geometry, IoError> {
    let axes = header.number("NAXIS").unwrap_or(0.0) as usize;
    let bitpix = header.number("BITPIX").unwrap_or(0.0) as i32;
    if !(axes == 2 || axes == 3) || ![8, 16, 32, -32, -64].contains(&bitpix) {
        return Err(IoError::unsupported(
            path,
            format!("FITS NAXIS={axes} BITPIX={bitpix}"),
        ));
    }
    let width = dimension(header, "NAXIS1");
    let height = dimension(header, "NAXIS2");
    let planes = if axes == 3 {
        dimension(header, "NAXIS3")
    } else {
        Some(1)
    };
    match (width, height, planes) {
        (Some(width), Some(height), Some(planes)) => Ok(Geometry {
            width,
            height,
            planes,
            bitpix,
            bzero: header.number("BZERO").unwrap_or(0.0),
            bscale: header.number("BSCALE").unwrap_or(1.0),
        }),
        _ => Err(IoError::decode(path, "invalid FITS axis lengths")),
    }
}

fn convert<const N: usize>(
    bytes: &[u8],
    geometry: &Geometry,
    value: impl Fn([u8; N]) -> f64 + Sync,
) -> Vec<f32> {
    bytes
        .as_chunks::<N>()
        .0
        .iter()
        .map(|chunk| (geometry.bzero + geometry.bscale * value(*chunk)) as f32)
        .collect()
}

fn decode_samples(bytes: &[u8], geometry: &Geometry) -> Vec<f32> {
    match geometry.bitpix {
        8 => convert::<1>(bytes, geometry, |[byte]| f64::from(byte)),
        16 => convert::<2>(bytes, geometry, |chunk| {
            f64::from(i16::from_be_bytes(chunk))
        }),
        32 => convert::<4>(bytes, geometry, |chunk| {
            f64::from(i32::from_be_bytes(chunk))
        }),
        -32 => convert::<4>(bytes, geometry, |chunk| {
            f64::from(f32::from_be_bytes(chunk))
        }),
        _ => convert::<8>(bytes, geometry, f64::from_be_bytes),
    }
}

pub fn read_physical_samples(
    path: &Path,
    header: &Header,
    geometry: &Geometry,
) -> Result<Vec<f32>, IoError> {
    let mut bytes = vec![0u8; geometry.sample_count() * geometry.bytes_per_sample()];
    File::open(path)
        .and_then(|mut file| {
            file.seek(SeekFrom::Start(header.data_offset))?;
            file.read_exact(&mut bytes)
        })
        .map_err(IoError::read(path))?;
    Ok(decode_samples(&bytes, geometry))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(bitpix: i32, bzero: f64) -> Geometry {
        Geometry {
            width: 2,
            height: 1,
            planes: 1,
            bitpix,
            bzero,
            bscale: 1.0,
        }
    }

    #[test]
    fn decodes_big_endian_sixteen_bit() {
        assert_eq!(
            decode_samples(&[0x01, 0x00, 0xFF, 0xFF], &geometry(16, 0.0)),
            vec![256.0, -1.0]
        );
    }

    #[test]
    fn sixteen_bit_with_the_unsigned_offset_gives_unsigned_values() {
        assert_eq!(
            decode_samples(&[0x80, 0x00, 0x7F, 0xFF], &geometry(16, 32768.0)),
            vec![0.0, 65535.0]
        );
    }

    #[test]
    fn decodes_big_endian_float() {
        let bytes = 1.5f32.to_be_bytes();
        assert_eq!(decode_samples(&bytes, &geometry(-32, 0.0)), vec![1.5]);
    }
}
