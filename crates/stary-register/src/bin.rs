/// Averages blocks of 2x2 samples. An odd last row or column is left out.
pub fn bin_by_two(data: &[f32], width: usize, height: usize) -> (Vec<f32>, usize, usize) {
    let (out_width, out_height) = (width / 2, height / 2);
    let mut out = vec![0.0f32; out_width * out_height];
    for (row, line) in out.chunks_mut(out_width).enumerate() {
        let (top, bottom) = (
            &data[2 * row * width..2 * row * width + 2 * out_width],
            &data[(2 * row + 1) * width..(2 * row + 1) * width + 2 * out_width],
        );
        for (column, value) in line.iter_mut().enumerate() {
            *value = 0.25
                * (top[2 * column]
                    + top[2 * column + 1]
                    + bottom[2 * column]
                    + bottom[2 * column + 1]);
        }
    }
    (out, out_width, out_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_averaged_and_odd_edges_dropped() {
        let data: Vec<f32> = (0..15).map(|v| v as f32).collect(); // 5 wide, 3 high
        let (out, width, height) = bin_by_two(&data, 5, 3);
        assert_eq!((width, height), (2, 1));
        assert_eq!(out, vec![3.0, 5.0]);
    }
}
