use std::sync::Arc;

use stary_gpu::{GpuContext, Pass, Placement, StackGpu, StackLayout};
use stary_register::Transform;
use stary_stack::{Kernel, Matching, SourcePlane, warp_rows};

const WIDTH: usize = 251;
const HEIGHT: usize = 187;
const COLORS: usize = 3;
const FULL_SCALE: f64 = 65535.0;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn noise(&mut self, spread: f64) -> f64 {
        (0..4).map(|_| self.next() - 0.5).sum::<f64>() * spread * 1.7
    }
}

struct TestFrame {
    planes: [Vec<u16>; COLORS],
    to_source: Transform,
    matching: [Matching; COLORS],
}

fn picture(seed: u64, trail: bool) -> [Vec<u16>; COLORS] {
    let mut rng = Lcg(seed);
    let stars = [(60.0, 50.0), (150.5, 120.5), (200.0, 40.25), (90.0, 150.0)];
    std::array::from_fn(|channel| {
        (0..WIDTH * HEIGHT)
            .map(|index| {
                let (x, y) = ((index % WIDTH) as f64, (index / WIDTH) as f64);
                let mut value = 2000.0 + 400.0 * channel as f64 + 0.8 * x + 0.5 * y;
                for &(sx, sy) in &stars {
                    let d2 = (x - sx).powi(2) + (y - sy).powi(2);
                    value += 20000.0 * (-d2 / (2.0 * 1.6 * 1.6)).exp();
                }
                if trail && (y - 90.0).abs() < 1.5 && x > 30.0 && x < 220.0 {
                    value += 9000.0;
                }
                (value + rng.noise(25.0)).clamp(0.0, 65535.0) as u16
            })
            .collect()
    })
}

fn frames(count: usize) -> Vec<TestFrame> {
    (0..count)
        .map(|index| {
            let t = index as f64;
            let to_source = match index % 3 {
                0 => Transform::from_affine(
                    1.0 + 0.0004 * t,
                    -0.0006 * t,
                    1.3 * t - 4.0,
                    0.0006 * t,
                    1.0 + 0.0004 * t,
                    -0.8 * t + 3.0,
                ),
                1 => Transform::from_affine(1.0, 0.0, 0.37 * t, 0.0, 1.0, -0.21 * t),
                _ => Transform {
                    m: [
                        [1.0002, 0.0003, 1.5 - 0.4 * t],
                        [-0.0002, 0.9998, 0.5 * t],
                        [1e-6 * t, -2e-6, 1.0],
                    ],
                },
            };
            let matching = std::array::from_fn(|channel| Matching {
                gain: (1.0 + 0.01 * (t % 5.0) - 0.01 * channel as f64) as f32,
                offset: (3.0 * (t % 7.0) - 6.0) as f32,
            });
            TestFrame {
                planes: picture(100 + index as u64, index == 4),
                to_source,
                matching,
            }
        })
        .collect()
}

fn warped(frame: &TestFrame, kernel: Kernel, channel: usize) -> Vec<f32> {
    let source = SourcePlane {
        width: WIDTH,
        height: HEIGHT,
        data: &frame.planes[channel],
    };
    let mut out = vec![0.0; WIDTH * HEIGHT];
    warp_rows(
        &source,
        &frame.to_source,
        kernel,
        0..HEIGHT,
        WIDTH,
        &mut out,
    );
    let matching = frame.matching[channel];
    out.iter()
        .map(|value| {
            if value.is_finite() {
                matching.apply(*value)
            } else {
                f32::NAN
            }
        })
        .collect()
}

