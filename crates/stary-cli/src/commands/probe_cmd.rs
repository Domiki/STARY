use std::path::{Path, PathBuf};

use stary_core::{ProbeInfo, SourceFormat};

fn describe(info: &ProbeInfo) -> String {
    let details = &info.details;
    format!(
        "{:<28} {:<5} {:>5}x{:<5} {:<24} {:>6} {:>6} {}",
        info.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?"),
        format_label(info.format),
        details.width,
        details.height,
        details.camera.as_deref().unwrap_or("-"),
        details
            .exposure_s
            .map_or("-".into(), |seconds| format!("{seconds}s")),
        details.iso.map_or("-".into(), |iso| iso.to_string()),
        details.captured_at.as_deref().unwrap_or("-"),
    )
}

fn format_label(format: SourceFormat) -> &'static str {
    match format {
        SourceFormat::Raw => "raw",
        SourceFormat::Fits => "fits",
        SourceFormat::Jpeg => "jpeg",
        SourceFormat::Png => "png",
        SourceFormat::Tiff => "tiff",
    }
}

fn probe_one(path: &Path) -> Result<ProbeInfo, String> {
    stary_io::probe_file(path).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn run(paths: &[PathBuf], json: bool) -> anyhow::Result<()> {
    let (infos, errors): (Vec<_>, Vec<_>) = paths
        .iter()
        .map(|path| probe_one(path))
        .partition(Result::is_ok);
    let infos: Vec<ProbeInfo> = infos.into_iter().flatten().collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&infos)?);
    } else {
        infos.iter().for_each(|info| println!("{}", describe(info)));
    }
    errors
        .into_iter()
        .filter_map(Result::err)
        .for_each(|message| eprintln!("error: {message}"));
    Ok(())
}
