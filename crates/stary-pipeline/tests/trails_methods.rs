mod common;

use std::path::PathBuf;

use common::{HEIGHT, WIDTH, mosaic, star_field, write_all};
use stary_core::{Frame, LinearRgb16};
use stary_pipeline::{
    Cancel, RunOptions, TrailsMethod, TrailsResult, TrailsSettings, no_observer, run_trails,
};
use stary_raw::DemosaicQuality;
use stary_register::DetectParams;

const FRAMES: usize = 6;
const STEP: (f64, f64) = (12.0, 4.0);

/// The same stars in every frame, moved by `STEP` from one frame to the next like a sky that turns.
fn moving_frames() -> Vec<Frame> {
    let stars = star_field();
    (0..FRAMES)
        .map(|index| {
            let t = index as f64;
            mosaic(&stars, (STEP.0 * t, STEP.1 * t), 0.0, 40 + index as u64)
        })
        .collect()
}

fn lights() -> (tempfile::TempDir, Vec<PathBuf>) {
    let dir = tempfile::tempdir().unwrap();
    let paths = write_all(dir.path(), &moving_frames());
    (dir, paths)
}

fn settings(method: TrailsMethod) -> TrailsSettings {
    let mut settings = TrailsSettings::new(DemosaicQuality::Fast, method);
    settings.gap_fill.detect = DetectParams {
        threshold_sigma: 8.0,
        ..DetectParams::default()
    };
    settings
}

fn run(
    lights: &[PathBuf],
    settings: TrailsSettings,
    workers: usize,
) -> Result<TrailsResult, stary_pipeline::PipelineError> {
    run_trails(
        lights,
        None,
        settings,
        &RunOptions {
            workers,
            ..RunOptions::default()
        },
        &Cancel::default(),
        &|_| {},
        &no_observer,
    )
}

fn lighten_or_comet(lights: &[PathBuf], method: TrailsMethod) -> LinearRgb16 {
    run(lights, settings(method), 2).unwrap().image
}

fn fill_gaps(lights: &[PathBuf]) -> TrailsResult {
    run(lights, settings(TrailsMethod::GapFilling), 2).unwrap()
}

fn brightness16(image: &LinearRgb16, x: usize, y: usize) -> f32 {
    let at = (y * image.width + x) * 3;
    image.samples[at..at + 3]
        .iter()
        .map(|v| f32::from(*v))
        .sum::<f32>()
        / 3.0
}

