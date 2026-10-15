use crate::{
    star::Star,
    transform::{Model, Point, Transform, fit},
    triangles::{Triangle, triangles},
};

#[derive(Debug, Clone, Copy)]
pub struct AlignParams {
    pub model: Model,
    /// How many of the brightest stars are used to find the first match.
    pub match_stars: usize,
    pub neighbours: usize,
    pub triangle_tolerance: f64,
    /// A star counts as matched when it lands this close (pixels) to its partner in the end.
    pub inlier_distance: f64,
    /// How far apart partners may be at the start, before the first fits have corrected the
    /// transform. The distance shrinks from here to `inlier_distance`.
    pub start_distance: f64,
    pub min_inliers: usize,
    pub scale_range: (f64, f64),
}

impl Default for AlignParams {
    fn default() -> Self {
        Self {
            model: Model::Similarity,
            match_stars: 70,
            neighbours: 6,
            triangle_tolerance: 0.012,
            inlier_distance: 1.5,
            start_distance: 10.0,
            min_inliers: 10,
            scale_range: (0.9, 1.1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    /// Maps coordinates of the frame onto the reference.
    pub transform: Transform,
    pub inliers: usize,
    pub rms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AlignError {
    #[error("not enough stars to align")]
    TooFewStars,
    #[error("the stars of the frame do not match the reference")]
    NoMatch,
}

const MAX_CANDIDATES: usize = 40;
const SCORE_STARS: usize = 150;
const REFINE_ROUNDS: usize = 8;
const MIN_PAIR_SEPARATION: f64 = 25.0;
const MIN_STARS: usize = 5;
const SIMILARITY_ROUNDS: usize = 3;

fn point(star: &Star) -> Point {
    (star.x, star.y)
}

fn votes_between(
    reference: &[Triangle],
    frame: &[Triangle],
    params: &AlignParams,
) -> Vec<(usize, usize, u32)> {
    let size = reference
        .iter()
        .flat_map(|t| t.vertices)
        .max()
        .map_or(0, |max| max + 1);
    let frame_size = frame
        .iter()
        .flat_map(|t| t.vertices)
        .max()
        .map_or(0, |max| max + 1);
    let mut votes = vec![0u32; size * frame_size];
    let tolerance = params.triangle_tolerance;
    for r in reference {
        for f in frame {
            if r.orientation != f.orientation
                || (r.short_ratio - f.short_ratio).abs() > tolerance
                || (r.middle_ratio - f.middle_ratio).abs() > tolerance
            {
                continue;
            }
            let scale = r.longest / f.longest;
            if scale < params.scale_range.0 || scale > params.scale_range.1 {
                continue;
            }
            for (a, b) in r.vertices.iter().zip(&f.vertices) {
                votes[a * frame_size + b] += 1;
            }
        }
    }
    let mut pairs: Vec<(usize, usize, u32)> = votes
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(index, count)| (index / frame_size, index % frame_size, *count))
        .collect();
    pairs.sort_by_key(|pair| std::cmp::Reverse(pair.2));
    pairs
}

fn similarity_from_two(
    frame: [Point; 2],
    reference: [Point; 2],
    params: &AlignParams,
) -> Option<Transform> {
    let (dx, dy) = (frame[1].0 - frame[0].0, frame[1].1 - frame[0].1);
    let (du, dv) = (
        reference[1].0 - reference[0].0,
        reference[1].1 - reference[0].1,
    );
    let length = dx.hypot(dy);
    if length < MIN_PAIR_SEPARATION {
        return None;
    }
    let (a, b) = (
        (du * dx + dv * dy) / (length * length),
        (dv * dx - du * dy) / (length * length),
    );
    let scale = a.hypot(b);
    if scale < params.scale_range.0 || scale > params.scale_range.1 {
        return None;
    }
    Some(Transform::from_affine(
        a,
        -b,
        reference[0].0 - (a * frame[0].0 - b * frame[0].1),
        b,
        a,
        reference[0].1 - (b * frame[0].0 + a * frame[0].1),
    ))
}

fn nearest(target: Point, stars: &[Star]) -> Option<(usize, f64)> {
    stars
        .iter()
        .enumerate()
        .map(|(index, star)| (index, (star.x - target.0).hypot(star.y - target.1)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn count_inliers(
    transform: &Transform,
    frame: &[Star],
    reference: &[Star],
    distance: f64,
) -> usize {
    frame
        .iter()
        .take(SCORE_STARS)
        .filter(|star| {
            nearest(transform.apply(point(star)), reference).is_some_and(|(_, d)| d <= distance)
        })
        .count()
}

fn best_hypothesis(
    candidates: &[(usize, usize)],
    frame: &[Star],
    reference: &[Star],
    params: &AlignParams,
) -> Option<(Transform, usize)> {
    let mut best: Option<(Transform, usize)> = None;
    for (position, &(ra, fa)) in candidates.iter().enumerate() {
        for &(rb, fb) in &candidates[position + 1..] {
            if ra == rb || fa == fb {
                continue;
            }
            let Some(transform) = similarity_from_two(
                [point(&frame[fa]), point(&frame[fb])],
                [point(&reference[ra]), point(&reference[rb])],
                params,
            ) else {
                continue;
            };
            let inliers = count_inliers(&transform, frame, reference, params.inlier_distance);
            if best.is_none_or(|(_, count)| inliers > count) {
                best = Some((transform, inliers));
            }
        }
    }
    best
}

fn matched_pairs(
    transform: &Transform,
    frame: &[Star],
    reference: &[Star],
    distance: f64,
) -> Vec<(Point, Point)> {
    frame
        .iter()
        .filter_map(|star| {
            let moved = transform.apply(point(star));
            let (index, d) = nearest(moved, reference)?;
            (d <= distance).then(|| (point(star), point(&reference[index])))
        })
        .collect()
}

fn rms_of(transform: &Transform, pairs: &[(Point, Point)]) -> f64 {
    let total: f64 = pairs
        .iter()
        .map(|(from, to)| {
            let moved = transform.apply(*from);
            (moved.0 - to.0).powi(2) + (moved.1 - to.1).powi(2)
        })
        .sum();
    (total / pairs.len().max(1) as f64).sqrt()
}

fn refine(
    start: Transform,
    frame: &[Star],
    reference: &[Star],
    params: &AlignParams,
) -> Option<(Transform, Vec<(Point, Point)>)> {
    let mut transform = start;
    let mut result = None;
    for round in 0..REFINE_ROUNDS {
        let fraction = round as f64 / (REFINE_ROUNDS - 1) as f64;
        let tolerance =
            params.start_distance * (params.inlier_distance / params.start_distance).powf(fraction);
        let pairs = matched_pairs(&transform, frame, reference, tolerance);
        let model = if round < SIMILARITY_ROUNDS {
            Model::Similarity
        } else {
            params.model
        };
        let Some(fitted) = fit(model, &pairs) else {
            break;
        };
        transform = fitted;
        result = Some((transform, pairs));
    }
    result
}

/// Finds how the stars of `frame` map onto those of `reference` (both brightest first).
pub fn align(
    reference: &[Star],
    frame: &[Star],
    params: &AlignParams,
) -> Result<Alignment, AlignError> {
    if reference.len() < MIN_STARS || frame.len() < MIN_STARS {
        return Err(AlignError::TooFewStars);
    }
    let top = |stars: &[Star]| stars.len().min(params.match_stars);
    let reference_triangles = triangles(&reference[..top(reference)], params.neighbours);
    let frame_triangles = triangles(&frame[..top(frame)], params.neighbours);
    let votes = votes_between(&reference_triangles, &frame_triangles, params);
    let strongest = votes.first().map_or(0, |vote| vote.2);
    let floor = (strongest / 4).max(2);
    let candidates: Vec<(usize, usize)> = votes
        .iter()
        .filter(|vote| vote.2 >= floor)
        .take(MAX_CANDIDATES)
        .map(|vote| (vote.0, vote.1))
        .collect();
    let (start, _) =
        best_hypothesis(&candidates, frame, reference, params).ok_or(AlignError::NoMatch)?;
    let (transform, pairs) = refine(start, frame, reference, params).ok_or(AlignError::NoMatch)?;
    if pairs.len() < params.min_inliers {
        return Err(AlignError::NoMatch);
    }
    Ok(Alignment {
        transform,
        inliers: pairs.len(),
        rms: rms_of(&transform, &pairs),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }

        fn gauss(&mut self) -> f64 {
            let (a, b) = (self.next().max(1e-12), self.next());
            (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
        }
    }

    fn star_at(x: f64, y: f64, flux: f32) -> Star {
        Star {
            x,
            y,
            flux,
            peak: flux,
            fwhm: 3.0,
            roundness: 1.0,
        }
    }

    fn sky(rng: &mut Lcg, count: usize) -> Vec<Star> {
        let mut stars: Vec<Star> = (0..count)
            .map(|_| {
                star_at(
                    rng.next() * 6000.0,
                    rng.next() * 4000.0,
                    (100.0 + rng.next() * 50000.0) as f32,
                )
            })
            .collect();
        stars.sort_by(|a, b| b.flux.total_cmp(&a.flux));
        stars
    }

    /// The reference stars as seen by a frame whose coordinates map onto the reference by `truth`.
    fn observe(
        reference: &[Star],
        truth: &Transform,
        rng: &mut Lcg,
        keep: f64,
        noise: f64,
        spurious: usize,
    ) -> Vec<Star> {
        let back = truth.inverse().unwrap();
        let mut frame = Vec::new();
        for star in reference {
            if rng.next() >= keep {
                continue;
            }
            let (x, y) = back.apply((star.x, star.y));
            let (x, y) = (x + noise * rng.gauss(), y + noise * rng.gauss());
            if (0.0..6000.0).contains(&x) && (0.0..4000.0).contains(&y) {
                frame.push(star_at(x, y, star.flux * (0.8 + 0.4 * rng.next()) as f32));
            }
        }
        frame.extend((0..spurious).map(|_| {
            star_at(
                rng.next() * 6000.0,
                rng.next() * 4000.0,
                (100.0 + rng.next() * 30000.0) as f32,
            )
        }));
        frame.sort_by(|a, b| b.flux.total_cmp(&a.flux));
        frame
    }

    fn mean_error(estimate: &Transform, truth: &Transform) -> f64 {
        let probes = [
            (500.0, 400.0),
            (5500.0, 300.0),
            (3000.0, 2000.0),
            (600.0, 3600.0),
            (5400.0, 3700.0),
        ];
        probes
            .iter()
            .map(|&p| {
                let (a, b) = (estimate.apply(p), truth.apply(p));
                (a.0 - b.0).hypot(a.1 - b.1)
            })
            .sum::<f64>()
            / probes.len() as f64
    }

    #[test]
    fn a_shift_and_a_small_rotation_are_recovered_with_missing_and_spurious_stars() {
        let mut rng = Lcg(42);
        let reference = sky(&mut rng, 300);
        let angle = 0.6f64.to_radians();
        let truth = Transform::from_affine(
            angle.cos(),
            -angle.sin(),
            -83.4,
            angle.sin(),
            angle.cos(),
            47.9,
        );
        let frame = observe(&reference, &truth, &mut rng, 0.7, 0.1, 40);
        let alignment = align(&reference, &frame, &AlignParams::default()).unwrap();
        assert!(alignment.inliers >= 100, "{}", alignment.inliers);
        assert!(alignment.rms < 0.3, "{}", alignment.rms);
        assert!(
            mean_error(&alignment.transform, &truth) < 0.1,
            "{}",
            mean_error(&alignment.transform, &truth)
        );
    }

    #[test]
    fn the_identity_is_found_when_nothing_moved() {
        let mut rng = Lcg(7);
        let reference = sky(&mut rng, 200);
        let frame = observe(&reference, &Transform::IDENTITY, &mut rng, 0.9, 0.05, 10);
        let alignment = align(&reference, &frame, &AlignParams::default()).unwrap();
        assert!(mean_error(&alignment.transform, &Transform::IDENTITY) < 0.05);
    }

    #[test]
    fn an_affine_model_recovers_a_slight_stretch() {
        let mut rng = Lcg(99);
        let reference = sky(&mut rng, 300);
        let truth = Transform::from_affine(1.002, 0.004, 20.0, -0.003, 0.999, -15.0);
        let frame = observe(&reference, &truth, &mut rng, 0.8, 0.08, 20);
        let params = AlignParams {
            model: Model::Affine,
            ..AlignParams::default()
        };
        let alignment = align(&reference, &frame, &params).unwrap();
        assert!(
            mean_error(&alignment.transform, &truth) < 0.15,
            "{}",
            mean_error(&alignment.transform, &truth)
        );
    }

    #[test]
    fn unrelated_skies_do_not_match() {
        let mut rng = Lcg(5);
        let a = sky(&mut rng, 200);
        let b = sky(&mut rng, 200);
        assert_eq!(
            align(&a, &b, &AlignParams::default()),
            Err(AlignError::NoMatch)
        );
    }

    #[test]
    fn a_handful_of_stars_is_not_enough() {
        let mut rng = Lcg(1);
        let a = sky(&mut rng, 4);
        let b = sky(&mut rng, 100);
        assert_eq!(
            align(&a, &b, &AlignParams::default()),
            Err(AlignError::TooFewStars)
        );
    }
}
