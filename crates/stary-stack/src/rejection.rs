#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rejection {
    None,
    Sigma { low: f32, high: f32 },
    Winsorized { low: f32, high: f32 },
}

const MAX_ROUNDS: usize = 10;
const WINSOR_LIMIT: f64 = 1.5;
const WINSOR_CORRECTION: f64 = 1.134;
const WINSOR_CONVERGENCE: f64 = 0.0005;
const MIN_FOR_REJECTION: usize = 3;

fn median(values: &mut [f32]) -> f32 {
    let middle = values.len() / 2;
    let (_, value, _) = values.select_nth_unstable_by(middle, f32::total_cmp);
    *value
}

fn mean_and_deviation(values: &[f32]) -> (f64, f64) {
    let count = values.len() as f64;
    let mean = values.iter().map(|value| f64::from(*value)).sum::<f64>() / count;
    let variance = values
        .iter()
        .map(|value| (f64::from(*value) - mean).powi(2))
        .sum::<f64>()
        / count;
    (mean, variance.sqrt())
}

/// A robust estimate of the centre and spread: the values are pulled in to 1.5 sigma around the
/// centre until the spread stops changing.
fn winsorized_estimate(values: &[f32], scratch: &mut Vec<f32>) -> (f64, f64) {
    scratch.clear();
    scratch.extend_from_slice(values);
    let mut center = f64::from(median(scratch));
    let (_, mut sigma) = mean_and_deviation(values);
    for _ in 0..MAX_ROUNDS {
        if sigma <= 0.0 {
            break;
        }
        let (low, high) = (center - WINSOR_LIMIT * sigma, center + WINSOR_LIMIT * sigma);
        scratch.clear();
        scratch.extend(
            values
                .iter()
                .map(|value| f64::from(*value).clamp(low, high) as f32),
        );
        let (mean, deviation) = mean_and_deviation(scratch);
        let next = WINSOR_CORRECTION * deviation;
        let settled = (next - sigma).abs() <= WINSOR_CONVERGENCE * sigma;
        center = mean;
        sigma = next;
        if settled {
            break;
        }
    }
    (center, sigma)
}

fn keep_within(values: &mut Vec<f32>, center: f64, sigma: f64, low: f32, high: f32) -> bool {
    let before = values.len();
    let (lower, upper) = (
        center - f64::from(low) * sigma,
        center + f64::from(high) * sigma,
    );
    values.retain(|value| (lower..=upper).contains(&f64::from(*value)));
    values.len() != before
}

/// Combines the values of one pixel taken from all frames. Frames that do not cover the pixel
/// contribute NaN and are ignored. The result is NaN when no frame covers it.
pub fn combine(values: &mut Vec<f32>, rejection: Rejection, scratch: &mut Vec<f32>) -> f32 {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return f32::NAN;
    }
    if values.len() >= MIN_FOR_REJECTION {
        match rejection {
            Rejection::None => {}
            Rejection::Sigma { low, high } => {
                for _ in 0..MAX_ROUNDS {
                    scratch.clear();
                    scratch.extend_from_slice(values);
                    let center = f64::from(median(scratch));
                    let (_, sigma) = mean_and_deviation(values);
                    if sigma <= 0.0 || !keep_within(values, center, sigma, low, high) {
                        break;
                    }
                    if values.len() < MIN_FOR_REJECTION {
                        break;
                    }
                }
            }
            Rejection::Winsorized { low, high } => {
                for _ in 0..MAX_ROUNDS {
                    let (center, sigma) = winsorized_estimate(values, scratch);
                    if sigma <= 0.0 || !keep_within(values, center, sigma, low, high) {
                        break;
                    }
                    if values.len() < MIN_FOR_REJECTION {
                        break;
                    }
                }
            }
        }
    }
    let (mean, _) = mean_and_deviation(values);
    mean as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(values: &[f32], rejection: Rejection) -> f32 {
        combine(&mut values.to_vec(), rejection, &mut Vec::new())
    }

    fn flat_with_noise(count: usize) -> Vec<f32> {
        (0..count)
            .map(|i| 100.0 + ((i * 37) % 11) as f32 - 5.0)
            .collect()
    }

    #[test]
    fn without_rejection_everything_is_averaged() {
        assert_eq!(run(&[1.0, 2.0, 6.0], Rejection::None), 3.0);
    }

    #[test]
    fn not_a_number_marks_frames_that_do_not_cover_the_pixel() {
        assert_eq!(run(&[2.0, f32::NAN, 4.0], Rejection::None), 3.0);
        assert!(run(&[f32::NAN, f32::NAN], Rejection::None).is_nan());
        assert!(run(&[], Rejection::None).is_nan());
    }

    #[test]
    fn a_bright_outlier_such_as_a_satellite_trail_is_rejected() {
        let mut values = flat_with_noise(30);
        let clean: f32 = values.iter().sum::<f32>() / 30.0;
        values[7] = 5000.0;
        for rejection in [
            Rejection::Sigma {
                low: 3.0,
                high: 3.0,
            },
            Rejection::Winsorized {
                low: 3.0,
                high: 3.0,
            },
        ] {
            let result = run(&values, rejection);
            assert!(
                (result - clean).abs() < 1.0,
                "{rejection:?}: {result} vs {clean}"
            );
        }
        assert!(run(&values, Rejection::None) > clean + 100.0);
    }

    #[test]
    fn a_dark_outlier_is_rejected_too() {
        let mut values = flat_with_noise(25);
        values[3] = -4000.0;
        let result = run(
            &values,
            Rejection::Winsorized {
                low: 3.0,
                high: 3.0,
            },
        );
        assert!((result - 100.0).abs() < 1.5, "{result}");
    }

    #[test]
    fn clean_data_loses_almost_nothing() {
        let values = flat_with_noise(40);
        let plain = run(&values, Rejection::None);
        let clipped = run(
            &values,
            Rejection::Winsorized {
                low: 3.0,
                high: 3.0,
            },
        );
        assert!((plain - clipped).abs() < 0.2);
    }

    #[test]
    fn the_sides_can_be_treated_differently() {
        let mut values = flat_with_noise(30);
        values[0] = 5000.0;
        values[1] = -5000.0;
        let only_high = run(
            &values,
            Rejection::Sigma {
                low: 100.0,
                high: 3.0,
            },
        );
        assert!(only_high < 100.0 - 100.0, "{only_high}");
        let only_low = run(
            &values,
            Rejection::Sigma {
                low: 3.0,
                high: 100.0,
            },
        );
        assert!(only_low > 100.0 + 100.0, "{only_low}");
    }

    #[test]
    fn identical_values_stay_identical() {
        assert_eq!(
            run(
                &[7.0; 10],
                Rejection::Winsorized {
                    low: 3.0,
                    high: 3.0
                }
            ),
            7.0
        );
        assert_eq!(
            run(
                &[7.0; 10],
                Rejection::Sigma {
                    low: 3.0,
                    high: 3.0
                }
            ),
            7.0
        );
    }

    #[test]
    fn two_values_are_just_averaged() {
        assert_eq!(
            run(
                &[1.0, 100.0],
                Rejection::Sigma {
                    low: 1.0,
                    high: 1.0
                }
            ),
            50.5
        );
    }
}
