use crate::geometry::{GeometryError, SensorGeometry};

#[derive(Debug, Clone, PartialEq)]
pub struct Master {
    pub geometry: SensorGeometry,
    pub black: u16,
    pub count: u32,
    pub samples: Vec<f32>,
    /// A master (usually a flat) whose offset was taken out before it was saved.
    pub offset_removed: bool,
}

impl Master {
    pub fn new(
        geometry: SensorGeometry,
        black: u16,
        count: u32,
        samples: Vec<f32>,
    ) -> Result<Self, GeometryError> {
        geometry.check_samples(samples.len())?;
        Ok(Self {
            geometry,
            black,
            count,
            samples,
            offset_removed: false,
        })
    }

    pub fn with_offset_removed(mut self, removed: bool) -> Self {
        self.offset_removed = removed;
        self
    }

    /// The master with its own black level removed, as the signed amount to subtract from a light.
    pub fn signal(&self) -> Vec<i16> {
        let black = f32::from(self.black);
        self.samples
            .iter()
            .map(|sample| {
                (sample - black)
                    .round()
                    .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stary_core::{BayerPattern, CfaPattern, PixelLayout};

    pub fn geometry(width: usize, height: usize) -> SensorGeometry {
        SensorGeometry {
            width,
            height,
            layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        }
    }

    #[test]
    fn signal_removes_the_black_level() {
        let master = Master::new(geometry(2, 1), 100, 5, vec![100.0, 112.4]).unwrap();
        assert_eq!(master.signal(), vec![0, 12]);
    }

    #[test]
    fn rejects_a_wrong_sample_count() {
        assert_eq!(
            Master::new(geometry(2, 2), 0, 1, vec![0.0; 3]),
            Err(GeometryError::BadSampleCount)
        );
    }
}
