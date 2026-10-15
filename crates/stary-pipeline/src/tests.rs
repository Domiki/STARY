use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use stary_calib::Master;
use stary_core::{BayerPattern, CfaPattern, Frame, PixelLayout};

use crate::*;

const WIDTH: usize = 4;
const HEIGHT: usize = 2;

fn frame(value: u16, width: usize) -> Frame {
    Frame {
        width,
        height: HEIGHT,
        layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        black: 100,
        maximum: 16383,
        samples: vec![value; width * HEIGHT],
    }
}

/// File names carry the pixel value: "v123.raf" decodes to a frame filled with 123.
fn fake_decode(path: &Path) -> Result<Frame, PipelineError> {
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("v0");
    let value: u16 = name.trim_start_matches('v').parse().unwrap_or(0);
    let width = if name.starts_with("wide") {
        WIDTH + 2
    } else {
        WIDTH
    };
    Ok(frame(value, width))
}

fn paths(values: &[&str]) -> Vec<PathBuf> {
    values
        .iter()
        .map(|name| PathBuf::from(format!("{name}.raf")))
        .collect()
}

fn options(workers: usize) -> RunOptions {
    RunOptions {
        workers,
        kappa: 2.0,
        decode: fake_decode,
        read: ReadMode::Exact,
    }
}

fn fast(workers: usize) -> RunOptions {
    RunOptions {
        read: ReadMode::Fast,
        ..options(workers)
    }
}

fn integrate(frames: &[PathBuf], options: &RunOptions) -> (Master, MasterNote) {
    integrate_master(frames, Stage::Dark, options, &Cancel::default(), &quiet).unwrap()
}

fn spread(values: &[u16]) -> Vec<PathBuf> {
    values
        .iter()
        .map(|value| PathBuf::from(format!("v{value}.raf")))
        .collect()
}

fn quiet(_: Progress) {}

#[test]
fn integrating_enough_frames_rejects_an_outlier() {
    let frames = paths(&[
        "v500", "v500", "v500", "v500", "v500", "v500", "v500", "v500", "v500", "v9000",
    ]);
    let master = build_master(
        &frames,
        Stage::Dark,
        &options(4),
        &Cancel::default(),
        &quiet,
    )
    .unwrap();
    assert_eq!(master.count, 10);
    assert!(master.samples.iter().all(|sample| *sample == 500.0));
    assert_eq!(master.black, 100);
}

#[test]
fn a_few_frames_are_averaged_without_clipping() {
    let frames = paths(&["v100", "v200", "v300"]);
    let master = build_master(
        &frames,
        Stage::Bias,
        &options(2),
        &Cancel::default(),
        &quiet,
    )
    .unwrap();
    assert!(master.samples.iter().all(|sample| *sample == 200.0));
}

#[test]
fn frames_of_different_sizes_are_refused() {
    let frames = paths(&["v100", "wide100"]);
    let result = build_master(
        &frames,
        Stage::Flat,
        &options(1),
        &Cancel::default(),
        &quiet,
    );
    assert!(matches!(result, Err(PipelineError::Geometry { .. })));
}

#[test]
fn no_input_is_an_error() {
    let result = build_master(&[], Stage::Dark, &options(1), &Cancel::default(), &quiet);
    assert!(matches!(result, Err(PipelineError::NoInput)));
}

