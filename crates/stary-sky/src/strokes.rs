/// What a brush stroke says about the pixels under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    Sky,
    Ground,
    /// The border near here is to be found again at a finer scale.
    Refine,
}

/// A stroke of a round brush over a picture, in pixels of the picture (a pixel covers `[x, x + 1)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub paint: Paint,
    pub radius: f32,
    pub points: Vec<[f32; 2]>,
}

fn distance_to_segment(point: [f32; 2], from: [f32; 2], to: [f32; 2]) -> f32 {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let length = dx * dx + dy * dy;
    let along = if length > 0.0 {
        (((point[0] - from[0]) * dx + (point[1] - from[1]) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point[0] - (from[0] + along * dx)).hypot(point[1] - (from[1] + along * dy))
}

impl Stroke {
    fn segments(&self) -> impl Iterator<Item = ([f32; 2], [f32; 2])> + '_ {
        match self.points.as_slice() {
            [] => Vec::new(),
            [only] => vec![(*only, *only)],
            points => points.windows(2).map(|pair| (pair[0], pair[1])).collect(),
        }
        .into_iter()
    }

    /// The smallest box around the stroke, as pixel ranges clipped to a `width` x `height` picture.
    pub fn bounds(&self, width: usize, height: usize) -> Option<(usize, usize, usize, usize)> {
        let first = self.points.first()?;
        let (mut low, mut high) = (*first, *first);
        for point in &self.points {
            low = [low[0].min(point[0]), low[1].min(point[1])];
            high = [high[0].max(point[0]), high[1].max(point[1])];
        }
        let clip = |value: f32, limit: usize| value.clamp(0.0, limit as f32) as usize;
        let (left, top) = (
            clip((low[0] - self.radius).floor(), width),
            clip((low[1] - self.radius).floor(), height),
        );
        let (right, bottom) = (
            clip((high[0] + self.radius).ceil(), width),
            clip((high[1] + self.radius).ceil(), height),
        );
        (right > left && bottom > top).then_some((left, top, right, bottom))
    }

    fn covers(&self, point: [f32; 2]) -> bool {
        self.segments()
            .any(|(from, to)| distance_to_segment(point, from, to) <= self.radius)
    }
}

/// What the strokes say about each pixel of a `width` x `height` grid whose pixels are `scale`
/// pixels of the picture wide. A later stroke has the last word.
pub fn raster(
    strokes: &[Stroke],
    width: usize,
    height: usize,
    scale: f32,
    only: impl Fn(Paint) -> bool,
) -> Vec<Option<Paint>> {
    let mut grid = vec![None; width * height];
    for stroke in strokes.iter().filter(|stroke| only(stroke.paint)) {
        let Some((left, top, right, bottom)) = stroke.bounds(
            width * scale.ceil() as usize,
            height * scale.ceil() as usize,
        ) else {
            continue;
        };
        let (first_x, last_x) = (
            (left as f32 / scale) as usize,
            ((right as f32 / scale).ceil() as usize).min(width),
        );
        let (first_y, last_y) = (
            (top as f32 / scale) as usize,
            ((bottom as f32 / scale).ceil() as usize).min(height),
        );
        for y in first_y..last_y {
            for x in first_x..last_x {
                let centre = [(x as f32 + 0.5) * scale, (y as f32 + 0.5) * scale];
                if stroke.covers(centre) {
                    grid[y * width + x] = Some(stroke.paint);
                }
            }
        }
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(paint: Paint, radius: f32, points: &[[f32; 2]]) -> Stroke {
        Stroke {
            paint,
            radius,
            points: points.to_vec(),
        }
    }

    #[test]
    fn a_dab_covers_a_disc_of_pixels() {
        let grid = raster(
            &[stroke(Paint::Sky, 2.0, &[[5.0, 5.0]])],
            10,
            10,
            1.0,
            |_| true,
        );
        assert_eq!(grid[4 * 10 + 4], Some(Paint::Sky));
        assert_eq!(grid[5 * 10 + 6], Some(Paint::Sky));
        assert_eq!(grid[0], None);
        assert_eq!(grid[5 * 10 + 8], None);
    }

    #[test]
    fn a_line_covers_everything_along_it() {
        let grid = raster(
            &[stroke(Paint::Ground, 1.0, &[[1.0, 5.0], [9.0, 5.0]])],
            10,
            10,
            1.0,
            |_| true,
        );
        assert!((1..9).all(|x| grid[4 * 10 + x] == Some(Paint::Ground)));
        assert_eq!(grid[0], None);
    }

    #[test]
    fn a_later_stroke_wins_where_they_overlap() {
        let strokes = [
            stroke(Paint::Sky, 3.0, &[[5.0, 5.0]]),
            stroke(Paint::Ground, 1.0, &[[5.0, 5.0]]),
        ];
        let grid = raster(&strokes, 10, 10, 1.0, |_| true);
        assert_eq!(grid[5 * 10 + 5], Some(Paint::Ground));
        assert_eq!(grid[5 * 10 + 7], Some(Paint::Sky));
    }

    #[test]
    fn a_coarser_grid_covers_the_pixels_of_the_picture_it_stands_for() {
        let grid = raster(
            &[stroke(Paint::Sky, 1.5, &[[10.0, 10.0]])],
            10,
            10,
            2.0,
            |_| true,
        );
        assert_eq!(grid[5 * 10 + 5], Some(Paint::Sky));
        assert_eq!(grid[2 * 10 + 2], None);
    }

    #[test]
    fn only_the_wanted_paints_are_taken() {
        let grid = raster(
            &[stroke(Paint::Refine, 3.0, &[[5.0, 5.0]])],
            10,
            10,
            1.0,
            |paint| paint != Paint::Refine,
        );
        assert!(grid.iter().all(Option::is_none));
    }
}
