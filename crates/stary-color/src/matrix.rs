pub type Vector = [f64; 3];
pub type Matrix = [[f64; 3]; 3];

#[cfg(test)]
pub const IDENTITY: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| (0..3).map(|k| a[row][k] * b[k][column]).sum())
    })
}

pub fn apply(matrix: &Matrix, vector: &Vector) -> Vector {
    std::array::from_fn(|row| (0..3).map(|k| matrix[row][k] * vector[k]).sum())
}

pub fn inverse(m: &Matrix) -> Option<Matrix> {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
        let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let determinant = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum::<f64>();
    if determinant.abs() < 1e-12 {
        return None;
    }
    Some(std::array::from_fn(|row| {
        std::array::from_fn(|column| cofactor(column, row) / determinant)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matrix_times_its_inverse_is_the_identity() {
        let m = [[2.0, 0.5, 0.1], [0.3, 1.5, -0.2], [0.0, 0.4, 1.1]];
        let product = multiply(&m, &inverse(&m).unwrap());
        for (row, line) in product.iter().enumerate() {
            for (column, value) in line.iter().enumerate() {
                assert!((value - IDENTITY[row][column]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn a_flat_matrix_has_no_inverse() {
        assert!(inverse(&[[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 1.0, 0.0]]).is_none());
    }
}