fn expected(frames: &[TestFrame], kernel: Kernel, clipping: Option<(f64, f64)>) -> Vec<f32> {
    let layers: Vec<[Vec<f32>; COLORS]> = frames
        .iter()
        .map(|frame| std::array::from_fn(|channel| warped(frame, kernel, channel)))
        .collect();
    let mut out = vec![0.0f32; WIDTH * HEIGHT * COLORS];
    for index in 0..WIDTH * HEIGHT {
        for channel in 0..COLORS {
            let values: Vec<f64> = layers
                .iter()
                .map(|layer| f64::from(layer[channel][index]))
                .filter(|value| value.is_finite())
                .collect();
            let value = if values.is_empty() {
                0.0
            } else {
                let n = values.len() as f64;
                let mean = values.iter().sum::<f64>() / n;
                match clipping {
                    None => mean,
                    Some((low, high)) => {
                        let spread =
                            (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
                        let kept: Vec<f64> = values
                            .iter()
                            .copied()
                            .filter(|v| *v - mean >= -low * spread && *v - mean <= high * spread)
                            .collect();
                        if kept.is_empty() {
                            mean
                        } else {
                            kept.iter().sum::<f64>() / kept.len() as f64
                        }
                    }
                }
            };
            out[index * COLORS + channel] = (value.max(0.0) / FULL_SCALE) as f32;
        }
    }
    out
}

fn on_gpu(
    context: &Arc<GpuContext>,
    frames: &[TestFrame],
    kernel: Kernel,
    clipping: Option<(f32, f32)>,
) -> Vec<f32> {
    let mut stack = StackGpu::new(
        Arc::clone(context),
        StackLayout {
            width: WIDTH,
            height: HEIGHT,
            layers: 1,
            kernel,
        },
    )
    .unwrap();
    let passes: Vec<Pass> = [Some(Pass::Statistics)]
        .into_iter()
        .chain(clipping.map(|(low, high)| Some(Pass::Clipped { low, high })))
        .flatten()
        .collect();
    for pass in passes {
        for frame in frames {
            let [r, g, b] = &frame.planes;
            stack.upload([r, g, b]).unwrap();
            stack
                .lay(0, pass, &Placement::moved(frame.to_source, frame.matching))
                .unwrap();
        }
    }
    stack.finish_layer(0, clipping.is_some()).unwrap()
}

struct Difference {
    within_tolerance: f64,
    largest: f64,
}

fn compare(gpu: &[f32], cpu: &[f32], tolerance: f64) -> Difference {
    assert_eq!(gpu.len(), cpu.len());
    let differences: Vec<f64> = gpu
        .iter()
        .zip(cpu)
        .map(|(a, b)| (f64::from(*a) - f64::from(*b)).abs() * FULL_SCALE)
        .collect();
    Difference {
        within_tolerance: differences.iter().filter(|d| **d <= tolerance).count() as f64
            / differences.len() as f64,
        largest: differences.iter().copied().fold(0.0, f64::max),
    }
}

fn context() -> Option<Arc<GpuContext>> {
    match GpuContext::new() {
        Ok(context) => Some(Arc::new(context)),
        Err(error) => {
            eprintln!("skipped, no GPU: {error}");
            None
        }
    }
}

#[test]
fn the_mean_of_moved_frames_matches_the_cpu_for_every_kernel() {
    let Some(context) = context() else { return };
    let frames = frames(9);
    for kernel in [Kernel::Bilinear, Kernel::Bicubic, Kernel::Lanczos3] {
        let gpu = on_gpu(&context, &frames, kernel, None);
        let cpu = expected(&frames, kernel, None);
        let difference = compare(&gpu, &cpu, 0.05);
        eprintln!(
            "{kernel:?}: {:.4}% within 0.05 ADU, largest {:.3} ADU",
            difference.within_tolerance * 100.0,
            difference.largest
        );
        assert!(difference.within_tolerance > 0.995, "{kernel:?}");
        assert!(
            difference.largest < 0.2,
            "{kernel:?}: {}",
            difference.largest
        );
    }
}

#[test]
fn the_clipped_mean_matches_the_cpu() {
    let Some(context) = context() else { return };
    let frames = frames(15);
    let gpu = on_gpu(&context, &frames, Kernel::Bicubic, Some((3.0, 3.0)));
    let cpu = expected(&frames, Kernel::Bicubic, Some((3.0, 3.0)));
    let difference = compare(&gpu, &cpu, 0.05);
    eprintln!(
        "clipped: {:.4}% within 0.05 ADU, largest {:.3} ADU",
        difference.within_tolerance * 100.0,
        difference.largest
    );
    assert!(difference.within_tolerance > 0.995);
    assert!(difference.largest < 1.0, "{}", difference.largest);
}

#[test]
fn an_unmoved_layer_is_the_mean_of_the_frames_as_they_are() {
    let Some(context) = context() else { return };
    let frames = frames(6);
    let mut stack = StackGpu::new(
        Arc::clone(&context),
        StackLayout {
            width: WIDTH,
            height: HEIGHT,
            layers: 2,
            kernel: Kernel::Bicubic,
        },
    )
    .unwrap();
    for frame in &frames {
        let [r, g, b] = &frame.planes;
        stack.upload([r, g, b]).unwrap();
        stack
            .lay(
                0,
                Pass::Statistics,
                &Placement::moved(frame.to_source, frame.matching),
            )
            .unwrap();
        stack
            .lay(1, Pass::Statistics, &Placement::unmoved())
            .unwrap();
    }
    let under = stack.finish_layer(1, false).unwrap();
    let expected: Vec<f32> = (0..WIDTH * HEIGHT * COLORS)
        .map(|sample| {
            let (index, channel) = (sample / COLORS, sample % COLORS);
            let total: f64 = frames
                .iter()
                .map(|frame| f64::from(frame.planes[channel][index]))
                .sum();
            (total / frames.len() as f64 / FULL_SCALE) as f32
        })
        .collect();
    let difference = compare(&under, &expected, 0.01);
    assert!(
        difference.within_tolerance > 0.9999,
        "{}",
        difference.largest
    );
}

#[test]
fn a_frame_that_is_not_the_size_of_the_picture_is_refused() {
    let Some(context) = context() else { return };
    let mut stack = StackGpu::new(
        context,
        StackLayout {
            width: WIDTH,
            height: HEIGHT,
            layers: 1,
            kernel: Kernel::Bilinear,
        },
    )
    .unwrap();
    let small = vec![0u16; 10];
    assert!(stack.upload([&small, &small, &small]).is_err());
}
