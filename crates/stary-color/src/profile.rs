use crate::{
    curve::Curve,
    matrix::{Matrix, Vector},
    space::{D50, Space, camera_to_xyz_d50},
};

const TABLE_LEN: usize = 1024;
const HEADER_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("not an ICC profile")]
    NotIcc,
    #[error("the profile is not a matrix profile for RGB")]
    Unsupported,
    #[error("the profile is cut short")]
    Truncated,
}

/// An RGB color space as an ICC matrix profile keeps it: where its primaries lie (as XYZ with a D50
/// white) and how its values are encoded.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Linear RGB to XYZ (D50); the columns are the primaries.
    pub to_xyz: Matrix,
    pub curve: Curve,
    pub description: String,
}

impl Profile {
    /// A standard space, with its usual encoding or as linear light.
    pub fn of_space(space: Space, linear: bool) -> Self {
        Self {
            to_xyz: space.to_xyz_d50(),
            curve: if linear { Curve::Linear } else { space.curve() },
            description: if linear {
                format!("STARY {} linear", space.name())
            } else {
                format!("STARY {}", space.name())
            },
        }
    }

    /// The colors a camera records (white balanced, linear), from the matrix that turns them into
    /// linear sRGB.
    pub fn camera(camera_to_srgb: &[[f32; 3]; 3]) -> Self {
        Self {
            to_xyz: camera_to_xyz_d50(camera_to_srgb),
            curve: Curve::Linear,
            description: "STARY camera RGB linear".to_string(),
        }
    }

    /// The same primaries, encoded another way.
    pub fn with_curve(&self, curve: Curve, description: String) -> Self {
        Self {
            to_xyz: self.to_xyz,
            curve,
            description,
        }
    }

    pub fn to_icc(&self) -> Vec<u8> {
        let mut writer = Writer::default();
        writer.tag(*b"desc", description_tag(&self.description));
        writer.tag(*b"cprt", text_tag("No copyright, made by STARY"));
        writer.tag(*b"wtpt", xyz_tag(D50));
        for (index, name) in [*b"rXYZ", *b"gXYZ", *b"bXYZ"].into_iter().enumerate() {
            writer.tag(
                name,
                xyz_tag([
                    self.to_xyz[0][index],
                    self.to_xyz[1][index],
                    self.to_xyz[2][index],
                ]),
            );
        }
        let curve = curve_tag(&self.curve);
        for name in [*b"rTRC", *b"gTRC", *b"bTRC"] {
            writer.tag(name, curve.clone());
        }
        writer.finish()
    }

    pub fn from_icc(bytes: &[u8]) -> Result<Self, ProfileError> {
        if bytes.len() < HEADER_LEN + 4 || &bytes[36..40] != b"acsp" {
            return Err(ProfileError::NotIcc);
        }
        if &bytes[16..20] != b"RGB " || &bytes[20..24] != b"XYZ " {
            return Err(ProfileError::Unsupported);
        }
        let count = be_u32(bytes, HEADER_LEN)? as usize;
        let find = |name: &[u8; 4]| -> Result<&[u8], ProfileError> {
            (0..count.min(256))
                .find_map(|index| {
                    let entry = HEADER_LEN + 4 + index * 12;
                    (bytes.get(entry..entry + 4)? == name).then(|| {
                        let offset = be_u32(bytes, entry + 4).ok()? as usize;
                        let size = be_u32(bytes, entry + 8).ok()? as usize;
                        bytes.get(offset..offset.checked_add(size)?)
                    })?
                })
                .ok_or(ProfileError::Unsupported)
        };
        let columns: Vec<Vector> = [b"rXYZ", b"gXYZ", b"bXYZ"]
            .into_iter()
            .map(|name| read_xyz(find(name)?))
            .collect::<Result<_, _>>()?;
        let to_xyz = std::array::from_fn(|row| std::array::from_fn(|column| columns[column][row]));
        let curve = read_curve(find(b"rTRC")?)?;
        Ok(Self {
            to_xyz,
            curve,
            description: read_description(find(b"desc").unwrap_or(&[])),
        })
    }
}

