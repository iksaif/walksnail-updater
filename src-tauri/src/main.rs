// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache;
mod commands;
mod events;
mod state;

use anyhow::Result;
use tauri::Emitter;
use tokio::sync::mpsc;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("info,walksnail_updater=debug")
            }),
        )
        .init();

    if let Err(err) = run() {
        eprintln!("fatal: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::fetch_index,
            commands::list_releases,
            commands::latest_for,
            commands::scan_sd,
            commands::download_firmware,
            commands::stage_firmware,
            commands::reveal_in_file_manager,
            commands::open_url,
            commands::load_instructions,
            commands::get_app_paths,
            commands::mark_onboarded,
            commands::is_onboarded,
            commands::get_download_dir,
            commands::set_download_dir,
            commands::get_download_dir_pref,
            commands::list_cached_firmware,
        ])
        .setup(|app| {
            let (tx, mut rx) = mpsc::channel(32);
            tauri::async_runtime::spawn(sdcard::run_watcher(tx));
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let channel = match &event {
                        sdcard::SdEvent::Mounted { .. } => events::SD_MOUNTED,
                        sdcard::SdEvent::Removed { .. } => events::SD_REMOVED,
                    };
                    let _ = app_handle.emit(channel, &event);
                    info!(?event, "emitted sd event");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(Into::into)
}
