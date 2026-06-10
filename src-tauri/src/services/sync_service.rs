use crate::db::models::{Profile, SyncProgress};
use crate::db::{profiles, SharedDb};
use crate::error::{AppError, AppResult};
use crate::parsers::m3u::M3uParser;
use crate::parsers::xtream::XtreamClient;
use crate::services::catalog_sync::{
    build_xtream_category_map, classify_m3u_entries_with_progress, map_xtream_live_streams,
    map_xtream_series, map_xtream_vod_streams, persist_catalog_with_options,
    reindex_catalog_search, CatalogSyncResult, PersistOptions,
};
use crate::services::profile_service;
use crate::services::sync_guard::SyncGuard;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Concurrent get_series_info requests during an Xtream sync.
const SERIES_INFO_CONCURRENCY: usize = 8;

pub async fn sync_profile(
    app: AppHandle,
    db: SharedDb,
    profile_id: String,
    guard: Option<Arc<SyncGuard>>,
) -> AppResult<()> {
    let acquired = match &guard {
        Some(g) => g.try_acquire(),
        None => true,
    };
    if !acquired {
        return Ok(());
    }

    let result = sync_profile_inner(app.clone(), db.clone(), profile_id.clone()).await;
    if let Some(g) = guard {
        g.release();
    }
    result
}

pub async fn sync_profile_manual(
    app: AppHandle,
    db: SharedDb,
    profile_id: String,
    guard: Arc<SyncGuard>,
) -> AppResult<()> {
    if !guard.try_acquire() {
        eprintln!("[sync] skipped profile={profile_id}: another sync in progress");
        return Err(AppError::msg(
            "Sincronização já em andamento. Aguarde a conclusão.",
        ));
    }

    eprintln!("[sync] start profile={profile_id}");
    let result = sync_profile_inner(app.clone(), db.clone(), profile_id.clone()).await;
    guard.release();
    match &result {
        Ok(()) => eprintln!("[sync] done profile={profile_id}"),
        Err(err) => {
            eprintln!("[sync] FAILED profile={profile_id}: {err}");
            let _ = app.emit("sync-error", err.to_string());
        }
    }
    result
}

async fn sync_profile_inner(app: AppHandle, db: SharedDb, profile_id: String) -> AppResult<()> {
    let profile = profile_service::get_profile(&db, &profile_id)?;

    emit_progress(&app, &profile_id, 5.0, "Preparando sincronização...")?;

    match profile.profile_type.as_str() {
        "m3u" => sync_m3u_profile(&app, &db, &profile_id, &profile).await,
        "xtream" => sync_xtream_profile(&app, &db, &profile_id, &profile).await,
        other => Err(AppError::msg(format!("Tipo de perfil não suportado: {other}"))),
    }
}

async fn sync_m3u_profile(
    app: &AppHandle,
    db: &SharedDb,
    profile_id: &str,
    profile: &Profile,
) -> AppResult<()> {
    let mut parser = M3uParser::new(profile_id.to_string());
    let entries = if let Some(url) = profile.url.as_deref() {
        emit_progress(app, profile_id, 15.0, "Baixando lista M3U...")?;
        parser.parse_url(url).await?
    } else if let Some(path) = profile.file_path.as_deref() {
        emit_progress(app, profile_id, 15.0, "Lendo arquivo M3U...")?;
        parser.parse_file(path).await?
    } else {
        return Err(AppError::msg("Perfil sem URL ou arquivo configurado."));
    };

    let total = entries.len();
    emit_progress(
        app,
        profile_id,
        30.0,
        format!("Classificando {total} entradas..."),
    )?;

    let catalog = classify_m3u_entries_with_progress(entries, |done, total| {
        if total == 0 {
            return;
        }
        let fraction = done as f64 / total as f64;
        let percent = 30.0 + fraction * 35.0;
        let _ = emit_progress(
            app,
            profile_id,
            percent,
            format!("Classificando {done}/{total} entradas..."),
        );
    });

    emit_progress(app, profile_id, 68.0, "Salvando catálogo...")?;
    persist_catalog_with_options(
        db,
        profile_id,
        &catalog,
        PersistOptions { reindex_fts: false },
        |percent, message| {
            let _ = emit_progress(app, profile_id, percent, message);
        },
    )?;

    let db_bg = db.clone();
    let profile_id_bg = profile_id.to_string();
    tauri::async_runtime::spawn(async move {
        let _ = reindex_catalog_search(&db_bg, &profile_id_bg);
    });

    db.with_conn(|conn| {
        profiles::update_last_sync(conn, profile_id, chrono::Utc::now().timestamp())
    })?;

    let summary = format!(
        "Sincronização concluída: {} canais, {} filmes, {} séries.",
        catalog.channels.len(),
        catalog.movies.len(),
        catalog.series.len()
    );
    emit_progress(app, profile_id, 100.0, summary)?;

    Ok(())
}

