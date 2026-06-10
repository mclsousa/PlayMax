use crate::db::settings::{self, KEY_AUTO_SYNC, KEY_LAST_BACKGROUND_SYNC};
use crate::db::SharedDb;
use crate::services::profile_service;
use crate::services::sync_guard::SyncGuard;
use crate::services::sync_service;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;

const SYNC_INTERVAL: Duration = Duration::from_secs(30 * 60);

pub fn start_background_sync(app: AppHandle, db: SharedDb, guard: Arc<SyncGuard>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(SYNC_INTERVAL).await;
            if let Err(err) = run_background_sync(&app, &db, guard.clone()).await {
                eprintln!("[background_sync] {err}");
            }
        }
    });
}

async fn run_background_sync(
    app: &AppHandle,
    db: &SharedDb,
    guard: Arc<SyncGuard>,
) -> crate::error::AppResult<()> {
    let auto_sync = db.with_conn(|conn| settings::get_bool(conn, KEY_AUTO_SYNC, false))?;
    if !auto_sync {
        return Ok(());
    }

    if guard.is_in_progress() {
        return Ok(());
    }

    let profile_id = profile_service::resolve_active_profile_id(db)?;

    let Some(profile_id) = profile_id else {
        return Ok(());
    };

    let result = sync_service::sync_profile(
        app.clone(),
        db.clone(),
        profile_id.clone(),
        Some(guard.clone()),
    )
    .await;

    if result.is_ok() {
        let now = chrono::Utc::now().timestamp();
        db.with_conn(|conn| settings::set_i64(conn, KEY_LAST_BACKGROUND_SYNC, now))?;
    }

    result
}
