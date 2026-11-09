#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod app_menu;
mod commands;
mod jobs;
mod launch_args;
mod mask_store;
mod payload;
mod sky_cache;
mod update;

use commands::{
    app_cmd, device_cmd, export_cmd, files_cmd, jobs_cmd, launch_cmd, mask_cmd, menu_cmd, sky_cmd,
    update_cmd,
};
use launch_args::LaunchContext;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

/// Sent to the screen when the program is asked to close while a job is running.
const CLOSE_REQUESTED_EVENT: &str = "close-requested";

fn job_is_running(app: &tauri::AppHandle) -> bool {
    app.try_state::<jobs::JobRegistry>()
        .is_some_and(|registry| registry.is_running())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(LaunchContext::from_args(std::env::args().skip(1)))
        .manage(jobs::JobRegistry::default())
        .manage(mask_store::MaskStore::default())
        .manage(sky_cache::SkyCache::default())
        .setup(|app| {
            app_menu::install(app.handle())?;
            Ok(())
        })
        .on_menu_event(app_menu::on_event)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && job_is_running(window.app_handle())
            {
                api.prevent_close();
                let _ = window.app_handle().emit(CLOSE_REQUESTED_EVENT, ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            files_cmd::probe_files,
            files_cmd::expand_paths,
            files_cmd::open_image,
            launch_cmd::launch_context,
            device_cmd::gpu_info,
            sky_cmd::detect_sky,
            sky_cmd::shape_sky,
            sky_cmd::sky_star_count,
            mask_cmd::set_mask,
            mask_cmd::clear_mask,
            export_cmd::save_result_as,
            app_cmd::quit_app,
            update_cmd::check_update,
            menu_cmd::set_menu_text,
            jobs_cmd::start_job,
            jobs_cmd::cancel_job,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build STARY")
        .run(|app, event| {
            if let RunEvent::ExitRequested {
                api, code: None, ..
            } = event
                && job_is_running(app)
            {
                api.prevent_exit();
                let _ = app.emit(CLOSE_REQUESTED_EVENT, ());
            }
        });
}