fn be_u32(bytes: &[u8], at: usize) -> Result<u32, ProfileError> {
    let slice = bytes.get(at..at + 4).ok_or(ProfileError::Truncated)?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn be_u16(bytes: &[u8], at: usize) -> Result<u16, ProfileError> {
    let slice = bytes.get(at..at + 2).ok_or(ProfileError::Truncated)?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn fixed(bytes: &[u8], at: usize) -> Result<f64, ProfileError> {
    Ok(f64::from(be_u32(bytes, at)? as i32) / 65536.0)
}

fn read_xyz(tag: &[u8]) -> Result<Vector, ProfileError> {
    if tag.get(..4) != Some(b"XYZ ") {
        return Err(ProfileError::Unsupported);
    }
    Ok([fixed(tag, 8)?, fixed(tag, 12)?, fixed(tag, 16)?])
}

fn parametric(kind: u16, p: &[f64], x: f64) -> f64 {
    let (g, a, b, c, d, e, f) = (p[0], p[1], p[2], p[3], p[4], p[5], p[6]);
    match kind {
        0 => x.max(0.0).powf(g),
        1 => {
            if x >= -b / a {
                (a * x + b).max(0.0).powf(g)
            } else {
                0.0
            }
        }
        2 => {
            if x >= -b / a {
                (a * x + b).max(0.0).powf(g) + c
            } else {
                c
            }
        }
        3 => {
            if x >= d {
                (a * x + b).max(0.0).powf(g)
            } else {
                c * x
            }
        }
        _ => {
            if x >= d {
                (a * x + b).max(0.0).powf(g) + e
            } else {
                c * x + f
            }
        }
    }
}

fn read_curve(tag: &[u8]) -> Result<Curve, ProfileError> {
    match tag.get(..4) {
        Some(b"curv") => match be_u32(tag, 8)? as usize {
            0 => Ok(Curve::Linear),
            1 => Ok(Curve::Gamma(f64::from(be_u16(tag, 12)?) / 256.0)),
            n => Ok(Curve::Table(
                (0..n)
                    .map(|index| be_u16(tag, 12 + 2 * index).map(|v| f32::from(v) / 65535.0))
                    .collect::<Result<_, _>>()?,
            )),
        },
        Some(b"para") => {
            let kind = be_u16(tag, 8)?;
            let count = match kind {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => return Err(ProfileError::Unsupported),
            };
            let mut p = [0.0f64; 7];
            for (index, value) in p.iter_mut().enumerate().take(count) {
                *value = fixed(tag, 12 + 4 * index)?;
            }
            Ok(Curve::Table(
                (0..TABLE_LEN)
                    .map(|i| parametric(kind, &p, i as f64 / (TABLE_LEN - 1) as f64) as f32)
                    .collect(),
            ))
        }
        _ => Err(ProfileError::Unsupported),
    }
}

fn read_description(tag: &[u8]) -> String {
    let text = match tag.get(..4) {
        Some(b"desc") => tag.get(12..).map(|rest| {
            let length = be_u32(tag, 8).unwrap_or(0) as usize;
            &rest[..length.saturating_sub(1).min(rest.len())]
        }),
        Some(b"text") => tag.get(8..),
        _ => None,
    };
    text.map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default()
}

fn s15fixed16(value: f64) -> [u8; 4] {
    ((value * 65536.0).round() as i32).to_be_bytes()
}

fn xyz_tag(value: Vector) -> Vec<u8> {
    let mut bytes = b"XYZ \0\0\0\0".to_vec();
    value
        .iter()
        .for_each(|v| bytes.extend_from_slice(&s15fixed16(*v)));
    bytes
}

fn text_tag(text: &str) -> Vec<u8> {
    let mut bytes = b"text\0\0\0\0".to_vec();
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    bytes
}

fn description_tag(text: &str) -> Vec<u8> {
    let mut bytes = b"desc\0\0\0\0".to_vec();
    bytes.extend_from_slice(&(text.len() as u32 + 1).to_be_bytes());
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&[0; 4 + 4 + 2 + 1 + 67]);
    bytes
}

fn curve_tag(curve: &Curve) -> Vec<u8> {
    let mut bytes = b"curv\0\0\0\0".to_vec();
    match curve {
        Curve::Linear => bytes.extend_from_slice(&0u32.to_be_bytes()),
        Curve::Gamma(gamma) => {
            bytes.extend_from_slice(&1u32.to_be_bytes());
            bytes.extend_from_slice(&((gamma * 256.0).round() as u16).to_be_bytes());
        }
        Curve::Srgb | Curve::Table(_) => {
            let table: Vec<u16> = match curve {
                Curve::Table(table) => table.iter().map(|v| (v * 65535.0).round() as u16).collect(),
                _ => (0..TABLE_LEN)
                    .map(|i| {
                        (curve.decode(i as f64 / (TABLE_LEN - 1) as f64) * 65535.0).round() as u16
                    })
                    .collect(),
            };
            bytes.extend_from_slice(&(table.len() as u32).to_be_bytes());
            table
                .iter()
                .for_each(|v| bytes.extend_from_slice(&v.to_be_bytes()));
        }
    }
    bytes
}

#[derive(Default)]
struct Writer {
    tags: Vec<([u8; 4], Vec<u8>)>,
}

impl Writer {
    fn tag(&mut self, name: [u8; 4], data: Vec<u8>) {
        self.tags.push((name, data));
    }

    fn finish(self) -> Vec<u8> {
        let table_len = 4 + 12 * self.tags.len();
        let mut offset = HEADER_LEN + table_len;
        let mut table = (self.tags.len() as u32).to_be_bytes().to_vec();
        let mut body = Vec::new();
        for (name, data) in &self.tags {
            table.extend_from_slice(name);
            table.extend_from_slice(&(offset as u32).to_be_bytes());
            table.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            let padding = (4 - data.len() % 4) % 4;
            body.extend(std::iter::repeat_n(0u8, padding));
            offset += data.len() + padding;
        }
        let mut bytes = vec![0u8; HEADER_LEN];
        bytes[8..12].copy_from_slice(&[2, 0x40, 0, 0]);
        bytes[12..16].copy_from_slice(b"mntr");
        bytes[16..20].copy_from_slice(b"RGB ");
        bytes[20..24].copy_from_slice(b"XYZ ");
        bytes[24..36].copy_from_slice(&[0x07, 0xEA, 0, 10, 0, 1, 0, 0, 0, 0, 0, 0]);
        bytes[36..40].copy_from_slice(b"acsp");
        for (index, value) in D50.iter().enumerate() {
            bytes[68 + 4 * index..72 + 4 * index].copy_from_slice(&s15fixed16(*value));
        }
        bytes[80..84].copy_from_slice(b"STRY");
        bytes.extend_from_slice(&table);
        bytes.extend_from_slice(&body);
        let size = bytes.len() as u32;
        bytes[0..4].copy_from_slice(&size.to_be_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Matrix, b: &Matrix, tolerance: f64) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (a[row][column] - b[row][column]).abs() < tolerance,
                    "{a:?} vs {b:?}"
                );
            }
        }
    }

    #[test]
    fn a_profile_reads_back_as_it_was_written() {
        for space in [Space::Srgb, Space::DisplayP3, Space::AdobeRgb] {
            for linear in [true, false] {
                let profile = Profile::of_space(space, linear);
                let back = Profile::from_icc(&profile.to_icc()).unwrap();
                close(&profile.to_xyz, &back.to_xyz, 5e-5);
                assert_eq!(back.description, profile.description);
                for step in 0..=10 {
                    let encoded = f64::from(step) / 10.0;
                    let (a, b) = (profile.curve.decode(encoded), back.curve.decode(encoded));
                    assert!((a - b).abs() < 2e-3, "{space:?} {linear}: {a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn the_header_is_that_of_an_icc_profile() {
        let bytes = Profile::of_space(Space::Srgb, false).to_icc();
        assert_eq!(&bytes[36..40], b"acsp");
        assert_eq!(
            u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize,
            bytes.len()
        );
        assert_eq!(bytes.len() % 4, 0);
        assert_eq!(&bytes[12..16], b"mntr");
    }

    #[test]
    fn a_camera_profile_keeps_white_white() {
        let camera = [
            [1.2582287, 0.052999634, -0.3112283],
            [-0.11689235, 1.5896847, -0.4727924],
            [-0.09838579, -0.50626373, 1.6046495],
        ];
        let profile = Profile::camera(&camera);
        let white = crate::matrix::apply(&profile.to_xyz, &[1.0, 1.0, 1.0]);
        for (value, expected) in white.iter().zip(D50) {
            assert!((value - expected).abs() < 5e-3, "{white:?}");
        }
    }

    #[test]
    fn bytes_that_are_not_a_profile_are_refused() {
        assert_eq!(Profile::from_icc(b"nope"), Err(ProfileError::NotIcc));
        let mut bytes = Profile::of_space(Space::Srgb, true).to_icc();
        bytes.truncate(200);
        assert!(Profile::from_icc(&bytes).is_err());
    }

    #[test]
    fn a_parametric_srgb_curve_is_understood() {
        let mut tag = b"para\0\0\0\0".to_vec();
        tag.extend_from_slice(&3u16.to_be_bytes());
        tag.extend_from_slice(&[0, 0]);
        for value in [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045] {
            tag.extend_from_slice(&s15fixed16(value));
        }
        let curve = read_curve(&tag).unwrap();
        assert!((curve.decode(0.5) - Curve::Srgb.decode(0.5)).abs() < 1e-3);
    }
}
