mod clipped_accumulator;
mod stats_accumulator;
mod trimmed_accumulator;

pub use clipped_accumulator::ClippedAccumulator;
pub use stats_accumulator::StatsAccumulator;
pub use trimmed_accumulator::{MAX_TRIM, Trimmed, TrimmedAccumulator, trim_count};

pub const MIN_FRAMES_FOR_CLIPPING: u32 = 4;
