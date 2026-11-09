use std::path::{Path, PathBuf};

use stary_core::{BayerPattern, CfaPattern, Frame, PixelLayout};
use stary_io::{
    fits::{Pixels, write_fits},
    master_file::write_calibrated_light,
    read_preview,
};
use stary_pipeline::{
    CalSource, Cancel, Device, JobKind, JobOutput, JobReport, JobSpec, PipelineError, ReadMode,
    RunOptions, StackSettings, TrailsMethod, run_job,
};
use stary_raw::DemosaicQuality;

const SIZE: usize = 8;

fn bayer_frame(level: impl Fn(usize, usize) -> u16) -> Frame {
    Frame {
        width: SIZE,
        height: SIZE,
        layout: PixelLayout::Cfa(CfaPattern::Bayer(BayerPattern::Rggb)),
        black: 0,
        maximum: u16::MAX,
        samples: (0..SIZE * SIZE)
            .map(|i| level(i / SIZE, i % SIZE))
            .collect(),
    }
}

fn write_light(dir: &Path, name: &str, frame: &Frame) -> PathBuf {
    let path = dir.join(name);
    write_calibrated_light(&path, frame).unwrap();
    path
}

fn spec(lights: Vec<PathBuf>, result_folder: &Path) -> JobSpec {
    JobSpec {
        kind: JobKind::Trails,
        lights,
        bias: CalSource::None,
        dark: CalSource::None,
        dark_flat: CalSource::None,
        flat: CalSource::None,
        more_sessions: Vec::new(),
        apply_calibration: true,
        masters_folder: None,
        calibrated_folder: None,
        result_folder: Some(result_folder.to_path_buf()),
        quality: DemosaicQuality::Fast,
        trails: TrailsMethod::Lighten,
        stacking: StackSettings::default(),
        memory_percent: 50,
        masking: None,
        device: Device::Cpu,
        read: ReadMode::Exact,
        hdr: false,
    }
}

fn run(spec: &JobSpec) -> Result<JobReport, PipelineError> {
    run_job(spec, &RunOptions::default(), &Cancel::default(), &|_| {})
}

#[test]
fn trails_of_fits_lights_keeps_the_brightest_value_of_each_sample() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let lights = vec![
        write_light(dir.path(), "a.fits", &bayer_frame(|_, _| 1000)),
        write_light(
            dir.path(),
            "b.fits",
            &bayer_frame(|row, column| if (row, column) == (3, 3) { 40000 } else { 500 }),
        ),
    ];
    let outputs = run(&spec(lights, &out)).unwrap().outputs;
    let [JobOutput::Result(path)] = outputs.as_slice() else {
        panic!("expected one result, got {outputs:?}");
    };
    let preview = read_preview(path, 100).unwrap();
    assert_eq!((preview.width, preview.height), (SIZE as u32, SIZE as u32));
}

#[test]
fn a_dark_made_of_fits_frames_is_subtracted_from_fits_lights() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let calibrated = dir.path().join("calibrated");
    let darks: Vec<PathBuf> = (0..3)
        .map(|index| {
            write_light(
                dir.path(),
                &format!("dark{index}.fits"),
                &bayer_frame(|_, _| 300),
            )
        })
        .collect();
    let lights = vec![write_light(
        dir.path(),
        "light.fits",
        &bayer_frame(|_, _| 1300),
    )];
    let mut job = spec(lights, &out);
    job.dark = CalSource::Frames(darks);
    job.calibrated_folder = Some(calibrated.clone());
    let outputs = run(&job).unwrap().outputs;
    assert!(
        outputs
            .iter()
            .any(|output| matches!(output, JobOutput::Calibrated(_)))
    );
    let saved = stary_io::decode_frame(&calibrated.join("light.fits")).unwrap();
    assert!(saved.samples.iter().all(|value| *value == 1000));
}

#[test]
fn frames_of_different_layouts_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mono = dir.path().join("mono.fits");
    write_fits(&mono, 4, 4, 1, &Pixels::U16(&[10; 16]), &[]).unwrap();
    let lights = vec![
        write_light(dir.path(), "bayer.fits", &bayer_frame(|_, _| 1000)),
        mono,
    ];
    let result = run(&spec(lights, &dir.path().join("out")));
    assert!(matches!(result, Err(PipelineError::Geometry { .. })));
}