#[test]
fn a_cancelled_run_stops_with_an_error() {
    let cancel = Cancel::default();
    cancel.request();
    let result = build_master(
        &paths(&["v1", "v2", "v3", "v4"]),
        Stage::Dark,
        &options(2),
        &cancel,
        &quiet,
    );
    assert!(result.is_err());
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn slow_decode(path: &Path) -> Result<Frame, PipelineError> {
    let now = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
    PEAK.fetch_max(now, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(15));
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
    fake_decode(path)
}

#[test]
fn never_decodes_more_frames_at_once_than_the_workers_allow() {
    let many: Vec<PathBuf> = (0..24)
        .map(|i| PathBuf::from(format!("v{i}.raf")))
        .collect();
    let options = RunOptions {
        workers: 3,
        kappa: 3.0,
        read: ReadMode::Exact,
        decode: slow_decode,
    };
    build_master(&many, Stage::Dark, &options, &Cancel::default(), &quiet).unwrap();
    assert!(
        PEAK.load(Ordering::SeqCst) <= 3,
        "peak was {}",
        PEAK.load(Ordering::SeqCst)
    );
}

fn master_of(value: f32, black: u16) -> Master {
    let geometry = stary_calib::SensorGeometry {
        width: WIDTH,
        height: HEIGHT,
        layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
    };
    Master::new(geometry, black, 5, vec![value; WIDTH * HEIGHT]).unwrap()
}

#[test]
fn calibrating_lights_removes_the_dark_and_reports_progress() {
    let inputs = CalibrationInputs {
        dark: Source::Master(master_of(130.0, 100)),
        ..Default::default()
    };
    let prepared = prepare_calibration(&inputs, &options(2), &Cancel::default(), &quiet).unwrap();
    assert!(prepared.built.is_empty());

    let steps = AtomicUsize::new(0);
    let mut results = Vec::new();
    calibrate_lights(
        &paths(&["v1100", "v1130"]),
        prepared.set.as_ref(),
        &options(2),
        &Cancel::default(),
        &|progress| {
            assert_eq!(progress.stage, Stage::Lights);
            steps.fetch_add(1, Ordering::SeqCst);
        },
        &crate::no_observer,
        |index, frame| {
            results.push((index, frame.samples[0]));
            Ok(())
        },
    )
    .unwrap();
    results.sort();
    assert_eq!(results, vec![(0, 1070), (1, 1100)]);
    assert_eq!(steps.load(Ordering::SeqCst), 2);
}

#[test]
fn masters_built_from_frames_are_returned_for_saving() {
    let inputs = CalibrationInputs {
        bias: Source::Frames(paths(&["v110", "v110", "v110", "v110"])),
        dark: Source::Frames(paths(&["v150", "v150", "v150", "v150"])),
        ..Default::default()
    };
    let prepared = prepare_calibration(&inputs, &options(2), &Cancel::default(), &quiet).unwrap();
    let kinds: Vec<_> = prepared.built.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(
        kinds,
        vec![
            stary_core::StaryKind::MasterBias,
            stary_core::StaryKind::MasterDark
        ]
    );
    assert!(prepared.set.is_some());
}

#[test]
fn without_any_calibration_data_there_is_nothing_to_apply() {
    let prepared = prepare_calibration(
        &CalibrationInputs::default(),
        &options(1),
        &Cancel::default(),
        &quiet,
    )
    .unwrap();
    assert!(prepared.set.is_none());
}

#[test]
fn flats_are_flattened_using_the_bias() {
    let inputs = CalibrationInputs {
        bias: Source::Master(master_of(110.0, 100)),
        flat: Source::Master(master_of(1110.0, 100)),
        ..Default::default()
    };
    let prepared = prepare_calibration(&inputs, &options(1), &Cancel::default(), &quiet).unwrap();
    let mut out = Vec::new();
    calibrate_lights(
        &paths(&["v2000"]),
        prepared.set.as_ref(),
        &options(1),
        &Cancel::default(),
        &quiet,
        &crate::no_observer,
        |_, frame| {
            out.push(frame.samples[0]);
            Ok(())
        },
    )
    .unwrap();
    // uniform flat: gain 1, bias signal 10 removed from the light
    assert_eq!(out, vec![1990]);
}

#[test]
fn reading_once_leaves_out_a_spoiled_frame_and_reads_each_file_once() {
    let mut values = vec![500u16; 9];
    values.push(9000);
    let (master, note) = integrate(&spread(&values), &fast(4));
    assert!(master.samples.iter().all(|sample| *sample == 500.0));
    assert_eq!(note.reads, 1);
}

#[test]
fn reading_once_takes_one_read_of_the_frames_and_clipping_takes_two() {
    let values: Vec<u16> = (0..30).map(|index| 480 + (index * 7 % 41) as u16).collect();
    let frames = spread(&values);
    let (_, once) = integrate(&frames, &fast(3));
    let (_, twice) = integrate(&frames, &options(3));
    assert_eq!((once.reads, twice.reads), (1, 2));
}

#[test]
fn reading_once_gives_nearly_the_mean_that_reading_twice_gives_for_clean_frames() {
    let values: Vec<u16> = (0..40).map(|index| 480 + (index * 7 % 41) as u16).collect();
    let frames = spread(&values);
    let (once, _) = integrate(&frames, &fast(3));
    let (twice, _) = integrate(&frames, &options(3));
    let difference = (once.samples[0] - twice.samples[0]).abs();
    assert!(
        difference < 1.0,
        "{} vs {}",
        once.samples[0],
        twice.samples[0]
    );
}

#[test]
fn too_many_frames_for_one_read_are_read_twice() {
    let frames = spread(&vec![500u16; 1200]);
    let (master, note) = integrate(&frames, &fast(4));
    assert_eq!(note.reads, 2);
    assert!(master.samples.iter().all(|sample| *sample == 500.0));
}

#[test]
fn a_few_frames_are_read_once_in_either_mode_and_say_nothing_about_room() {
    let (_, note) = integrate(&spread(&[100, 200, 300]), &fast(2));
    assert_eq!((note.reads, note.full_share), (1, 0.0));
}

#[test]
fn too_few_frames_to_leave_values_out_are_read_twice_even_when_asked_to_be_fast() {
    let (master, note) = integrate(&spread(&[500, 500, 500, 500, 500, 9000]), &fast(2));
    assert_eq!(note.reads, 2);
    assert!(master.samples.iter().all(|sample| *sample == 500.0));
}

#[test]
fn spoiled_values_beyond_the_room_show_in_the_share_of_full_pixels() {
    let mut values = vec![500u16; 30];
    values[3] = 9000;
    values[8] = 9500;
    values[20] = 8000;
    let (_, note) = integrate(&spread(&values), &fast(4));
    assert_eq!(note.reads, 1);
    assert!(note.full_share > 0.99, "{}", note.full_share);
}
