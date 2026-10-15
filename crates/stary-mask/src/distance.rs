const FAR: f64 = 1e20;

/// The squared distance of every sample of a line to the nearest zero of `line`, where the other
/// samples hold the squared distance already known for them (or `FAR`).
fn lower_envelope(line: &mut [f64]) {
    let len = line.len();
    let mut parents = vec![0usize; len];
    let mut bounds = vec![0.0f64; len + 1];
    let mut last = 0;
    bounds[0] = f64::NEG_INFINITY;
    bounds[1] = f64::INFINITY;
    let source = line.to_vec();
    for q in 1..len {
        let meeting = |p: usize| {
            ((source[q] + (q * q) as f64) - (source[p] + (p * p) as f64)) / (2.0 * (q - p) as f64)
        };
        let mut start = meeting(parents[last]);
        while start <= bounds[last] && last > 0 {
            last -= 1;
            start = meeting(parents[last]);
        }
        last += 1;
        parents[last] = q;
        bounds[last] = start;
        bounds[last + 1] = f64::INFINITY;
    }
    let mut at = 0;
    for (q, out) in line.iter_mut().enumerate() {
        while bounds[at + 1] < q as f64 {
            at += 1;
        }
        let p = parents[at];
        *out = (q as f64 - p as f64).powi(2) + source[p];
    }
}

/// How far every pixel is from the nearest pixel of the set, or `None` when the set is empty.
pub fn distance_to(set: &[bool], width: usize, height: usize) -> Option<Vec<f32>> {
    if !set.iter().any(|&inside| inside) {
        return None;
    }
    let mut squared: Vec<f64> = set
        .iter()
        .map(|&inside| if inside { 0.0 } else { FAR })
        .collect();
    let mut column = vec![0.0; height];
    for x in 0..width {
        for (y, value) in column.iter_mut().enumerate() {
            *value = squared[y * width + x];
        }
        lower_envelope(&mut column);
        for (y, value) in column.iter().enumerate() {
            squared[y * width + x] = *value;
        }
    }
    for row in squared.chunks_exact_mut(width) {
        lower_envelope(row);
    }
    Some(squared.iter().map(|value| value.sqrt() as f32).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_to_one_pixel_are_euclidean() {
        let mut set = vec![false; 5 * 5];
        set[2 * 5 + 2] = true;
        let distance = distance_to(&set, 5, 5).unwrap();
        assert_eq!(distance[2 * 5 + 2], 0.0);
        assert_eq!(distance[2 * 5 + 4], 2.0);
        assert!((distance[4 * 5 + 4] - 8.0f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn the_nearest_of_several_pixels_counts() {
        let mut set = vec![false; 10];
        set[0] = true;
        set[9] = true;
        let distance = distance_to(&set, 10, 1).unwrap();
        assert_eq!(distance[3], 3.0);
        assert_eq!(distance[6], 3.0);
    }

    #[test]
    fn an_empty_set_has_no_distances() {
        assert!(distance_to(&[false; 6], 3, 2).is_none());
    }
}
