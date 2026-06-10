mod commands;
mod db;
mod error;
mod parsers;
mod services;

use db::init_db;
use services::background_sync::start_background_sync;
use services::sync_guard::SyncGuard;
use std::sync::{Arc, OnceLock};
use tauri::{Manager, RunEvent};

static BACKGROUND_SYNC_STARTED: OnceLock<()> = OnceLock::new();

fn apply_window_icon(handle: &tauri::AppHandle) {
    let Some(icon) = handle.default_window_icon() else {
        return;
    };
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.set_icon(icon.clone());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_libmpv::init())
        .setup(|app| {
            init_db(app.handle())?;
            let guard = Arc::new(SyncGuard::new());
            app.manage(guard);
            apply_window_icon(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::get_active_profile_id,
            commands::set_active_profile_id,
            commands::add_m3u_profile,
            commands::add_xtream_profile,
            commands::remove_profile,
            commands::sync_profile,
            commands::sync_profile_blocking,
            commands::list_channels,
            commands::get_channel,
            commands::list_groups,
            commands::pick_m3u_file,
            commands::list_movies,
            commands::list_movie_categories,
            commands::get_movie,
            commands::list_series,
            commands::list_series_categories,
            commands::get_series_detail,
            commands::list_recent,
            commands::list_recent_movies,
            commands::list_recent_series,
            commands::list_releases,
            commands::list_releases_movies,
            commands::list_releases_series,
            commands::list_featured_movies,
            commands::get_home_catalog,
            commands::save_history,
            commands::list_continue_watching,
            commands::list_recent_channels,
            commands::add_favorite,
            commands::remove_favorite,
            commands::list_favorites,
            commands::is_favorite,
            commands::search_catalog,
            commands::get_channel_epg,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::verify_parental_pin,
            commands::is_adult_category,
            commands::get_device_fingerprint,
            commands::get_license_state,
            commands::activate_license,
            commands::validate_license,
            commands::clear_license,
            commands::update_m3u_profile,
            commands::update_xtream_profile,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::Ready = event {
                apply_window_icon(app_handle);
                BACKGROUND_SYNC_STARTED.get_or_init(|| {
                    let Some(guard) = app_handle.try_state::<Arc<SyncGuard>>() else {
                        eprintln!("[background_sync] SyncGuard not ready; skipping");
                        return;
                    };
                    let Some(db) = app_handle.try_state::<db::SharedDb>() else {
                        eprintln!("[background_sync] database not ready; skipping");
                        return;
                    };
                    start_background_sync(
                        app_handle.clone(),
                        db.inner().clone(),
                        guard.inner().clone(),
                    );
                });
            }
        });
}
