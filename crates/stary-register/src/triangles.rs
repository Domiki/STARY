use std::collections::HashSet;

use crate::star::Star;

const MIN_LONGEST_SIDE: f64 = 12.0;
const MIN_AREA_RATIO: f64 = 0.04;

/// A triangle of stars described by numbers that do not change when the image is rotated, moved
/// or scaled. The vertices are listed by the length of the side opposite to them, shortest first.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub vertices: [usize; 3],
    pub short_ratio: f64,
    pub middle_ratio: f64,
    pub longest: f64,
    pub orientation: f64,
}

fn distance(a: &Star, b: &Star) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn describe(stars: &[Star], indices: [usize; 3]) -> Option<Triangle> {
    let [i, j, k] = indices;
    let sides = [
        (distance(&stars[j], &stars[k]), i),
        (distance(&stars[i], &stars[k]), j),
        (distance(&stars[i], &stars[j]), k),
    ];
    let mut ordered = sides;
    ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
    let longest = ordered[2].0;
    if longest < MIN_LONGEST_SIDE {
        return None;
    }
    let [a, b, c] = [ordered[0].1, ordered[1].1, ordered[2].1];
    let cross = (stars[b].x - stars[a].x) * (stars[c].y - stars[a].y)
        - (stars[b].y - stars[a].y) * (stars[c].x - stars[a].x);
    if cross.abs() < MIN_AREA_RATIO * longest * longest {
        return None;
    }
    Some(Triangle {
        vertices: [a, b, c],
        short_ratio: ordered[0].0 / longest,
        middle_ratio: ordered[1].0 / longest,
        longest,
        orientation: cross.signum(),
    })
}

/// The triangles formed by each star with pairs of its nearest neighbours.
pub fn triangles(stars: &[Star], neighbours: usize) -> Vec<Triangle> {
    let mut seen: HashSet<[usize; 3]> = HashSet::new();
    let mut out = Vec::new();
    for center in 0..stars.len() {
        let mut nearest: Vec<usize> = (0..stars.len()).filter(|&other| other != center).collect();
        nearest.sort_by(|&a, &b| {
            distance(&stars[center], &stars[a]).total_cmp(&distance(&stars[center], &stars[b]))
        });
        nearest.truncate(neighbours);
        for (position, &first) in nearest.iter().enumerate() {
            for &second in &nearest[position + 1..] {
                let mut key = [center, first, second];
                key.sort_unstable();
                if seen.insert(key) {
                    out.extend(describe(stars, key));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(x: f64, y: f64) -> Star {
        Star {
            x,
            y,
            flux: 1.0,
            peak: 1.0,
            fwhm: 3.0,
            roundness: 1.0,
        }
    }

    #[test]
    fn the_description_ignores_rotation_scale_and_position() {
        let base = [star(0.0, 0.0), star(100.0, 0.0), star(30.0, 60.0)];
        let (cos, sin, scale) = (0.8f64, 0.6f64, 1.7f64);
        let moved: Vec<Star> = base
            .iter()
            .map(|s| {
                star(
                    scale * (cos * s.x - sin * s.y) + 500.0,
                    scale * (sin * s.x + cos * s.y) - 40.0,
                )
            })
            .collect();
        let a = describe(&base, [0, 1, 2]).unwrap();
        let b = describe(&moved, [0, 1, 2]).unwrap();
        assert!((a.short_ratio - b.short_ratio).abs() < 1e-9);
        assert!((a.middle_ratio - b.middle_ratio).abs() < 1e-9);
        assert_eq!(a.vertices, b.vertices);
        assert_eq!(a.orientation, b.orientation);
        assert!((b.longest / a.longest - scale).abs() < 1e-9);
    }

    #[test]
    fn a_mirror_image_flips_the_orientation() {
        let base = [star(0.0, 0.0), star(100.0, 0.0), star(30.0, 60.0)];
        let mirrored: Vec<Star> = base.iter().map(|s| star(-s.x, s.y)).collect();
        let a = describe(&base, [0, 1, 2]).unwrap();
        let b = describe(&mirrored, [0, 1, 2]).unwrap();
        assert_eq!(a.orientation, -b.orientation);
    }

    #[test]
    fn tiny_and_flat_triangles_are_skipped() {
        let tiny = [star(0.0, 0.0), star(3.0, 0.0), star(1.0, 2.0)];
        assert!(describe(&tiny, [0, 1, 2]).is_none());
        let flat = [star(0.0, 0.0), star(100.0, 0.0), star(50.0, 0.5)];
        assert!(describe(&flat, [0, 1, 2]).is_none());
    }

    #[test]
    fn each_triangle_appears_once() {
        let stars: Vec<Star> = (0..6)
            .map(|i| star(f64::from(i) * 37.0 % 90.0, f64::from(i * i) * 23.0 % 80.0))
            .collect();
        let found = triangles(&stars, 4);
        let mut keys: Vec<[usize; 3]> = found
            .iter()
            .map(|t| {
                let mut key = t.vertices;
                key.sort_unstable();
                key
            })
            .collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(before, keys.len());
        assert!(!found.is_empty());
    }
}
