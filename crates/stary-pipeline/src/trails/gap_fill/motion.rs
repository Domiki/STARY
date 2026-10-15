use rayon::prelude::*;
use stary_register::{AlignParams, Star, Transform, align};

const SINGULAR: f64 = 1e-12;

/// How the stars move from each of the named lights to the next: `None` where their stars did
/// not match.
pub fn pair_motions(
    stars: &[Vec<Star>],
    pairs: &[usize],
    params: &AlignParams,
) -> Vec<Option<Transform>> {
    pairs
        .par_iter()
        .map(|&index| {
            align(&stars[index + 1], &stars[index], params)
                .ok()
                .map(|alignment| alignment.transform)
        })
        .collect()
}

fn normalized(transform: &Transform) -> Option<[[f64; 3]; 3]> {
    let scale = transform.m[2][2];
    (scale.abs() > SINGULAR).then(|| transform.m.map(|row| row.map(|value| value / scale)))
}

/// The motion most of the pairs agree on: every entry of the matrix is the median of the entries
/// of the pairs that were found, so a pair that matched wrongly does not count.
pub fn typical_motion(motions: &[Option<Transform>]) -> Option<Transform> {
    let found: Vec<[[f64; 3]; 3]> = motions.iter().flatten().filter_map(normalized).collect();
    if found.is_empty() {
        return None;
    }
    let mut m = [[0.0; 3]; 3];
    for (row, line) in m.iter_mut().enumerate() {
        for (column, value) in line.iter_mut().enumerate() {
            let mut entries: Vec<f64> = found.iter().map(|matrix| matrix[row][column]).collect();
            entries.sort_by(f64::total_cmp);
            *value = entries[entries.len() / 2];
        }
    }
    Some(Transform { m })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typical_motion_ignores_a_pair_that_matched_wrongly() {
        let shift = |dx: f64| Some(Transform::from_affine(1.0, 0.0, dx, 0.0, 1.0, 0.0));
        let motions = [shift(5.0), shift(5.1), None, shift(400.0), shift(4.9)];
        let typical = typical_motion(&motions).unwrap();
        assert!((typical.m[0][2] - 5.0).abs() < 0.2, "{:?}", typical.m);
    }

    #[test]
    fn without_any_found_pair_there_is_no_motion() {
        assert!(typical_motion(&[None, None]).is_none());
        assert!(typical_motion(&[]).is_none());
    }
}
