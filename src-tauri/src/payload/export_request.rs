use serde::Deserialize;
use stary_io::export::{Container, Depth, Export, Tone};

use stary_color::Space;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerWire {
    Tiff,
    Png,
    Jpeg,
    Fits,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepthWire {
    Eight,
    Sixteen,
    Float,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceWire {
    Srgb,
    DisplayP3,
    AdobeRgb,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneWire {
    Linear,
    Display,
    Stretch,
}

/// Which file to make from which result, and how.
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub source: String,
    pub target: String,
    pub container: ContainerWire,
    pub depth: DepthWire,
    /// `null` keeps the colors of the result as they are.
    pub space: Option<SpaceWire>,
    pub tone: ToneWire,
    pub quality: u8,
}

impl ExportRequest {
    pub fn export(&self) -> Export {
        Export {
            container: match self.container {
                ContainerWire::Tiff => Container::Tiff,
                ContainerWire::Png => Container::Png,
                ContainerWire::Jpeg => Container::Jpeg,
                ContainerWire::Fits => Container::Fits,
            },
            depth: match self.depth {
                DepthWire::Eight => Depth::Eight,
                DepthWire::Sixteen => Depth::Sixteen,
                DepthWire::Float => Depth::Float,
            },
            space: self.space.map(|space| match space {
                SpaceWire::Srgb => Space::Srgb,
                SpaceWire::DisplayP3 => Space::DisplayP3,
                SpaceWire::AdobeRgb => Space::AdobeRgb,
            }),
            tone: match self.tone {
                ToneWire::Linear => Tone::Linear,
                ToneWire::Display => Tone::Display,
                ToneWire::Stretch => Tone::Stretch,
            },
            quality: self.quality,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_from_the_screen_becomes_an_export() {
        let request: ExportRequest = serde_json::from_str(
            r#"{"source": "/a/r.tif", "target": "/b/r.png", "container": "png",
                "depth": "sixteen", "space": "display_p3", "tone": "stretch", "quality": 90}"#,
        )
        .unwrap();
        let export = request.export();
        assert_eq!(export.container, Container::Png);
        assert_eq!(export.depth, Depth::Sixteen);
        assert_eq!(export.space, Some(Space::DisplayP3));
        assert_eq!(export.tone, Tone::Stretch);
    }

    #[test]
    fn no_space_keeps_the_colors() {
        let request: ExportRequest = serde_json::from_str(
            r#"{"source": "a", "target": "b", "container": "fits", "depth": "float",
                "space": null, "tone": "linear", "quality": 90}"#,
        )
        .unwrap();
        assert_eq!(request.export().space, None);
    }
}
