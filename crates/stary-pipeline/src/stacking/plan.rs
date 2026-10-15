use crate::budget::MemoryBudget;

const COLORS: u64 = 3;
const PLANE_SAMPLE_BYTES: u64 = 2;
const STACK_SAMPLE_BYTES: u64 = 4;
const WORKER_BYTES_PER_FILE_BYTE: u64 = 14;
/// What a worker holds per pixel while it makes a frame (the sensor values, the colors as they come
/// out of the demosaicing, the planes and the binned brightness), which is more than the file size
/// says when the file is compressed.
const WORKER_BYTES_PER_PIXEL: u64 = 28;
const MIN_STRIP_ROWS: usize = 16;
const MAX_STRIP_ROWS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackPlan {
    /// Frames prepared at the same time.
    pub workers: usize,
    /// Frames whose color planes stay in memory; the others are made again for every strip.
    pub cached_frames: usize,
    /// Rows of the result that are combined at once.
    pub strip_rows: usize,
}

pub struct StackShape {
    pub width: usize,
    pub height: usize,
    pub frames: usize,
    pub file_bytes: u64,
    /// How many strips are combined at the same time: 2 when the ground is combined apart.
    pub strip_layers: usize,
}

impl StackShape {
    fn pixels(&self) -> u64 {
        (self.width * self.height) as u64
    }

    pub fn planes_bytes(&self) -> u64 {
        self.pixels() * COLORS * PLANE_SAMPLE_BYTES
    }

    fn worker_bytes(&self) -> u64 {
        (self.file_bytes * WORKER_BYTES_PER_FILE_BYTE).max(self.pixels() * WORKER_BYTES_PER_PIXEL)
    }

    fn result_bytes(&self) -> u64 {
        self.pixels() * COLORS * STACK_SAMPLE_BYTES
    }

    fn strip_bytes(&self, rows: usize) -> u64 {
        (self.strip_layers * self.frames * rows * self.width) as u64 * COLORS * STACK_SAMPLE_BYTES
    }
}

fn plan_with(shape: &StackShape, budget: MemoryBudget, workers: usize) -> StackPlan {
    let per_worker = shape.worker_bytes();
    let fixed = shape.result_bytes() + shape.strip_bytes(MIN_STRIP_ROWS);
    let after_workers = budget
        .bytes
        .saturating_sub(fixed + workers as u64 * per_worker);
    let cached_frames = ((after_workers / shape.planes_bytes().max(1)) as usize).min(shape.frames);
    let cache_bytes = cached_frames as u64 * shape.planes_bytes();
    let repeating = if cached_frames < shape.frames {
        workers as u64 * per_worker
    } else {
        0
    };
    let spare = budget
        .bytes
        .saturating_sub(shape.result_bytes() + cache_bytes + repeating);
    let row_bytes = shape.strip_bytes(1).max(1);
    let strip_rows = ((spare / row_bytes) as usize).clamp(MIN_STRIP_ROWS, MAX_STRIP_ROWS);
    StackPlan {
        workers,
        cached_frames,
        strip_rows: strip_rows.min(shape.height.max(1)),
    }
}

/// Rough time in units of "one frame made from its file": the frames are made once at the start
/// and the frames that are not kept are made again for every strip.
fn cost(shape: &StackShape, plan: &StackPlan) -> usize {
    let strips = shape.height.div_ceil(plan.strip_rows.max(1));
    let first_pass = shape.frames.div_ceil(plan.workers);
    let repeated = (shape.frames - plan.cached_frames).div_ceil(plan.workers);
    first_pass + strips * repeated
}

/// Bytes per sample of the picture that streaming keeps for one kind of lay: sum, sum of squares,
/// count, and what is added up in the second lay.
const STREAM_LAYER_SAMPLE_BYTES: u64 = 8 + 8 + 2 + 8 + 2;

/// Time in units of "one frame made from its file" of streaming: the frames are made once to be
/// examined, and those that are not kept are made again for each of the two lays.
fn streaming_cost(frames: usize, workers: usize, cached: usize) -> usize {
    frames.div_ceil(workers) + 2 * (frames - cached).div_ceil(workers)
}

/// Where the picture that the frames are laid on is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accumulation {
    /// In the memory of the computer, laid on by the processor.
    Host,
    /// On the GPU, which holds `bytes`; they come out of the memory of the computer when the GPU
    /// shares it.
    Gpu { bytes: u64, shared: bool },
}

/// What stays in the memory of the computer for the whole run, apart from workers and kept frames.
fn streaming_fixed_bytes(shape: &StackShape, accumulation: Accumulation) -> u64 {
    let samples = shape.pixels() * COLORS;
    let layers = shape.strip_layers as u64;
    match accumulation {
        Accumulation::Host => {
            samples * (STREAM_LAYER_SAMPLE_BYTES * layers + 2 * STACK_SAMPLE_BYTES)
        }
        Accumulation::Gpu { bytes, shared } => {
            let sending = 2 * shape.planes_bytes();
            let handed_back = samples * STACK_SAMPLE_BYTES * layers;
            sending + handed_back + if shared { bytes } else { 0 }
        }
    }
}

