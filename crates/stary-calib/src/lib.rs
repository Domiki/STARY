mod apply;
mod flat_gain;
mod geometry;
mod integrate;
mod master;

pub use apply::CalibrationSet;
pub use flat_gain::{GAIN_ONE, flat_gain};
pub use geometry::{GeometryError, SensorGeometry};
pub use integrate::{
    ClippedAccumulator, MAX_TRIM, MIN_FRAMES_FOR_CLIPPING, StatsAccumulator, Trimmed,
    TrimmedAccumulator, trim_count,
};
pub use master::Master;
