use rayon::prelude::*;

use crate::{
    flat_gain::GAIN_ONE,
    geometry::{GeometryError, SensorGeometry},
    master::Master,
};

const CHUNK: usize = 1 << 16;
const GAIN_SHIFT: u32 = 13;
const GAIN_ROUNDING: i32 = 1 << (GAIN_SHIFT - 1);

/// What gets removed from every light: the dark (or bias) signal, then the flat gain.
#[derive(Debug, Clone)]
pub struct CalibrationSet {
    geometry: SensorGeometry,
    offset: Option<Vec<i16>>,
    gain: Option<Vec<u16>>,
}

impl CalibrationSet {
    pub fn new(
        geometry: SensorGeometry,
        dark: Option<&Master>,
        bias: Option<&Master>,
        gain: Option<Vec<u16>>,
    ) -> Result<Self, GeometryError> {
        for master in dark.into_iter().chain(bias) {
            geometry.check_matches(&master.geometry)?;
        }
        if let Some(gain) = &gain {
            geometry.check_samples(gain.len())?;
        }
        let offset = dark.or(bias).map(Master::signal);
        Ok(Self {
            geometry,
            offset,
            gain,
        })
    }

    pub const fn is_empty(&self) -> bool {
        self.offset.is_none() && self.gain.is_none()
    }

    /// Calibrates one light in place. The light keeps its own black level as the pedestal.
    pub fn apply(
        &self,
        samples: &mut [u16],
        black: u16,
        maximum: u16,
    ) -> Result<(), GeometryError> {
        self.geometry.check_samples(samples.len())?;
        let black = i32::from(black);
        let maximum = i32::from(maximum);
        samples
            .par_chunks_mut(CHUNK)
            .enumerate()
            .for_each(|(chunk, values)| {
                for (offset, value) in values.iter_mut().enumerate() {
                    let index = chunk * CHUNK + offset;
                    let mut signal = i32::from(*value) - black;
                    if let Some(dark) = &self.offset {
                        signal -= i32::from(dark[index]);
                    }
                    if let Some(gain) = &self.gain {
                        signal = (signal * i32::from(gain[index]) + GAIN_ROUNDING) >> GAIN_SHIFT;
                    }
                    *value = (signal + black).clamp(0, maximum) as u16;
                }
            });
        Ok(())
    }
}

const _: () = assert!(GAIN_ONE == 1 << GAIN_SHIFT);

#[cfg(test)]
mod tests {
    use super::*;
    use stary_core::{BayerPattern, CfaPattern, PixelLayout};

    fn geometry(width: usize, height: usize) -> SensorGeometry {
        SensorGeometry {
            width,
            height,
            layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        }
    }

    fn master(values: Vec<f32>, black: u16) -> Master {
        Master::new(geometry(values.len(), 1), black, 5, values).unwrap()
    }

    #[test]
    fn subtracts_the_dark_signal_and_keeps_the_light_pedestal() {
        // dark black is 1022, light black is 1024: the pedestal of the light must survive
        let dark = master(vec![1022.0, 1032.0, 1022.0], 1022);
        let set = CalibrationSet::new(geometry(3, 1), Some(&dark), None, None).unwrap();
        let mut light = vec![1124u16, 1134, 1024];
        set.apply(&mut light, 1024, 16383).unwrap();
        assert_eq!(light, vec![1124, 1124, 1024]);
    }

    #[test]
    fn clamps_at_zero_and_at_the_maximum() {
        let dark = master(vec![0.0, 0.0], 0);
        let set = CalibrationSet::new(geometry(2, 1), Some(&dark), None, None).unwrap();
        let mut light = vec![10u16, 10];
        let low = master(vec![100.0, 0.0], 0);
        let set_low = CalibrationSet::new(geometry(2, 1), Some(&low), None, None).unwrap();
        set_low.apply(&mut light, 0, 50).unwrap();
        assert_eq!(light, vec![0, 10]);
        let mut bright = vec![60000u16, 5];
        set.apply(&mut bright, 0, 16383).unwrap();
        assert_eq!(bright, vec![16383, 5]);
    }

    #[test]
    fn the_flat_gain_scales_the_signal_not_the_pedestal() {
        let gain = vec![GAIN_ONE * 2, GAIN_ONE, GAIN_ONE / 2];
        let set = CalibrationSet::new(geometry(3, 1), None, None, Some(gain)).unwrap();
        let mut light = vec![1100u16, 1100, 1100];
        set.apply(&mut light, 1000, 16383).unwrap();
        assert_eq!(light, vec![1200, 1100, 1050]);
    }

    #[test]
    fn uses_the_bias_when_there_is_no_dark() {
        let bias = master(vec![1010.0, 1010.0], 1000);
        let set = CalibrationSet::new(geometry(2, 1), None, Some(&bias), None).unwrap();
        let mut light = vec![1510u16, 1010];
        set.apply(&mut light, 1000, 16383).unwrap();
        assert_eq!(light, vec![1500, 1000]);
    }

    #[test]
    fn without_anything_to_apply_the_light_is_unchanged() {
        let set = CalibrationSet::new(geometry(2, 1), None, None, None).unwrap();
        assert!(set.is_empty());
        let mut light = vec![5u16, 6];
        set.apply(&mut light, 1, 100).unwrap();
        assert_eq!(light, vec![5, 6]);
    }

    #[test]
    fn mismatched_sizes_are_rejected() {
        let dark = Master::new(geometry(4, 1), 0, 1, vec![0.0; 4]).unwrap();
        assert!(CalibrationSet::new(geometry(3, 1), Some(&dark), None, None).is_err());
        let set = CalibrationSet::new(geometry(3, 1), None, None, None).unwrap();
        assert!(set.apply(&mut [0u16; 2], 0, 10).is_err());
    }
}
