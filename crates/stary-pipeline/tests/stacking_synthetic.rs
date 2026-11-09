mod common;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use common::{HEIGHT, Lcg, WIDTH, mosaic, star_field, write_all};
use stary_core::{Frame, LinearRgbF32};
use stary_pipeline::{
    Cancel, ComputeDevice, Device, Exposure, GpuKind, MemoryBudget, PipelineError, RunOptions,
    StackGroup, StackOptions, StackResult, detect_gpu, no_observer, run_stacking,
    run_stacking_groups,
};
use stary_register::{DetectParams, find_stars};
use stary_stack::Rejection;

fn options(bytes: u64) -> StackOptions {
    let mut options = StackOptions::new(MemoryBudget { bytes });
    options.device = Device::Cpu;
    options.detect = DetectParams {
        threshold_sigma: 8.0,
        ..DetectParams::default()
    };
    options
}

static GPU: Mutex<()> = Mutex::new(());

fn one_gpu_at_a_time() -> MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(PoisonError::into_inner)
}

fn stack(lights: &[PathBuf], options: &StackOptions) -> Result<StackResult, PipelineError> {
    let _gpu = (options.device != Device::Cpu).then(one_gpu_at_a_time);
    let run = RunOptions {
        workers: 2,
        ..RunOptions::default()
    };
    run_stacking(
        lights,
        None,
        options,
        &run,
        &no_observer,
        &Cancel::default(),
        &|_| {},
    )
}

fn luminance(image: &LinearRgbF32) -> Vec<f32> {
    image
        .samples
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| (pixel[0] + pixel[1] + pixel[2]) / 3.0 * 65535.0)
        .collect()
}

fn drifting_frames(count: usize) -> Vec<Frame> {
    let stars = star_field();
    (0..count)
        .map(|index| {
            let t = index as f64;
            mosaic(
                &stars,
                (1.3 * t - 4.0 + 0.37 * t.sin(), -0.8 * t + 3.0),
                0.0006 * t,
                100 + index as u64,
            )
        })
        .collect()
}

#[test]
fn drifting_frames_are_aligned_onto_the_first_one() {
    let dir = tempfile::tempdir().unwrap();
    let frames = drifting_frames(8);
    let lights = write_all(dir.path(), &frames);
    let result = stack(&lights, &options(1 << 30)).unwrap();
    assert_eq!(result.frames, 8);
    assert!(result.skipped.is_empty(), "{:?}", result.skipped);
    assert_eq!(result.reference, 0);

    let found = find_stars(WIDTH, HEIGHT, luminance(&result.image), &options(0).detect);
    let truth = star_field();
    let (cx, cy) = (WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0);
    let first_angle = 0.0f64;
    let mut checked = 0;
    for &(x, y, amplitude) in &truth {
        let blended = truth
            .iter()
            .any(|&(ox, oy, _)| (ox, oy) != (x, y) && (ox - x).hypot(oy - y) < 12.0);
        if blended {
            continue;
        }
        let (dx, dy) = (x - cx, y - cy);
        let (rx, ry) = (
            cx + first_angle.cos() * dx - first_angle.sin() * dy - 4.0,
            cy + first_angle.sin() * dx + first_angle.cos() * dy + 3.0,
        );
        if amplitude < 12000.0
            || !(15.0..WIDTH as f64 - 15.0).contains(&rx)
            || !(15.0..HEIGHT as f64 - 15.0).contains(&ry)
        {
            continue;
        }
        let nearest = found
            .iter()
            .map(|star| ((star.x - rx).hypot(star.y - ry), star))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap();
        assert!(
            nearest.0 < 0.35,
            "star at {rx:.1},{ry:.1} is {:.2}px off",
            nearest.0
        );
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} stars were checked");
}

/// The worst difference, in the rows around a trail laid over one frame, from the same stack
/// without the trail.
fn trail_left_behind(frames: usize, rejection: Rejection, device: Device) -> f32 {
    let dir = tempfile::tempdir().unwrap();
    let clean_frames = drifting_frames(frames);
    let mut trailed = clean_frames.clone();
    for column in 0..WIDTH {
        trailed[frames / 2].samples[120 * WIDTH + column] = 60000;
    }
    let clean = write_all(&dir.path().join("clean"), &clean_frames);
    let with_trail = write_all(&dir.path().join("trail"), &trailed);
    let mut settings = options(1 << 30);
    settings.rejection = rejection;
    settings.device = device;
    let reference = luminance(&stack(&clean, &settings).unwrap().image);
    let result = luminance(&stack(&with_trail, &settings).unwrap().image);
    (118..124)
        .flat_map(|row| (10..WIDTH - 10).map(move |column| row * WIDTH + column))
        .map(|index| (result[index] - reference[index]).abs())
        .fold(0.0f32, f32::max)
}

#[test]
fn a_satellite_trail_in_one_frame_does_not_reach_the_result_of_the_exact_stack() {
    let winsorized = Rejection::Winsorized {
        low: 3.0,
        high: 3.0,
    };
    let worst = trail_left_behind(9, winsorized, Device::Cpu);
    assert!(worst < 300.0, "the trail left {worst} behind");
}

