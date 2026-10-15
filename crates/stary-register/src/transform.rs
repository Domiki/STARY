pub type Point = (f64, f64);

/// A 3x3 plane transform acting on column vectors `(x, y, 1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub m: [[f64; 3]; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Similarity,
    Affine,
    Homography,
}

const SINGULAR: f64 = 1e-12;

impl Transform {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub const fn from_affine(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self {
            m: [[a, b, c], [d, e, f], [0.0, 0.0, 1.0]],
        }
    }

    pub fn apply(&self, (x, y): Point) -> Point {
        let m = &self.m;
        let w = m[2][0] * x + m[2][1] * y + m[2][2];
        (
            (m[0][0] * x + m[0][1] * y + m[0][2]) / w,
            (m[1][0] * x + m[1][1] * y + m[1][2]) / w,
        )
    }

    /// The transform that first applies `self`, then `next`.
    pub fn then(&self, next: &Self) -> Self {
        let mut m = [[0.0; 3]; 3];
        for (row, line) in m.iter_mut().enumerate() {
            for (column, value) in line.iter_mut().enumerate() {
                *value = (0..3).map(|k| next.m[row][k] * self.m[k][column]).sum();
            }
        }
        Self { m }
    }

    pub fn inverse(&self) -> Option<Self> {
        let m = &self.m;
        let cofactor = |r: usize, c: usize| {
            let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
            let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
            m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
        };
        let det = m[0][0] * cofactor(0, 0) + m[0][1] * cofactor(0, 1) + m[0][2] * cofactor(0, 2);
        if det.abs() < SINGULAR {
            return None;
        }
        let mut out = [[0.0; 3]; 3];
        for (row, line) in out.iter_mut().enumerate() {
            for (column, value) in line.iter_mut().enumerate() {
                *value = cofactor(column, row) / det;
            }
        }
        Some(Self { m: out })
    }

    /// Scale of the linear part (exact for a similarity).
    pub fn scale(&self) -> f64 {
        (self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0])
            .abs()
            .sqrt()
    }

    /// Rotation of the linear part in radians (exact for a similarity).
    pub fn rotation(&self) -> f64 {
        self.m[1][0].atan2(self.m[0][0])
    }
}

/// Solves `a x = b` for a small dense system (row-major `a`, `n` unknowns).
fn solve(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&i, &j| a[i * n + column].abs().total_cmp(&a[j * n + column].abs()))?;
        if a[pivot * n + column].abs() < SINGULAR {
            return None;
        }
        if pivot != column {
            for k in 0..n {
                a.swap(pivot * n + k, column * n + k);
            }
            b.swap(pivot, column);
        }
        for row in column + 1..n {
            let factor = a[row * n + column] / a[column * n + column];
            for k in column..n {
                a[row * n + k] -= factor * a[column * n + k];
            }
            b[row] -= factor * b[column];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let tail: f64 = (row + 1..n).map(|k| a[row * n + k] * x[k]).sum();
        x[row] = (b[row] - tail) / a[row * n + row];
    }
    Some(x)
}

fn centroid(points: impl Iterator<Item = Point>) -> Point {
    let (sum, count) = points.fold(((0.0, 0.0), 0.0), |((sx, sy), n), (x, y)| {
        ((sx + x, sy + y), n + 1.0)
    });
    (sum.0 / count, sum.1 / count)
}

fn fit_similarity(pairs: &[(Point, Point)]) -> Option<Transform> {
    let (mx, my) = centroid(pairs.iter().map(|pair| pair.0));
    let (mu, mv) = centroid(pairs.iter().map(|pair| pair.1));
    let (mut denominator, mut dot, mut cross) = (0.0, 0.0, 0.0);
    for ((x, y), (u, v)) in pairs {
        let (xc, yc, uc, vc) = (x - mx, y - my, u - mu, v - mv);
        denominator += xc * xc + yc * yc;
        dot += xc * uc + yc * vc;
        cross += xc * vc - yc * uc;
    }
    if denominator < SINGULAR {
        return None;
    }
    let (a, b) = (dot / denominator, cross / denominator);
    Some(Transform::from_affine(
        a,
        -b,
        mu - a * mx + b * my,
        b,
        a,
        mv - b * mx - a * my,
    ))
}

fn fit_affine(pairs: &[(Point, Point)]) -> Option<Transform> {
    let (mx, my) = centroid(pairs.iter().map(|pair| pair.0));
    let (mu, mv) = centroid(pairs.iter().map(|pair| pair.1));
    let mut normal = vec![0.0; 9];
    let (mut rhs_u, mut rhs_v) = (vec![0.0; 3], vec![0.0; 3]);
    for ((x, y), (u, v)) in pairs {
        let row = [x - mx, y - my, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                normal[i * 3 + j] += row[i] * row[j];
            }
            rhs_u[i] += row[i] * (u - mu);
            rhs_v[i] += row[i] * (v - mv);
        }
    }
    let first = solve(normal.clone(), rhs_u, 3)?;
    let second = solve(normal, rhs_v, 3)?;
    Some(Transform::from_affine(
        first[0],
        first[1],
        mu + first[2] - first[0] * mx - first[1] * my,
        second[0],
        second[1],
        mv + second[2] - second[0] * mx - second[1] * my,
    ))
}

