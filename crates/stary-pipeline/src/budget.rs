use sysinfo::System;

const DEFAULT_PERCENT: u64 = 50;
const MIN_IN_FLIGHT: usize = 1;
// Measured on 26 MP RAF files: each decoder in flight holds about this many times the file size,
// and the accumulators, masters and gains of a calibration run hold about this many times.
const WORKER_BYTES_PER_FILE_BYTE: u64 = 8;
const FIXED_BYTES_PER_FILE_BYTE: u64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryBudget {
    pub bytes: u64,
}

impl MemoryBudget {
    pub fn from_available(available: u64, percent: u64) -> Self {
        Self {
            bytes: (u128::from(available) * u128::from(percent.min(100)) / 100) as u64,
        }
    }

    pub fn detect(percent: Option<u64>) -> Self {
        let mut system = System::new();
        system.refresh_memory();
        Self::from_available(
            system.available_memory(),
            percent.unwrap_or(DEFAULT_PERCENT),
        )
    }
}

/// Memory a calibration run needs, judged from the size of one input file: (fixed, per worker).
pub fn estimate_costs(file_bytes: u64) -> (u64, u64) {
    (
        file_bytes * FIXED_BYTES_PER_FILE_BYTE,
        file_bytes * WORKER_BYTES_PER_FILE_BYTE,
    )
}

/// How many frames may be decoded at the same time: limited by the cores and by what is left of
/// the memory budget after the fixed buffers.
pub fn plan_in_flight(
    budget: MemoryBudget,
    fixed_bytes: u64,
    per_frame_bytes: u64,
    cores: usize,
) -> usize {
    let spare = budget.bytes.saturating_sub(fixed_bytes);
    let by_memory = (spare / per_frame_bytes.max(1)) as usize;
    by_memory.min(cores).max(MIN_IN_FLIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1 << 30;

    #[test]
    fn a_share_of_the_available_memory_is_the_budget() {
        assert_eq!(MemoryBudget::from_available(40 * GB, 50).bytes, 20 * GB);
        assert_eq!(MemoryBudget::from_available(40 * GB, 500).bytes, 40 * GB);
    }

    #[test]
    fn costs_scale_with_the_file_size() {
        let (fixed, per_worker) = estimate_costs(60_000_000);
        assert!(fixed > per_worker);
        assert_eq!(per_worker, 480_000_000);
    }

    #[test]
    fn plenty_of_memory_is_limited_by_the_cores() {
        let budget = MemoryBudget { bytes: 24 * GB };
        assert_eq!(plan_in_flight(budget, GB, GB / 8, 16), 16);
    }

    #[test]
    fn little_memory_is_limited_by_the_budget() {
        let budget = MemoryBudget { bytes: 2 * GB };
        assert_eq!(plan_in_flight(budget, GB / 2, GB / 8, 16), 12);
    }

    #[test]
    fn never_drops_below_one_frame_even_when_over_budget() {
        let budget = MemoryBudget { bytes: GB / 4 };
        assert_eq!(plan_in_flight(budget, GB, GB / 8, 16), 1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunPlan {
    pub workers: usize,
    pub budget: MemoryBudget,
    pub per_worker_bytes: u64,
    pub cores: usize,
}

/// Decides how many frames to decode at once for a run whose files look like `sample_file`.
pub fn plan_run(sample_file: Option<&std::path::Path>, memory_percent: u64) -> RunPlan {
    let file_bytes = sample_file
        .and_then(|path| std::fs::metadata(path).ok())
        .map_or(0, |metadata| metadata.len());
    let (fixed, per_worker_bytes) = estimate_costs(file_bytes);
    let budget = MemoryBudget::detect(Some(memory_percent));
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    RunPlan {
        workers: plan_in_flight(budget, fixed, per_worker_bytes, cores),
        budget,
        per_worker_bytes,
        cores,
    }
}