#[test]
fn a_satellite_trail_in_one_frame_does_not_reach_the_streamed_result() {
    let sigma = Rejection::Sigma {
        low: 3.0,
        high: 3.0,
    };
    let worst = trail_left_behind(15, sigma, Device::Cpu);
    assert!(worst < 300.0, "the trail left {worst} behind");
}

#[test]
fn a_satellite_trail_in_one_frame_does_not_reach_the_result_of_the_gpu() {
    let sigma = Rejection::Sigma {
        low: 3.0,
        high: 3.0,
    };
    if !gpu_is_there() {
        return;
    }
    let worst = trail_left_behind(15, sigma, Device::Gpu);
    assert!(worst < 300.0, "the trail left {worst} behind");
}

fn gpu_is_there() -> bool {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(3));
    let mut probe = options(1 << 30);
    probe.device = Device::Auto;
    let there = stack(&lights, &probe).is_ok_and(|result| result.device != ComputeDevice::Cpu);
    if !there {
        eprintln!("skipped, no GPU");
    }
    there
}

fn difference_between_devices(rejection: Rejection, frames: usize) -> Option<(f32, f64)> {
    if !gpu_is_there() {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(frames));
    let mut cpu = options(1 << 30);
    cpu.rejection = rejection;
    let mut gpu = cpu.clone();
    gpu.device = Device::Gpu;
    let on_gpu = stack(&lights, &gpu).unwrap();
    assert!(matches!(on_gpu.device, ComputeDevice::Gpu(_)));
    let a = luminance(&stack(&lights, &cpu).unwrap().image);
    let b = luminance(&on_gpu.image);
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    let mean = a
        .iter()
        .zip(&b)
        .map(|(x, y)| f64::from((x - y).abs()))
        .sum::<f64>()
        / a.len() as f64;
    Some((worst, mean))
}

#[test]
fn the_gpu_mean_equals_the_mean_of_the_processor() {
    let Some((worst, mean)) = difference_between_devices(Rejection::None, 8) else {
        return;
    };
    assert!(worst < 0.5 && mean < 0.01, "worst {worst}, mean {mean}");
}

#[test]
fn the_gpu_clipped_mean_equals_that_of_the_processor() {
    let sigma = Rejection::Sigma {
        low: 3.0,
        high: 3.0,
    };
    let Some((worst, mean)) = difference_between_devices(sigma, 15) else {
        return;
    };
    assert!(worst < 5.0 && mean < 0.05, "worst {worst}, mean {mean}");
}

fn gpu_shares_the_memory() -> bool {
    let _gpu = one_gpu_at_a_time();
    detect_gpu().is_some_and(|gpu| gpu.kind == GpuKind::Integrated)
}

#[test]
fn a_picture_that_does_not_fit_the_gpu_is_stacked_by_the_processor_when_the_device_is_auto() {
    if !gpu_shares_the_memory() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(4));
    let mut auto = options(1);
    auto.device = Device::Auto;
    assert_eq!(stack(&lights, &auto).unwrap().device, ComputeDevice::Cpu);
}

#[test]
fn a_picture_that_does_not_fit_the_gpu_is_an_error_when_the_device_is_the_gpu() {
    if !gpu_shares_the_memory() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(4));
    let mut gpu = options(1);
    gpu.device = Device::Gpu;
    assert!(matches!(stack(&lights, &gpu), Err(PipelineError::Gpu(_))));
}

#[test]
fn the_exact_stack_is_always_done_by_the_processor() {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(5));
    let mut exact = options(1 << 30);
    exact.rejection = Rejection::Winsorized {
        low: 3.0,
        high: 3.0,
    };
    exact.device = Device::Gpu;
    assert_eq!(stack(&lights, &exact).unwrap().device, ComputeDevice::Cpu);
}

#[test]
fn the_streamed_mean_equals_the_mean_of_the_strips() {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(6));
    let mut streamed = options(1 << 30);
    streamed.rejection = Rejection::None;
    let mut by_strips = streamed.clone();
    by_strips.rejection = Rejection::Winsorized {
        low: 1000.0,
        high: 1000.0,
    };
    let a = luminance(&stack(&lights, &streamed).unwrap().image);
    let b = luminance(&stack(&lights, &by_strips).unwrap().image);
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    assert!(worst < 1.0, "the means differ by {worst}");
}

#[test]
fn streaming_and_strips_agree_on_a_clean_stack() {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(15));
    let streamed = options(1 << 30);
    let mut by_strips = streamed.clone();
    by_strips.rejection = Rejection::Winsorized {
        low: 3.0,
        high: 3.0,
    };
    let a = luminance(&stack(&lights, &streamed).unwrap().image);
    let b = luminance(&stack(&lights, &by_strips).unwrap().image);
    let mean_difference = a
        .iter()
        .zip(&b)
        .map(|(x, y)| f64::from((x - y).abs()))
        .sum::<f64>()
        / a.len() as f64;
    assert!(
        mean_difference < 2.0,
        "the stacks differ by {mean_difference} on average"
    );
}

