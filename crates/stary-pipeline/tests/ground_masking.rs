mod common;

use std::{path::PathBuf, sync::Arc};

use common::{HEIGHT, Lcg, WIDTH, mosaic_with_ground, star_field, write_all};
use stary_core::{Frame, LinearRgb16, LinearRgbF32};
use stary_mask::Mask;
use stary_pipeline::{
    Cancel, ComputeDevice, Device, Ground, Masking, MemoryBudget, PipelineError, RunOptions,
    StackOptions, TrailsMethod, TrailsResult, TrailsSettings, no_observer, run_stacking,
    run_trails,
};
use stary_raw::DemosaicQuality;
use stary_register::{DetectParams, find_stars};

/// Rows above this are sky, rows below it are ground.
const HORIZON: usize = 120;
const FRAMES: usize = 8;
/// Frames of the trail tests, and the steps the sky moves between the first and the last.
const TRAIL_FRAMES: usize = 5;
const TRAIL_STEPS: f64 = (TRAIL_FRAMES - 1) as f64;

fn sky_stars() -> Vec<(f64, f64, f32)> {
    star_field()
        .into_iter()
        .filter(|&(_, y, _)| y < 100.0)
        .collect()
}

/// Lights on the ground that stay where they are, and many more than there are stars in the sky.
fn ground_lights() -> Vec<(f64, f64, f32)> {
    let mut rng = Lcg(555);
    (0..60)
        .map(|_| {
            (
                50.0 + rng.next() * (WIDTH as f64 - 80.0),
                150.0 + rng.next() * (HEIGHT as f64 - 180.0),
                (20000.0 + rng.next() * 30000.0) as f32,
            )
        })
        .collect()
}

fn drifting_frames() -> Vec<Frame> {
    let (sky, ground) = (sky_stars(), ground_lights());
    (0..FRAMES)
        .map(|index| {
            let t = index as f64;
            mosaic_with_ground(
                &sky,
                (1.3 * t - 4.0 + 0.37 * t.sin(), -0.8 * t + 3.0),
                0.0006 * t,
                &ground,
                300 + index as u64,
            )
        })
        .collect()
}

fn lights(frames: &[Frame]) -> (tempfile::TempDir, Vec<PathBuf>) {
    let dir = tempfile::tempdir().unwrap();
    let paths = write_all(dir.path(), frames);
    (dir, paths)
}

