use serde::{Deserialize, Serialize};
use stary_sky::{Paint, Stroke};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaintWire {
    Sky,
    Ground,
    Refine,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StrokeWire {
    pub paint: PaintWire,
    pub radius: f32,
    pub points: Vec<[f32; 2]>,
}

impl From<StrokeWire> for Stroke {
    fn from(wire: StrokeWire) -> Self {
        Self {
            paint: match wire.paint {
                PaintWire::Sky => Paint::Sky,
                PaintWire::Ground => Paint::Ground,
                PaintWire::Refine => Paint::Refine,
            },
            radius: wire.radius,
            points: wire.points,
        }
    }
}

/// What the screen is told about the sky that was found; the sky itself follows as bytes, 255 for
/// sky and 0 for ground, row by row.
#[derive(Debug, Serialize)]
pub struct SkyHeader {
    pub width: usize,
    pub height: usize,
    pub sky_fraction: f32,
    pub contours: Vec<Vec<[f32; 2]>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stroke_from_the_screen_becomes_a_stroke_of_the_detector() {
        let wire: StrokeWire =
            serde_json::from_str(r#"{"paint":"refine","radius":12.5,"points":[[1,2],[3,4]]}"#)
                .unwrap();
        let stroke = Stroke::from(wire);
        assert_eq!(stroke.paint, Paint::Refine);
        assert_eq!(stroke.points, vec![[1.0, 2.0], [3.0, 4.0]]);
    }
}