async fn sync_xtream_profile(
    app: &AppHandle,
    db: &SharedDb,
    profile_id: &str,
    profile: &Profile,
) -> AppResult<()> {
    let url = profile
        .url
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| AppError::msg("Perfil Xtream sem URL configurada."))?;
    let username = profile
        .username
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| AppError::msg("Perfil Xtream sem usuário configurado."))?;
    let password = profile
        .password
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| AppError::msg("Perfil Xtream sem senha configurada."))?;

    let client = XtreamClient::new(
        url.trim().to_string(),
        username.trim().to_string(),
        password.trim().to_string(),
    );

    emit_progress(app, profile_id, 5.0, "Buscando categorias ao vivo...")?;
    let live_categories = client.get_live_categories().await.unwrap_or_default();
    let live_category_map = build_xtream_category_map(&live_categories);

    emit_progress(app, profile_id, 8.0, "Buscando canais ao vivo...")?;
    let live_streams = client.get_live_streams().await?;
    let channels = map_xtream_live_streams(profile_id, &client, live_streams, &live_category_map);
    emit_progress(
        app,
        profile_id,
        10.0,
        format!("{} canais ao vivo encontrados.", channels.len()),
    )?;

    emit_progress(app, profile_id, 12.0, "Buscando categorias de filmes...")?;
    let vod_categories = client.get_vod_categories().await.unwrap_or_default();
    let vod_category_map = build_xtream_category_map(&vod_categories);

    emit_progress(app, profile_id, 15.0, "Buscando filmes...")?;
    let vod_streams = client.get_vod_streams().await?;
    let movies = map_xtream_vod_streams(profile_id, &client, vod_streams, &vod_category_map);
    emit_progress(
        app,
        profile_id,
        40.0,
        format!("{} filmes encontrados.", movies.len()),
    )?;

    emit_progress(app, profile_id, 42.0, "Buscando categorias de séries...")?;
    let series_categories = client.get_series_categories().await.unwrap_or_default();
    let series_category_map = build_xtream_category_map(&series_categories);

    emit_progress(app, profile_id, 45.0, "Buscando séries...")?;
    let series_list = client.get_series().await?;
    let total_series = series_list.len().max(1);
    let mut series_entries = Vec::new();
    let mut episodes = Vec::new();

    // One request per series dominates the whole sync when done serially;
    // fetch them in concurrent blocks while keeping the provider's order.
    let mut done = 0_usize;
    for chunk in series_list.chunks(SERIES_INFO_CONCURRENCY) {
        let results = futures_util::future::join_all(chunk.iter().map(|series_item| {
            let client = &client;
            async move {
                let info = client.get_series_info(series_item.series_id).await;
                (series_item, info)
            }
        }))
        .await;

        for (series_item, info) in results {
            done += 1;
            let info = match info {
                Ok(info) => info,
                Err(err) => {
                    // One flaky request must not abort a sync of thousands of
                    // series; episodes for this one sync on demand when opened.
                    eprintln!(
                        "[sync] series_info failed for {} ({}): {err}",
                        series_item.name, series_item.series_id
                    );
                    Default::default()
                }
            };
            let (series_entry, series_episodes) = map_xtream_series(
                profile_id,
                series_item,
                &info,
                &client,
                &series_category_map,
            );
            series_entries.push(series_entry);
            episodes.extend(series_episodes);
        }

        let percent = 70.0 + (25.0 * done as f64 / total_series as f64);
        emit_progress(
            app,
            profile_id,
            percent,
            format!("Processando séries ({}/{})...", done, series_list.len()),
        )?;
    }

    emit_progress(
        app,
        profile_id,
        95.0,
        format!(
            "{} séries e {} episódios encontrados.",
            series_entries.len(),
            episodes.len()
        ),
    )?;

    let catalog = CatalogSyncResult {
        channels,
        movies,
        series: series_entries,
        episodes,
    };

    persist_catalog_with_options(
        db,
        profile_id,
        &catalog,
        PersistOptions { reindex_fts: false },
        |percent, message| {
            let _ = emit_progress(app, profile_id, percent, message);
        },
    )?;

    // Search reindex happens in the background, same as the M3U path, so the
    // sync finishes (and the UI unblocks) as soon as the catalog is saved.
    let db_bg = db.clone();
    let profile_id_bg = profile_id.to_string();
    tauri::async_runtime::spawn(async move {
        let _ = reindex_catalog_search(&db_bg, &profile_id_bg);
    });

    db.with_conn(|conn| {
        profiles::update_last_sync(conn, profile_id, chrono::Utc::now().timestamp())
    })?;

    let summary = format!(
        "Sincronização concluída: {} canais, {} filmes, {} séries.",
        catalog.channels.len(),
        catalog.movies.len(),
        catalog.series.len()
    );
    emit_progress(app, profile_id, 100.0, summary)?;

    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    profile_id: &str,
    percent: f64,
    message: impl Into<String>,
) -> AppResult<()> {
    app.emit(
        "sync-progress",
        SyncProgress {
            percent,
            message: message.into(),
            profile_id: profile_id.to_string(),
        },
    )
    .map_err(|e| AppError::msg(e.to_string()))
}
