use stary_mask::shaped;
use stary_register::{DetectParams, find_stars};
use stary_sky::Stroke;
use tauri::{
    State,
    ipc::{InvokeBody, Request, Response},
};

use crate::{
    mask_store::MaskStore,
    payload::{
        binary_packet::pack,
        command_error::CommandError,
        sky_payload::{SkyHeader, StrokeWire},
    },
    sky_cache::SkyCache,
};

const SKY_BYTE: u8 = 255;
const CONTOUR_TOLERANCE: f32 = 0.8;
const FULL_SCALE: f32 = u16::MAX as f32;
const BYTE_SCALE: f32 = FULL_SCALE / 255.0;

fn number<T: std::str::FromStr>(request: &Request<'_>, name: &str) -> Result<T, CommandError> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| CommandError {
            code: "bad_request".into(),
            message: format!("the request has no usable {name}"),
        })
}

fn bytes_of(request: &Request<'_>) -> Result<Vec<u8>, CommandError> {
    match request.body() {
        InvokeBody::Raw(bytes) => Ok(bytes.clone()),
        InvokeBody::Json(_) => Err(CommandError {
            code: "bad_request".into(),
            message: "the request has no bytes".into(),
        }),
    }
}

/// Finds the sky in the preview of a photograph, given what the user painted so far.
#[tauri::command]
pub async fn detect_sky(
    path: String,
    max_edge: u32,
    strokes: Vec<StrokeWire>,
    cache: State<'_, SkyCache>,
) -> Result<Response, CommandError> {
    let cache = cache.inner().clone();
    let strokes: Vec<Stroke> = strokes.into_iter().map(Stroke::from).collect();
    let packet = tauri::async_runtime::spawn_blocking(move || {
        let sky = cache.analysis(&path, max_edge)?;
        let detection = sky.detect(&strokes);
        let header = SkyHeader {
            width: detection.width,
            height: detection.height,
            sky_fraction: detection.sky_fraction(),
            contours: detection.contours(CONTOUR_TOLERANCE),
        };
        let region: Vec<u8> = detection
            .sky
            .iter()
            .map(|&sky| if sky { SKY_BYTE } else { 0 })
            .collect();
        pack(&header, &region)
    })
    .await
    .map_err(CommandError::internal)??;
    Ok(Response::new(packet))
}

/// Turns a region of sky (bytes, 255 for sky) into the mask that follows its border moved by the
/// offset and falling from sky to ground over the feather.
#[tauri::command]
pub async fn shape_sky(request: Request<'_>) -> Result<Response, CommandError> {
    let (width, height): (usize, usize) =
        (number(&request, "x-width")?, number(&request, "x-height")?);
    let (offset, feather): (f32, f32) = (
        number(&request, "x-offset")?,
        number(&request, "x-feather")?,
    );
    let region = bytes_of(&request)?;
    let mask = tauri::async_runtime::spawn_blocking(move || {
        let sky: Vec<bool> = region.iter().map(|&byte| byte > SKY_BYTE / 2).collect();
        shaped(width, height, &sky, offset, feather)
            .map(|mask| {
                mask.values()
                    .iter()
                    .map(|value| (value * 255.0).round() as u8)
                    .collect::<Vec<u8>>()
            })
            .map_err(CommandError::internal)
    })
    .await
    .map_err(CommandError::internal)??;
    Ok(Response::new(mask))
}

/// How many stars of the photograph are in the sky of the mask of a session: too few and the
/// frames cannot be aligned on the sky.
#[tauri::command]
pub async fn sky_star_count(
    session: String,
    path: String,
    max_edge: u32,
    masks: State<'_, MaskStore>,
) -> Result<usize, CommandError> {
    let mask = masks.get(&session).ok_or_else(|| CommandError {
        code: "no_mask".into(),
        message: "the session has no mask".into(),
    })?;
    tauri::async_runtime::spawn_blocking(move || {
        let preview = stary_io::read_preview(std::path::Path::new(&path), max_edge)?;
        let (width, height) = (preview.width as usize, preview.height as usize);
        let light: Vec<f32> = preview
            .rgb8
            .as_chunks::<3>()
            .0
            .iter()
            .map(|pixel| {
                (0.2126 * f32::from(pixel[0])
                    + 0.7152 * f32::from(pixel[1])
                    + 0.0722 * f32::from(pixel[2]))
                    * BYTE_SCALE
            })
            .collect();
        let stars = find_stars(width, height, light, &DetectParams::default());
        Ok(stars
            .iter()
            .filter(|star| mask.is_sky_at(star.x, star.y, width, height))
            .count())
    })
    .await
    .map_err(CommandError::internal)?
}