fn normalizer(points: impl Iterator<Item = Point> + Clone) -> Transform {
    let (cx, cy) = centroid(points.clone());
    let (sum, count) = points.fold((0.0, 0.0), |(sum, count), (x, y)| {
        (
            sum + ((x - cx).powi(2) + (y - cy).powi(2)).sqrt(),
            count + 1.0,
        )
    });
    let scale = if sum > SINGULAR {
        std::f64::consts::SQRT_2 * count / sum
    } else {
        1.0
    };
    Transform::from_affine(scale, 0.0, -scale * cx, 0.0, scale, -scale * cy)
}

fn fit_homography(pairs: &[(Point, Point)]) -> Option<Transform> {
    let from = normalizer(pairs.iter().map(|pair| pair.0));
    let to = normalizer(pairs.iter().map(|pair| pair.1));
    let mut normal = vec![0.0; 64];
    let mut rhs = vec![0.0; 8];
    for (source, target) in pairs {
        let (x, y) = from.apply(*source);
        let (u, v) = to.apply(*target);
        let rows = [
            ([x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y], u),
            ([0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y], v),
        ];
        for (row, value) in rows {
            for i in 0..8 {
                for j in 0..8 {
                    normal[i * 8 + j] += row[i] * row[j];
                }
                rhs[i] += row[i] * value;
            }
        }
    }
    let h = solve(normal, rhs, 8)?;
    let normalized = Transform {
        m: [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]],
    };
    let denormalized = from.then(&normalized).then(&to.inverse()?);
    let scale = denormalized.m[2][2];
    if scale.abs() < SINGULAR {
        return None;
    }
    let mut m = denormalized.m;
    m.iter_mut().flatten().for_each(|value| *value /= scale);
    Some(Transform { m })
}

/// The transform of the given model that best maps each `.0` onto its `.1`, in the least
/// squares sense. `None` when the points do not determine it.
pub fn fit(model: Model, pairs: &[(Point, Point)]) -> Option<Transform> {
    let needed = match model {
        Model::Similarity => 2,
        Model::Affine => 3,
        Model::Homography => 4,
    };
    if pairs.len() < needed {
        return None;
    }
    match model {
        Model::Similarity => fit_similarity(pairs),
        Model::Affine => fit_affine(pairs),
        Model::Homography => fit_homography(pairs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Vec<Point> {
        (0..5)
            .flat_map(|i| {
                (0..4).map(move |j| (f64::from(i) * 37.0 + 3.0, f64::from(j) * 51.0 + 9.0))
            })
            .collect()
    }

    fn pairs_of(transform: &Transform) -> Vec<(Point, Point)> {
        grid()
            .into_iter()
            .map(|p| (p, transform.apply(p)))
            .collect()
    }

    fn assert_close(a: &Transform, b: &Transform) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (a.m[row][column] - b.m[row][column]).abs() < 1e-6,
                    "{a:?} vs {b:?}"
                );
            }
        }
    }

    #[test]
    fn inverse_undoes_the_transform() {
        let t = Transform::from_affine(0.9, -0.3, 12.0, 0.3, 0.9, -7.0);
        let back = t.inverse().unwrap();
        let (x, y) = back.apply(t.apply((33.0, 21.0)));
        assert!((x - 33.0).abs() < 1e-9 && (y - 21.0).abs() < 1e-9);
        assert_close(&t.then(&back), &Transform::IDENTITY);
    }

    #[test]
    fn a_singular_transform_has_no_inverse() {
        assert!(
            Transform::from_affine(1.0, 2.0, 0.0, 2.0, 4.0, 0.0)
                .inverse()
                .is_none()
        );
    }

    #[test]
    fn a_similarity_is_recovered_exactly() {
        let truth = Transform::from_affine(0.99, -0.02, 14.5, 0.02, 0.99, -6.25);
        let fitted = fit(Model::Similarity, &pairs_of(&truth)).unwrap();
        assert_close(&fitted, &truth);
        assert!((fitted.scale() - (0.99f64.powi(2) + 0.02f64.powi(2)).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn an_affine_is_recovered_exactly() {
        let truth = Transform::from_affine(1.01, 0.03, -4.0, -0.02, 0.98, 8.0);
        assert_close(&fit(Model::Affine, &pairs_of(&truth)).unwrap(), &truth);
    }

    #[test]
    fn a_homography_is_recovered_exactly() {
        let truth = Transform {
            m: [[1.0, 0.02, 5.0], [-0.01, 1.0, -3.0], [1e-5, 2e-5, 1.0]],
        };
        assert_close(&fit(Model::Homography, &pairs_of(&truth)).unwrap(), &truth);
    }

    #[test]
    fn too_few_or_degenerate_points_give_nothing() {
        let one = [((1.0, 1.0), (2.0, 2.0))];
        assert!(fit(Model::Similarity, &one).is_none());
        let same = [((1.0, 1.0), (2.0, 2.0)), ((1.0, 1.0), (3.0, 3.0))];
        assert!(fit(Model::Similarity, &same).is_none());
        let line: Vec<(Point, Point)> = (0..5)
            .map(|i| ((f64::from(i), 0.0), (f64::from(i), 1.0)))
            .collect();
        assert!(fit(Model::Affine, &line).is_none());
    }

    #[test]
    fn rotation_and_scale_are_read_back() {
        let angle = 0.1f64;
        let t = Transform::from_affine(
            1.02 * angle.cos(),
            -1.02 * angle.sin(),
            0.0,
            1.02 * angle.sin(),
            1.02 * angle.cos(),
            0.0,
        );
        assert!((t.rotation() - angle).abs() < 1e-9);
        assert!((t.scale() - 1.02).abs() < 1e-9);
    }
}