/// How the budget is shared between preparing frames and keeping them in memory when the frames
/// are laid on the picture one at a time (no strips).
pub fn plan_streaming(
    shape: &StackShape,
    budget: MemoryBudget,
    cores: usize,
    accumulation: Accumulation,
) -> StackPlan {
    let fixed = streaming_fixed_bytes(shape, accumulation);
    let per_worker = shape.worker_bytes();
    (1..=cores.max(1))
        .filter(|&workers| workers == 1 || fixed + workers as u64 * per_worker <= budget.bytes)
        .map(|workers| {
            let spare = budget
                .bytes
                .saturating_sub(fixed + workers as u64 * per_worker);
            let cached = ((spare / shape.planes_bytes().max(1)) as usize).min(shape.frames);
            StackPlan {
                workers,
                cached_frames: cached,
                strip_rows: 0,
            }
        })
        .min_by_key(|plan| {
            (
                streaming_cost(shape.frames, plan.workers, plan.cached_frames),
                std::cmp::Reverse(plan.workers),
            )
        })
        .expect("at least one worker count")
}

/// How the budget is shared between preparing frames, keeping them in memory and combining
/// strips of the result. Fewer workers leave room to keep more frames, and keeping frames is
/// worth it as long as it saves more time than the slower first pass costs.
pub fn plan_stacking(shape: &StackShape, budget: MemoryBudget, cores: usize) -> StackPlan {
    let per_worker = shape.worker_bytes();
    let fixed = shape.result_bytes() + shape.strip_bytes(MIN_STRIP_ROWS);
    (1..=cores.max(1))
        .filter(|&workers| workers == 1 || fixed + workers as u64 * per_worker <= budget.bytes)
        .map(|workers| plan_with(shape, budget, workers))
        .min_by_key(|plan| (cost(shape, plan), std::cmp::Reverse(plan.workers)))
        .expect("at least one worker count")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1 << 30;

    fn shape(frames: usize) -> StackShape {
        StackShape {
            width: 6246,
            height: 4170,
            frames,
            file_bytes: 58_000_000,
            strip_layers: 1,
        }
    }

    #[test]
    fn plenty_of_memory_keeps_every_frame_and_uses_every_core() {
        let plan = plan_stacking(&shape(40), MemoryBudget { bytes: 40 * GB }, 16);
        assert_eq!(plan.cached_frames, 40);
        assert_eq!(plan.workers, 16);
        assert_eq!(plan.strip_rows, 256);
    }

    #[test]
    fn fewer_workers_are_used_when_that_lets_every_frame_stay_in_memory() {
        let plan = plan_stacking(&shape(40), MemoryBudget { bytes: 18 * GB }, 16);
        assert_eq!(plan.cached_frames, 40, "{plan:?}");
        assert!(plan.workers >= 8 && plan.workers < 16, "{plan:?}");
    }

    #[test]
    fn a_tight_budget_caches_fewer_frames_and_cuts_strips_thinner() {
        let plan = plan_stacking(&shape(40), MemoryBudget { bytes: 6 * GB }, 16);
        assert!(plan.cached_frames < 40, "{plan:?}");
        assert!(plan.workers >= 1 && plan.workers < 16, "{plan:?}");
        assert!(plan.strip_rows >= MIN_STRIP_ROWS);
    }

    #[test]
    fn the_plan_is_never_slower_than_using_every_core_with_what_is_left() {
        let budget = MemoryBudget { bytes: 12 * GB };
        let chosen = plan_stacking(&shape(40), budget, 16);
        let greedy = plan_with(&shape(40), budget, 16);
        assert!(cost(&shape(40), &chosen) <= cost(&shape(40), &greedy));
    }

    #[test]
    fn streaming_keeps_what_the_budget_allows_and_the_memory_does_not_grow_with_frames() {
        let few = plan_streaming(
            &shape(40),
            MemoryBudget { bytes: 40 * GB },
            16,
            Accumulation::Host,
        );
        assert_eq!(few.cached_frames, 40);
        let many = plan_streaming(
            &shape(1200),
            MemoryBudget { bytes: 18 * GB },
            16,
            Accumulation::Host,
        );
        assert!(many.cached_frames < 1200 && many.workers >= 1);
        assert_eq!(many.strip_rows, 0);
    }

    #[test]
    fn a_gpu_that_shares_the_memory_takes_its_picture_out_of_the_budget() {
        let budget = MemoryBudget { bytes: 18 * GB };
        let gpu = |shared| Accumulation::Gpu {
            bytes: 2 * GB,
            shared,
        };
        let separate = plan_streaming(&shape(1200), budget, 16, gpu(false));
        let shared = plan_streaming(&shape(1200), budget, 16, gpu(true));
        assert!(shared.cached_frames <= separate.cached_frames);
    }

    #[test]
    fn a_compressed_file_does_not_hide_what_a_worker_holds() {
        let compressed = StackShape {
            file_bytes: 24_000_000,
            ..shape(200)
        };
        assert!(compressed.worker_bytes() >= 6246 * 4170 * WORKER_BYTES_PER_PIXEL);
        assert_eq!(
            shape(200).worker_bytes(),
            58_000_000 * WORKER_BYTES_PER_FILE_BYTE
        );
    }

    #[test]
    fn a_budget_that_is_too_small_still_makes_a_plan() {
        let plan = plan_stacking(&shape(40), MemoryBudget { bytes: GB / 4 }, 16);
        assert_eq!(plan.workers, 1);
        assert_eq!(plan.cached_frames, 0);
        assert_eq!(plan.strip_rows, MIN_STRIP_ROWS);
    }

    #[test]
    fn strips_never_exceed_the_image() {
        let tiny = StackShape {
            width: 100,
            height: 10,
            frames: 3,
            file_bytes: 1000,
            strip_layers: 1,
        };
        assert_eq!(
            plan_stacking(&tiny, MemoryBudget { bytes: 40 * GB }, 4).strip_rows,
            10
        );
    }
}
