use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use stary_sky::{Picture, Sky};

use crate::payload::command_error::CommandError;

struct Cached {
    path: String,
    max_edge: u32,
    sky: Arc<Sky>,
}

/// What was worked out about the last photograph the sky was looked for in, so that painting on it
/// does not start from the file again.
#[derive(Clone, Default)]
pub struct SkyCache {
    last: Arc<Mutex<Option<Cached>>>,
}

impl SkyCache {
    pub fn analysis(&self, path: &str, max_edge: u32) -> Result<Arc<Sky>, CommandError> {
        if let Some(cached) = self.last.lock().ok().and_then(|last| {
            last.as_ref()
                .filter(|cached| cached.path == path && cached.max_edge == max_edge)
                .map(|cached| Arc::clone(&cached.sky))
        }) {
            return Ok(cached);
        }
        let preview = stary_io::read_preview(Path::new(path), max_edge)?;
        let sky = Arc::new(
            Sky::analyze(&Picture {
                width: preview.width as usize,
                height: preview.height as usize,
                rgb: &preview.rgb8,
            })
            .map_err(CommandError::internal)?,
        );
        if let Ok(mut last) = self.last.lock() {
            *last = Some(Cached {
                path: path.to_string(),
                max_edge,
                sky: Arc::clone(&sky),
            });
        }
        Ok(sky)
    }
}
