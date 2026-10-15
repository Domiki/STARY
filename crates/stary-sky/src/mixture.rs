const MAX_SAMPLES: usize = 100_000;
const ROUNDS: usize = 40;
const MIN_DEVIATION: f32 = 1e-3;

/// Two bell curves that together describe a set of values: the dark and the bright part of a
/// picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixture {
    pub means: [f32; 2],
    pub deviations: [f32; 2],
    pub weights: [f32; 2],
}

fn percentile(sorted: &[f32], share: f32) -> f32 {
    sorted[((sorted.len() - 1) as f32 * share) as usize]
}

fn density(mean: f32, deviation: f32, value: f32) -> f32 {
    let distance = (value - mean) / deviation;
    (-0.5 * distance * distance).exp() / deviation
}

impl Mixture {
    pub fn fit(values: &[f32]) -> Self {
        let step = (values.len() / MAX_SAMPLES).max(1);
        let mut sample: Vec<f32> = values.iter().step_by(step).copied().collect();
        sample.sort_unstable_by(f32::total_cmp);
        let spread = (percentile(&sample, 0.9) - percentile(&sample, 0.1)).max(MIN_DEVIATION);
        let mut mixture = Self {
            means: [percentile(&sample, 0.15), percentile(&sample, 0.85)],
            deviations: [spread / 4.0; 2],
            weights: [0.5; 2],
        };
        for _ in 0..ROUNDS {
            mixture = mixture.refined(&sample);
        }
        mixture
    }

    fn refined(&self, sample: &[f32]) -> Self {
        let mut mass = [0.0f64; 2];
        let mut total = [0.0f64; 2];
        let mut squares = [0.0f64; 2];
        for &value in sample {
            let share = self.posteriors(value);
            for component in 0..2 {
                let responsibility = f64::from(share[component]);
                mass[component] += responsibility;
                total[component] += responsibility * f64::from(value);
                squares[component] += responsibility * f64::from(value) * f64::from(value);
            }
        }
        let mut next = *self;
        for component in 0..2 {
            if mass[component] < 1e-6 {
                continue;
            }
            let mean = total[component] / mass[component];
            let variance = (squares[component] / mass[component] - mean * mean).max(0.0);
            next.means[component] = mean as f32;
            next.deviations[component] = (variance.sqrt() as f32).max(MIN_DEVIATION);
            next.weights[component] = (mass[component] / (mass[0] + mass[1])) as f32;
        }
        next
    }

    fn posteriors(&self, value: f32) -> [f32; 2] {
        let likelihood = |component: usize| {
            self.weights[component]
                * density(self.means[component], self.deviations[component], value)
        };
        let (first, second) = (likelihood(0), likelihood(1));
        let total = (first + second).max(f32::MIN_POSITIVE);
        [first / total, second / total]
    }

    /// How likely a value belongs to a component.
    pub fn posterior(&self, component: usize, value: f32) -> f32 {
        self.posteriors(value)[component]
    }

    /// How far apart the two curves are, in their own widths. Near 0 the values are all alike.
    pub fn separation(&self) -> f32 {
        let widths = self.deviations[0].hypot(self.deviations[1]);
        (self.means[1] - self.means[0]).abs() / widths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_groups() -> Vec<f32> {
        (0..4000)
            .map(|index| {
                let wobble = ((index * 7919) % 1000) as f32 / 1000.0 - 0.5;
                if index % 5 == 0 {
                    -3.0 + 0.2 * wobble
                } else {
                    -1.5 + 0.5 * wobble
                }
            })
            .collect()
    }

    #[test]
    fn two_groups_are_found_with_their_shares() {
        let mixture = Mixture::fit(&two_groups());
        assert!((mixture.means[0] + 3.0).abs() < 0.05, "{mixture:?}");
        assert!((mixture.means[1] + 1.5).abs() < 0.05, "{mixture:?}");
        assert!((mixture.weights[0] - 0.2).abs() < 0.02, "{mixture:?}");
        assert!(mixture.separation() > 3.0);
    }

    #[test]
    fn a_value_belongs_to_the_curve_it_is_near() {
        let mixture = Mixture::fit(&two_groups());
        assert!(mixture.posterior(0, -3.0) > 0.99);
        assert!(mixture.posterior(1, -1.5) > 0.99);
    }

    #[test]
    fn one_group_is_hardly_two() {
        let one: Vec<f32> = (0..4000)
            .map(|index| ((index * 7919) % 1000) as f32 / 1000.0)
            .collect();
        assert!(Mixture::fit(&one).separation() < 2.5);
    }
}