/// A bright star of the first frame that stays inside the picture for the whole sequence.
fn tracked_star() -> (f64, f64, f32) {
    star_field()
        .into_iter()
        .filter(|&(x, y, amplitude)| {
            amplitude > 30000.0
                && x + STEP.0 * (FRAMES - 1) as f64 + 12.0 < WIDTH as f64
                && y + STEP.1 * (FRAMES - 1) as f64 + 12.0 < HEIGHT as f64
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .expect("a bright star that stays in view")
}

#[test]
fn lighten_leaves_a_gap_between_the_images_of_a_star() {
    let (_dir, paths) = lights();
    let image = lighten_or_comet(&paths, TrailsMethod::Lighten);
    let (x, y, amplitude) = tracked_star();
    let halfway = (
        (x + STEP.0 / 2.0).round() as usize,
        (y + STEP.1 / 2.0).round() as usize,
    );
    assert!(brightness16(&image, x.round() as usize, y.round() as usize) > 0.5 * amplitude);
    assert!(brightness16(&image, halfway.0, halfway.1) < 0.1 * amplitude);
}

#[test]
fn gap_filling_joins_the_images_of_a_star() {
    let (_dir, paths) = lights();
    let result = fill_gaps(&paths);
    assert_eq!(result.frames, FRAMES as u32);
    let gaps = result.gaps.unwrap();
    assert_eq!((gaps.filled, gaps.skipped), (FRAMES as u32 - 1, 0));
    let (x, y, amplitude) = tracked_star();
    for fraction in [0.25, 0.5, 0.75] {
        let at = (
            (x + STEP.0 * fraction).round() as usize,
            (y + STEP.1 * fraction).round() as usize,
        );
        let value = brightness16(&result.image, at.0, at.1);
        assert!(
            value > 0.55 * amplitude,
            "{fraction}: {value} of {amplitude}"
        );
    }
    // far from every star path nothing is added
    let sky = brightness16(&result.image, 2, HEIGHT - 3);
    assert!(sky < 6000.0, "{sky}");
}

#[test]
fn the_end_of_the_last_trail_is_not_stretched() {
    let (_dir, paths) = lights();
    let result = fill_gaps(&paths).image;
    let (x, y, amplitude) = tracked_star();
    let last = (
        x + STEP.0 * (FRAMES - 1) as f64,
        y + STEP.1 * (FRAMES - 1) as f64,
    );
    let at = |dx: f64, dy: f64| {
        brightness16(
            &result,
            (last.0 + dx).round() as usize,
            (last.1 + dy).round() as usize,
        )
    };
    assert!(at(0.0, 0.0) > 0.5 * amplitude);
    assert!(
        at(STEP.0 * 0.5, STEP.1 * 0.5) < 0.2 * amplitude,
        "{}",
        at(STEP.0 * 0.5, STEP.1 * 0.5)
    );
}

#[test]
fn gap_filling_gives_the_same_picture_however_many_workers_there_are() {
    let (_dir, paths) = lights();
    let one = run(&paths, settings(TrailsMethod::GapFilling), 1).unwrap();
    let many = run(&paths, settings(TrailsMethod::GapFilling), 4).unwrap();
    assert_eq!(one.image, many.image);
}

#[test]
fn lights_cut_into_stretches_fill_the_same_gaps_when_the_motion_is_the_same() {
    let (_dir, paths) = lights();
    let whole = fill_gaps(&paths).image;
    let mut cut = settings(TrailsMethod::GapFilling);
    cut.gap_fill.stretch_frames = 2;
    let in_stretches = run(&paths, cut, 2).unwrap().image;
    let differing = whole
        .samples
        .iter()
        .zip(&in_stretches.samples)
        .filter(|(a, b)| a.abs_diff(**b) > 1000)
        .count();
    assert!(
        differing * 100 < whole.samples.len(),
        "{differing} of {} samples differ by more than 1000",
        whole.samples.len()
    );
}

#[test]
fn a_comet_keeps_the_newest_frame_and_fades_the_oldest() {
    let (_dir, paths) = lights();
    let lighten = lighten_or_comet(&paths, TrailsMethod::Lighten);
    let comet = lighten_or_comet(
        &paths,
        TrailsMethod::Comet {
            tail_frames: 3,
            reverse: false,
        },
    );
    let (x, y, _) = tracked_star();
    let oldest = (x.round() as usize, y.round() as usize);
    let newest = (
        (x + STEP.0 * (FRAMES - 1) as f64).round() as usize,
        (y + STEP.1 * (FRAMES - 1) as f64).round() as usize,
    );
    let (kept, faded) = (
        brightness16(&comet, newest.0, newest.1),
        brightness16(&comet, oldest.0, oldest.1),
    );
    let full = brightness16(&lighten, newest.0, newest.1);
    assert!(kept > 0.9 * full, "newest {kept} of {full}");
    assert!(
        faded < 0.3 * brightness16(&lighten, oldest.0, oldest.1),
        "oldest {faded}"
    );
}

#[test]
fn reading_the_lights_in_reverse_equals_listing_them_backwards() {
    let (_dir, paths) = lights();
    let backwards: Vec<PathBuf> = paths.iter().rev().cloned().collect();
    let comet = |reverse| TrailsMethod::Comet {
        tail_frames: 3,
        reverse,
    };
    let reversed_option = lighten_or_comet(&paths, comet(true));
    let reversed_list = lighten_or_comet(&backwards, comet(false));
    assert_eq!(reversed_option, reversed_list);
    assert_ne!(reversed_option, lighten_or_comet(&paths, comet(false)));
}

#[test]
fn a_longer_tail_keeps_more_of_the_old_frames() {
    let (_dir, paths) = lights();
    let (x, y, _) = tracked_star();
    let at = (x.round() as usize, y.round() as usize);
    let short = brightness16(
        &lighten_or_comet(
            &paths,
            TrailsMethod::Comet {
                tail_frames: 2,
                reverse: false,
            },
        ),
        at.0,
        at.1,
    );
    let long = brightness16(
        &lighten_or_comet(
            &paths,
            TrailsMethod::Comet {
                tail_frames: 30,
                reverse: false,
            },
        ),
        at.0,
        at.1,
    );
    assert!(long > 2.0 * short, "{long} vs {short}");
}

#[test]
fn gap_filling_needs_two_frames() {
    let (_dir, paths) = lights();
    let result = run(&paths[..1], settings(TrailsMethod::GapFilling), 2);
    assert!(matches!(
        result,
        Err(stary_pipeline::PipelineError::NotEnoughFrames)
    ));
}
