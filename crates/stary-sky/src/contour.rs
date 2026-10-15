use std::collections::HashMap;

/// A line along the border between sky and ground, as points on the corners of pixels.
pub type Contour = Vec<[f32; 2]>;

fn corner(width: usize, x: usize, y: usize) -> u32 {
    (y * (width + 1) + x) as u32
}

fn position(width: usize, corner: u32) -> [f32; 2] {
    let corner = corner as usize;
    [(corner % (width + 1)) as f32, (corner / (width + 1)) as f32]
}

/// The pixel edges that have sky on one side and ground on the other, by their two corners.
fn border_edges(region: &[bool], width: usize, height: usize) -> Vec<(u32, u32)> {
    let mut edges = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let here = region[y * width + x];
            if x + 1 < width && region[y * width + x + 1] != here {
                edges.push((corner(width, x + 1, y), corner(width, x + 1, y + 1)));
            }
            if y + 1 < height && region[(y + 1) * width + x] != here {
                edges.push((corner(width, x, y + 1), corner(width, x + 1, y + 1)));
            }
        }
    }
    edges
}

fn take_edge(next: &mut HashMap<u32, Vec<u32>>, from: u32, to: u32) {
    for (a, b) in [(from, to), (to, from)] {
        if let Some(list) = next.get_mut(&a)
            && let Some(at) = list.iter().position(|&corner| corner == b)
        {
            list.swap_remove(at);
        }
    }
}

fn walk(next: &mut HashMap<u32, Vec<u32>>, start: u32) -> Vec<u32> {
    let mut path = vec![start];
    let mut here = start;
    while let Some(to) = next.get(&here).and_then(|list| list.last().copied()) {
        take_edge(next, here, to);
        path.push(to);
        here = to;
    }
    path
}

fn distance_to_line(point: [f32; 2], from: [f32; 2], to: [f32; 2]) -> f32 {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return (point[0] - from[0]).hypot(point[1] - from[1]);
    }
    ((point[0] - from[0]) * dy - (point[1] - from[1]) * dx).abs() / length
}

/// Drops the points that lie within `tolerance` of the line between the points kept around them.
fn simplified(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let (first, last) = (points[0], points[points.len() - 1]);
    let (far, farthest) = points[1..points.len() - 1]
        .iter()
        .enumerate()
        .map(|(at, &point)| (at + 1, distance_to_line(point, first, last)))
        .fold((0, 0.0), |best, candidate| {
            if candidate.1 > best.1 {
                candidate
            } else {
                best
            }
        });
    if farthest <= tolerance {
        return vec![first, last];
    }
    let mut head = simplified(&points[..=far], tolerance);
    head.pop();
    head.extend(simplified(&points[far..], tolerance));
    head
}

/// The borders between sky and ground inside the picture, as lines. A line that ends at the edge of
/// the picture is open; one that meets itself is closed (its first point is repeated at the end).
pub fn contours(region: &[bool], width: usize, height: usize, tolerance: f32) -> Vec<Contour> {
    let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
    for (a, b) in border_edges(region, width, height) {
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    let mut ends: Vec<u32> = next
        .iter()
        .filter(|(_, list)| list.len() == 1)
        .map(|(&corner, _)| corner)
        .collect();
    ends.sort_unstable();
    let mut lines = Vec::new();
    for start in ends {
        if next.get(&start).is_some_and(|list| !list.is_empty()) {
            lines.push(walk(&mut next, start));
        }
    }
    let mut rest: Vec<u32> = next
        .iter()
        .filter(|(_, list)| !list.is_empty())
        .map(|(&corner, _)| corner)
        .collect();
    rest.sort_unstable();
    for start in rest {
        while next.get(&start).is_some_and(|list| !list.is_empty()) {
            lines.push(walk(&mut next, start));
        }
    }
    lines
        .into_iter()
        .map(|line| {
            let points: Vec<[f32; 2]> = line.iter().map(|&c| position(width, c)).collect();
            simplified(&points, tolerance)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn halves(width: usize, height: usize, rows: usize) -> Vec<bool> {
        (0..width * height)
            .map(|index| index / width < rows)
            .collect()
    }

    #[test]
    fn a_straight_horizon_is_one_open_line() {
        let lines = contours(&halves(20, 10, 4), 20, 10, 0.5);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 2);
        let ends = [lines[0][0], lines[0][1]];
        assert!(ends.iter().all(|point| point[1] == 4.0));
        assert!(
            ends.iter().any(|point| point[0] == 0.0) && ends.iter().any(|point| point[0] == 20.0)
        );
    }

    #[test]
    fn an_island_is_a_closed_line() {
        let (width, height) = (12, 12);
        let region: Vec<bool> = (0..width * height)
            .map(|index| (4..8).contains(&(index % width)) && (4..8).contains(&(index / width)))
            .collect();
        let lines = contours(&region, width, height, 0.5);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].first(), lines[0].last());
        assert!(lines[0].len() >= 5);
    }

    #[test]
    fn no_border_means_no_lines() {
        assert!(contours(&[true; 30], 6, 5, 0.5).is_empty());
    }

    #[test]
    fn a_staircase_is_straightened_within_the_tolerance() {
        let (width, height) = (20, 20);
        let region: Vec<bool> = (0..width * height)
            .map(|index| (index % width) as isize > (index / width) as isize)
            .collect();
        let lines = contours(&region, width, height, 0.8);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].len() <= 4, "{:?}", lines[0]);
    }
}