#[test]
fn keeping_frames_in_memory_or_making_them_again_gives_the_same_result() {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(5));
    let roomy = stack(&lights, &options(1 << 30)).unwrap();
    let tight = stack(&lights, &options(1)).unwrap();
    assert_eq!(roomy.image, tight.image);
}

#[test]
fn a_frame_of_another_part_of_the_sky_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let mut frames = drifting_frames(6);
    let mut rng = Lcg(77);
    let elsewhere: Vec<(f64, f64, f32)> = (0..60)
        .map(|_| {
            (
                30.0 + rng.next() * 260.0,
                30.0 + rng.next() * 180.0,
                (4000.0 + rng.next() * 40000.0) as f32,
            )
        })
        .collect();
    frames[3] = mosaic(&elsewhere, (0.0, 0.0), 0.0, 5);
    let lights = write_all(dir.path(), &frames);
    let result = stack(&lights, &options(1 << 30)).unwrap();
    assert_eq!(result.frames, 5);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].index, 3);
}

#[test]
fn a_single_frame_cannot_be_stacked() {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(1));
    assert!(matches!(
        stack(&lights, &options(1 << 30)),
        Err(PipelineError::NotEnoughFrames)
    ));
}

#[test]
fn a_cancelled_stack_stops() {
    let dir = tempfile::tempdir().unwrap();
    let lights = write_all(dir.path(), &drifting_frames(4));
    let cancel = Cancel::default();
    cancel.request();
    let result = run_stacking(
        &lights,
        None,
        &options(1 << 30),
        &RunOptions::default(),
        &no_observer,
        &cancel,
        &|_| {},
    );
    assert!(matches!(result, Err(PipelineError::Cancelled)));
}

fn brightened(mut frame: Frame, extra: u16) -> Frame {
    for sample in &mut frame.samples {
        *sample = sample.saturating_add(extra);
    }
    frame
}

#[test]
fn two_nights_are_stacked_together_and_judged_each_by_its_own_frames() {
    let first_night = tempfile::tempdir().unwrap();
    let second_night = tempfile::tempdir().unwrap();
    let frames = drifting_frames(8);
    let (early, late) = frames.split_at(6);
    let early = write_all(first_night.path(), early);
    let late: Vec<Frame> = late
        .iter()
        .cloned()
        .map(|frame| brightened(frame, 6000))
        .collect();
    let late = write_all(second_night.path(), &late);
    let groups = [
        StackGroup {
            lights: &early,
            calibration: None,
            exposure: None,
        },
        StackGroup {
            lights: &late,
            calibration: None,
            exposure: None,
        },
    ];
    let run = RunOptions {
        workers: 2,
        ..RunOptions::default()
    };
    let result = run_stacking_groups(
        &groups,
        &options(1 << 30),
        &run,
        &no_observer,
        &Cancel::default(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(result.frames, 8);
    assert!(result.skipped.is_empty(), "{:?}", result.skipped);
    assert_eq!(result.reports.len(), 8);
}

fn doubled(frame: &Frame) -> Frame {
    let mut frame = frame.clone();
    for sample in &mut frame.samples {
        *sample = sample.saturating_mul(2);
    }
    frame
}

fn brightest(image: &LinearRgbF32) -> f32 {
    luminance(image).into_iter().fold(0.0, f32::max)
}

fn join_two_exposures(device: Device) {
    let _gpu = (device != Device::Cpu).then(one_gpu_at_a_time);
    let dir = tempfile::tempdir().unwrap();
    let short_frames = drifting_frames(6);
    let long_frames: Vec<Frame> = short_frames.iter().map(doubled).collect();
    let long = write_all(&dir.path().join("long"), &long_frames);
    let short = write_all(&dir.path().join("short"), &short_frames);
    let exposure = |seconds| {
        Some(Exposure {
            seconds,
            iso: 100.0,
        })
    };
    let groups = [
        StackGroup {
            lights: &long,
            calibration: None,
            exposure: exposure(20.0),
        },
        StackGroup {
            lights: &short,
            calibration: None,
            exposure: exposure(10.0),
        },
    ];
    let run = RunOptions {
        workers: 2,
        ..RunOptions::default()
    };
    let mut joined = options(1 << 30);
    joined.device = device;
    joined.hdr = true;
    let hdr = run_stacking_groups(
        &groups,
        &joined,
        &run,
        &no_observer,
        &Cancel::default(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(hdr.frames, 12);

    let long_only = brightest(&stack(&long, &options(1 << 30)).unwrap().image);
    let short_only = brightest(&stack(&short, &options(1 << 30)).unwrap().image);
    let joined = brightest(&hdr.image);
    assert!(
        joined > 1.2 * long_only,
        "{joined} against {long_only} cut off"
    );
    let wanted = 2.0 * short_only;
    assert!(
        (joined - wanted).abs() < 0.12 * wanted,
        "{joined} but the shorter stack says {wanted}"
    );
}

#[test]
fn joining_two_exposures_recovers_the_stars_the_longer_one_cut_off() {
    join_two_exposures(Device::Cpu);
}

#[test]
fn joining_two_exposures_works_on_the_gpu_too() {
    if detect_gpu().is_some() {
        join_two_exposures(Device::Gpu);
    }
}
