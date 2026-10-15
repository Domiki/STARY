/// Turns every patch of one kind that is smaller than `smallest` pixels into the other kind:
/// specks of sky inside the ground and specks of ground inside the sky are noise.
pub fn without_specks(region: &mut [bool], width: usize, height: usize, smallest: usize) {
    let mut seen = vec![false; region.len()];
    let mut stack = Vec::new();
    let mut patch = Vec::new();
    for start in 0..region.len() {
        if seen[start] {
            continue;
        }
        let kind = region[start];
        seen[start] = true;
        stack.push(start);
        patch.clear();
        while let Some(index) = stack.pop() {
            patch.push(index);
            let (x, y) = (index % width, index / width);
            let neighbours = [
                (x > 0).then(|| index - 1),
                (x + 1 < width).then(|| index + 1),
                (y > 0).then(|| index - width),
                (y + 1 < height).then(|| index + width),
            ];
            for next in neighbours.into_iter().flatten() {
                if !seen[next] && region[next] == kind {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        if patch.len() < smallest {
            for &index in &patch {
                region[index] = !kind;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specks_are_absorbed_and_large_patches_stay() {
        let (width, height) = (12, 12);
        let mut region: Vec<bool> = (0..width * height).map(|index| index / width < 6).collect();
        region[8 * width + 3] = true;
        region[2 * width + 7] = false;
        without_specks(&mut region, width, height, 4);
        assert!(!region[8 * width + 3]);
        assert!(region[2 * width + 7]);
        assert!(region[0] && !region[11 * width + 11]);
    }
}