fn sky_above_horizon() -> Arc<Mask> {
    let rows = 24;
    let values = (0..32 * rows)
        .map(|index| {
            if index / 32 < rows * HORIZON / HEIGHT {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    Arc::new(Mask::new(32, rows, values).unwrap())
}

fn masking(ground: Ground) -> Masking {
    Masking {
        mask: sky_above_horizon(),
        ground,
    }
}

fn run_options() -> RunOptions {
    RunOptions {
        workers: 2,
        ..RunOptions::default()
    }
}

fn stack(
    lights: &[PathBuf],
    masking: Option<Masking>,
) -> Result<stary_pipeline::StackResult, PipelineError> {
    stack_on(Device::Cpu, lights, masking)
}

fn stack_on(
    device: Device,
    lights: &[PathBuf],
    masking: Option<Masking>,
) -> Result<stary_pipeline::StackResult, PipelineError> {
    let mut options = StackOptions::new(MemoryBudget { bytes: 1 << 30 });
    options.device = device;
    options.detect = DetectParams {
        threshold_sigma: 8.0,
        ..DetectParams::default()
    };
    options.masking = masking;
    run_stacking(
        lights,
        None,
        &options,
        &run_options(),
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

/// How far the sky stars of the result are from where the reference frame has them.
fn sky_star_offsets(image: &LinearRgbF32) -> Vec<f64> {
    let found = find_stars(
        WIDTH,
        HEIGHT,
        luminance(image),
        &DetectParams {
            threshold_sigma: 8.0,
            ..DetectParams::default()
        },
    );
    let truth = sky_stars();
    truth
        .iter()
        .filter(|&&(x, y, amplitude)| {
            amplitude > 12000.0
                && (15.0..WIDTH as f64 - 15.0).contains(&(x - 4.0))
                && y < 95.0
                && truth
                    .iter()
                    .all(|&(ox, oy, _)| (ox, oy) == (x, y) || (ox - x).hypot(oy - y) > 12.0)
        })
        .map(|&(x, y, _)| {
            let expected = (x - 4.0, y + 3.0);
            found
                .iter()
                .map(|star| (star.x - expected.0).hypot(star.y - expected.1))
                .fold(f64::MAX, f64::min)
        })
        .collect()
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// The value at the pixel nearest to a position.
fn at(luma: &[f32], x: f64, y: f64) -> f32 {
    luma[y.round() as usize * WIDTH + x.round() as usize]
}

/// The brightest value within a few pixels of a position.
fn peak_near(luma: &[f32], x: f64, y: f64) -> f32 {
    let (cx, cy) = (x.round() as isize, y.round() as isize);
    (cy - 3..=cy + 3)
        .flat_map(|row| (cx - 3..=cx + 3).map(move |column| (row, column)))
        .filter(|&(row, column)| {
            (0..HEIGHT as isize).contains(&row) && (0..WIDTH as isize).contains(&column)
        })
        .map(|(row, column)| luma[row as usize * WIDTH + column as usize])
        .fold(0.0, f32::max)
}

#[test]
fn lights_on_the_ground_pull_the_alignment_unless_the_sky_is_masked() {
    let (_dir, paths) = lights(&drifting_frames());
    let plain = stack(&paths, None).unwrap();
    let masked = stack(&paths, Some(masking(Ground::WithSky))).unwrap();
    let (before, after) = (
        median(sky_star_offsets(&plain.image)),
        median(sky_star_offsets(&masked.image)),
    );
    assert!(
        after < 0.4,
        "with the mask the sky stars are {after} px off"
    );
    assert!(before > 1.0, "without it they are only {before} px off");
    assert_eq!(masked.frames, FRAMES as u32);
}

#[test]
fn a_mask_with_too_few_sky_stars_is_refused() {
    let (_dir, paths) = lights(&drifting_frames());
    let all_ground = Mask::new(2, 2, vec![0.0; 4]).unwrap();
    let result = stack(
        &paths,
        Some(Masking {
            mask: Arc::new(all_ground),
            ground: Ground::WithSky,
        }),
    );
    assert!(matches!(result, Err(PipelineError::NotEnoughSkyStars)));
}

#[test]
fn the_ground_is_combined_without_moving_the_frames_when_it_is_kept_apart() {
    let (_dir, paths) = lights(&drifting_frames());
    let with_sky = stack(&paths, Some(masking(Ground::WithSky))).unwrap();
    let apart = stack(&paths, Some(masking(Ground::Separate))).unwrap();
    let (blurred, sharp) = (luminance(&with_sky.image), luminance(&apart.image));
    let lights_on_ground = ground_lights();
    let kept = lights_on_ground
        .iter()
        .filter(|&&(x, y, amplitude)| {
            amplitude > 30000.0 && y > 160.0 && (20.0..WIDTH as f64 - 20.0).contains(&x)
        })
        .map(|&(x, y, amplitude)| {
            (
                peak_near(&blurred, x, y) / amplitude,
                peak_near(&sharp, x, y) / amplitude,
            )
        })
        .collect::<Vec<_>>();
    assert!(kept.len() >= 5);
    let (sharp_share, blurred_share) = (
        median(kept.iter().map(|pair| f64::from(pair.1)).collect()),
        median(kept.iter().map(|pair| f64::from(pair.0)).collect()),
    );
    assert!(sharp_share > 0.8, "kept apart: {sharp_share}");
    assert!(
        blurred_share < sharp_share - 0.3,
        "moved with the sky: {blurred_share}"
    );
    // the sky is the same either way
    let offsets = sky_star_offsets(&apart.image);
    assert!(median(offsets) < 0.4);
}

#[test]
fn the_gpu_keeps_the_ground_apart_like_the_processor() {
    let (_dir, paths) = lights(&drifting_frames());
    let masking = || Some(masking(Ground::Separate));
    let cpu = stack_on(Device::Cpu, &paths, masking()).unwrap();
    let gpu = stack_on(Device::Auto, &paths, masking()).unwrap();
    if gpu.device == ComputeDevice::Cpu {
        eprintln!("skipped, no GPU");
        return;
    }
    let (a, b) = (luminance(&cpu.image), luminance(&gpu.image));
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    assert!(worst < 3.0, "the pictures differ by {worst}");
}

#[test]
fn the_sky_of_a_result_with_the_ground_apart_equals_the_result_without_it() {
    let (_dir, paths) = lights(&drifting_frames());
    let with_sky = stack(&paths, Some(masking(Ground::WithSky))).unwrap();
    let apart = stack(&paths, Some(masking(Ground::Separate))).unwrap();
    let rows = 0..HORIZON - 12;
    let width = WIDTH * 3;
    assert_eq!(
        with_sky.image.samples[rows.start * width..rows.end * width],
        apart.image.samples[rows.start * width..rows.end * width]
    );
}

fn trails(
    lights: &[PathBuf],
    masking: Option<Masking>,
    method: TrailsMethod,
) -> Result<TrailsResult, PipelineError> {
    let mut settings = TrailsSettings::new(DemosaicQuality::Fast, method);
    settings.masking = masking;
    settings.gap_fill.detect = DetectParams {
        threshold_sigma: 8.0,
        ..DetectParams::default()
    };
    run_trails(
        lights,
        None,
        settings,
        &run_options(),
        &Cancel::default(),
        &|_| {},
        &no_observer,
    )
}

fn lighten(lights: &[PathBuf], masking: Option<Masking>, method: TrailsMethod) -> LinearRgb16 {
    trails(lights, masking, method).unwrap().image
}

fn region_mean(
    image: &LinearRgb16,
    rows: std::ops::Range<usize>,
    columns: std::ops::Range<usize>,
) -> f64 {
    let values: Vec<f64> = rows
        .flat_map(|row| columns.clone().map(move |column| (row, column)))
        .flat_map(|(row, column)| {
            let at = (row * image.width + column) * 3;
            image.samples[at..at + 3].iter().map(|&v| f64::from(v))
        })
        .collect();
    values.iter().sum::<f64>() / values.len() as f64
}

#[test]
fn trails_with_the_ground_apart_average_the_ground_and_keep_the_sky_trails() {
    let (_dir, paths) = lights(&drifting_frames());
    let plain = lighten(&paths, None, TrailsMethod::Lighten);
    let apart = lighten(
        &paths,
        Some(masking(Ground::Separate)),
        TrailsMethod::Lighten,
    );
    let single = lighten(&paths[..1], None, TrailsMethod::Lighten);
    let flat_ground = (HEIGHT - 40)..(HEIGHT - 10);
    let (brightest, averaged, one) = (
        region_mean(&plain, flat_ground.clone(), 2..40),
        region_mean(&apart, flat_ground.clone(), 2..40),
        region_mean(&single, flat_ground, 2..40),
    );
    assert!(brightest - averaged > 2.0, "{brightest} vs {averaged}");
    assert!((averaged - one).abs() < 1.5, "{averaged} vs {one}");
    let width = WIDTH * 3;
    assert_eq!(
        plain.samples[..(HORIZON - 12) * width],
        apart.samples[..(HORIZON - 12) * width]
    );
}

#[test]
fn trails_that_keep_the_ground_with_the_sky_are_not_changed_by_the_mask() {
    let (_dir, paths) = lights(&drifting_frames());
    let plain = lighten(&paths, None, TrailsMethod::Lighten);
    let masked = lighten(
        &paths,
        Some(masking(Ground::WithSky)),
        TrailsMethod::Lighten,
    );
    assert_eq!(plain, masked);
}

fn fill(lights: &[PathBuf], masking: Option<Masking>) -> TrailsResult {
    trails(lights, masking, TrailsMethod::GapFilling).unwrap()
}

fn luminance16(image: &LinearRgb16) -> Vec<f32> {
    image
        .samples
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| pixel.iter().map(|&v| f32::from(v)).sum::<f32>() / 3.0)
        .collect()
}

fn trail_frames() -> Vec<Frame> {
    let (sky, ground) = (sky_stars(), ground_lights());
    (0..TRAIL_FRAMES)
        .map(|index| {
            let t = index as f64;
            mosaic_with_ground(&sky, (9.0 * t, 3.0 * t), 0.0, &ground, 400 + index as u64)
        })
        .collect()
}

fn distance_to_segment(point: (f64, f64), from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let along =
        (((point.0 - from.0) * dx + (point.1 - from.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (point.0 - from.0 - along * dx).hypot(point.1 - from.1 - along * dy)
}

/// A bright star of the sky whose trail stays inside the picture and above the horizon, with no
/// other star near the trail.
fn brightest_trailing_star() -> (f64, f64, f32) {
    let stars = sky_stars();
    stars
        .iter()
        .copied()
        .filter(|&(x, y, a)| {
            let end = (x + 9.0 * TRAIL_STEPS, y + 3.0 * TRAIL_STEPS);
            a > 15000.0
                && end.0 + 12.0 < WIDTH as f64
                && end.1 + 12.0 < 100.0
                && stars.iter().all(|&(ox, oy, _)| {
                    (ox, oy) == (x, y) || distance_to_segment((ox, oy), (x, y), end) > 9.0
                })
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .expect("a bright star whose trail stays in the sky")
}

#[test]
fn gap_filling_fills_the_sky_trails_and_leaves_the_ground_lights_alone() {
    let (_dir, paths) = lights(&trail_frames());
    let (x, y, amplitude) = brightest_trailing_star();
    for ground in [Ground::WithSky, Ground::Separate] {
        let result = fill(&paths, Some(masking(ground)));
        let gaps = result.gaps.unwrap();
        assert_eq!(gaps.filled, TRAIL_FRAMES as u32 - 1, "{ground:?}");
        let luma = luminance16(&result.image);
        for fraction in [0.25, 0.5, 0.75] {
            let value = at(&luma, x + 9.0 * fraction, y + 3.0 * fraction);
            assert!(value > 0.55 * amplitude, "{ground:?} {fraction}: {value}");
        }
        // a ground light is one star, not a trail: next to it the picture is dark again
        let beside = beside_ground_lights(&luma);
        assert!(
            beside.iter().all(|&share| share < 0.5),
            "{ground:?}: {beside:?}"
        );
    }
}

/// How bright the picture is half a step of the sky's motion away from each bright light of the
/// ground, along that motion in either direction, as a share of the light.
fn beside_ground_lights(luma: &[f32]) -> Vec<f32> {
    let lights = ground_lights();
    lights
        .iter()
        .copied()
        .filter(|&(x, y, amplitude)| {
            amplitude > 40000.0
                && lights
                    .iter()
                    .all(|&(ox, oy, _)| (ox, oy) == (x, y) || (ox - x).hypot(oy - y) > 15.0)
        })
        .map(|(x, y, amplitude)| {
            let ahead = at(luma, x + 4.5, y + 1.5);
            let behind = at(luma, x - 4.5, y - 1.5);
            ahead.max(behind) / amplitude
        })
        .collect()
}

/// Rows where the mask is neither sky nor ground.
const BAND: std::ops::Range<usize> = 100..140;

/// A mask of one column: sky above the band, ground below it, and `inside(t)` in the band, where
/// `t` goes from 0 to 1 down the band.
fn banded(inside: impl Fn(f32) -> f32) -> Arc<Mask> {
    let values = (0..HEIGHT)
        .map(|row| {
            if row < BAND.start {
                1.0
            } else if row >= BAND.end {
                0.0
            } else {
                inside((row - BAND.start) as f32 / (BAND.end - BAND.start) as f32)
            }
        })
        .collect();
    Arc::new(Mask::new(1, HEIGHT, values).unwrap())
}

fn separate(mask: Arc<Mask>) -> Option<Masking> {
    Some(Masking {
        mask,
        ground: Ground::Separate,
    })
}

#[test]
fn where_the_mask_is_in_between_a_stack_mixes_the_sky_and_the_ground_by_its_value() {
    let (_dir, paths) = lights(&drifting_frames());
    let run = |mask: Arc<Mask>| stack(&paths, separate(mask)).unwrap().image;
    let (sky, ground) = (run(banded(|_| 1.0)), run(banded(|_| 0.0)));
    let mixed = run(banded(|t| 1.0 - t));
    let mut differing = 0;
    for row in BAND {
        let share = 1.0 - (row - BAND.start) as f32 / (BAND.end - BAND.start) as f32;
        for index in (row * WIDTH * 3)..((row + 1) * WIDTH * 3) {
            let (s, g, m) = (
                sky.samples[index],
                ground.samples[index],
                mixed.samples[index],
            );
            assert!(
                (m - (share * s + (1.0 - share) * g)).abs() < 1e-5,
                "row {row}: {m} is not {share} of {s} and the rest of {g}"
            );
            if (s - g).abs() > 1e-5 && share > 0.1 && share < 0.9 {
                assert!(m > s.min(g) - 1e-6 && m < s.max(g) + 1e-6);
                differing += 1;
            }
        }
    }
    assert!(
        differing > 1000,
        "only {differing} samples told the two apart"
    );
}

#[test]
fn where_the_mask_is_in_between_trails_mix_the_sky_and_the_ground_by_its_value() {
    let (_dir, paths) = lights(&drifting_frames());
    let run = |mask: Arc<Mask>| lighten(&paths, separate(mask), TrailsMethod::Lighten);
    let (sky, ground) = (run(banded(|_| 1.0)), run(banded(|_| 0.0)));
    let mixed = run(banded(|t| 1.0 - t));
    for row in BAND {
        let share = 1.0 - (row - BAND.start) as f32 / (BAND.end - BAND.start) as f32;
        for index in (row * WIDTH * 3)..((row + 1) * WIDTH * 3) {
            let expected = share * f32::from(sky.samples[index])
                + (1.0 - share) * f32::from(ground.samples[index]);
            assert!(
                (f32::from(mixed.samples[index]) - expected).abs() <= 1.0,
                "row {row}: {} is not {expected}",
                mixed.samples[index]
            );
        }
    }
}
