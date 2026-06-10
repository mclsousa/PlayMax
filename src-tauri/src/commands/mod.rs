use crate::db::models::{
    AppSettings, CatalogItem, CategoryCount, Channel, ChannelEpg, ChannelsPage, EnrichedMovie,
    Favorite, FavoriteItem, HistoryEntry, HomeCatalog, MoviesPage, Profile, RecommendedItem,
    SearchResult, SeriesDetail, SeriesPage,
};
use crate::db::{channels, search, settings::{self, KEY_AUTO_SYNC, KEY_LAST_BACKGROUND_SYNC, KEY_PARENTAL_ENABLED, KEY_PARENTAL_PIN_HASH}};
use crate::db::SharedDb;
use crate::error::{AppError, AppResult};
use crate::services::sync_guard::SyncGuard;
use crate::services::{epg_service, license_service, parental_service, profile_service, series_sync, sync_service, tmdb_service};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command(async)]
pub fn list_profiles(state: State<'_, SharedDb>) -> AppResult<Vec<Profile>> {
    profile_service::list_profiles(&state)
}

#[tauri::command(async)]
pub fn get_active_profile_id(state: State<'_, SharedDb>) -> AppResult<Option<String>> {
    profile_service::resolve_active_profile_id(&state)
}

#[tauri::command(async)]
pub fn set_active_profile_id(
    state: State<'_, SharedDb>,
    profile_id: String,
) -> AppResult<()> {
    profile_service::set_active_profile_id(&state, &profile_id)
}

#[tauri::command(async)]
pub fn add_m3u_profile(
    state: State<'_, SharedDb>,
    name: String,
    url: Option<String>,
    file_path: Option<String>,
) -> AppResult<Profile> {
    profile_service::add_m3u_profile(&state, name, url, file_path)
}

#[tauri::command(async)]
pub fn remove_profile(state: State<'_, SharedDb>, profile_id: String) -> AppResult<()> {
    profile_service::remove_profile(&state, &profile_id)
}

#[tauri::command(async)]
pub fn sync_profile(
    app: AppHandle,
    state: State<'_, SharedDb>,
    guard: State<'_, Arc<SyncGuard>>,
    profile_id: String,
) -> AppResult<()> {
    let db = state.inner().clone();
    let app_clone = app.clone();
    let profile_id_clone = profile_id.clone();
    let guard = guard.inner().clone();
    tauri::async_runtime::spawn(async move {
        let _ = sync_service::sync_profile_manual(app_clone, db, profile_id_clone, guard).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn sync_profile_blocking(
    app: AppHandle,
    state: State<'_, SharedDb>,
    guard: State<'_, Arc<SyncGuard>>,
    profile_id: String,
) -> AppResult<()> {
    sync_service::sync_profile_manual(
        app,
        state.inner().clone(),
        profile_id,
        guard.inner().clone(),
    )
    .await
}

#[tauri::command(async)]
pub fn list_channels(
    state: State<'_, SharedDb>,
    profile_id: String,
    group: Option<String>,
    search: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> AppResult<ChannelsPage> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(100).clamp(1, 500);
    state.with_read(|conn| {
        crate::db::channels::list_channels(
            conn,
            &profile_id,
            group.as_deref(),
            search.as_deref(),
            offset,
            limit,
        )
    })
}

#[tauri::command(async)]
pub fn list_groups(
    state: State<'_, SharedDb>,
    profile_id: String,
    content_kind: Option<String>,
) -> AppResult<Vec<CategoryCount>> {
    state.with_read(|conn| {
        match content_kind.as_deref() {
            Some("live") => crate::db::channels::list_live_groups(conn, &profile_id),
            _ => crate::db::channels::list_groups(conn, &profile_id),
        }
    })
}

#[tauri::command(async)]
pub fn get_channel(
    state: State<'_, SharedDb>,
    channel_id: String,
) -> AppResult<Option<Channel>> {
    state.with_read(|conn| channels::get_channel(conn, &channel_id))
}

#[tauri::command]
pub async fn pick_m3u_file(app: AppHandle) -> AppResult<Option<String>> {
    let path = app
        .dialog()
        .file()
        .add_filter("Playlist M3U", &["m3u", "m3u8"])
        .blocking_pick_file();
    Ok(path.map(|p| p.to_string()))
}

#[tauri::command(async)]
pub fn add_xtream_profile(
    state: State<'_, SharedDb>,
    name: String,
    url: String,
    username: String,
    password: String,
) -> AppResult<Profile> {
    profile_service::add_xtream_profile(&state, name, url, username, password)
}

#[tauri::command(async)]
pub fn list_movies(
    state: State<'_, SharedDb>,
    profile_id: String,
    category: Option<String>,
    search: Option<String>,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> AppResult<MoviesPage> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(100).clamp(1, 500);
    state.with_read(|conn| {
        crate::db::movies::list_movies(
            conn,
            &profile_id,
            category.as_deref(),
            search.as_deref(),
            sort.as_deref(),
            offset,
            limit,
        )
    })
}

#[tauri::command(async)]
pub fn list_movie_categories(
    state: State<'_, SharedDb>,
    profile_id: String,
) -> AppResult<Vec<CategoryCount>> {
    state.with_read(|conn| crate::db::movies::list_movie_categories(conn, &profile_id))
}

fn extract_year_from_title(title: &str) -> Option<i32> {
    tmdb_service::extract_year_from_title(title)
}

fn recommendation_search_queries(title: &str) -> Vec<String> {
    let trimmed = title.trim();
    let mut queries = Vec::new();
    if trimmed.is_empty() {
        return queries;
    }

    queries.push(trimmed.to_string());
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() > 4 {
        queries.push(words[..4].join(" "));
    }
    if words.len() > 2 {
        queries.push(words[..2].join(" "));
    }

    queries.sort_by_key(|query| std::cmp::Reverse(query.len()));
    queries.dedup();
    queries
}

fn match_recommendations(
    conn: &rusqlite::Connection,
    profile_id: &str,
    item_type: &str,
    exclude_id: &str,
    candidates: &[tmdb_service::SimilarCandidate],
    limit: usize,
) -> AppResult<Vec<RecommendedItem>> {
    let mut matched = Vec::new();
    let mut seen = HashSet::new();
    seen.insert(exclude_id.to_string());

    for candidate in candidates {
        if matched.len() >= limit {
            break;
        }

        let mut best: Option<(RecommendedItem, f64)> = None;

        for query in recommendation_search_queries(&candidate.title) {
            let results = search::search_catalog(conn, profile_id, &query, 15)?;
            for result in results {
                if result.item_type != item_type {
                    continue;
                }
                if seen.contains(&result.id) {
                    continue;
                }

                let score = tmdb_service::title_similarity(&candidate.title, &result.name);
                if score < 0.40 {
                    continue;
                }

                if let (Some(expected), Some(found)) = (
                    candidate.year,
                    extract_year_from_title(&result.name),
                ) {
                    if (expected - found).abs() > 2 {
                        continue;
                    }
                }

                if best.as_ref().map(|(_, current)| score > *current).unwrap_or(true) {
                    best = Some((
                        RecommendedItem {
                            id: result.id,
                            name: result.name,
                            poster: result.poster,
                            backdrop: None,
                        },
                        score,
                    ));
                }
            }
        }

        if let Some((item, _)) = best {
            seen.insert(item.id.clone());
            matched.push(item);
        }
    }

    Ok(matched)
}

fn build_similar_items(
    conn: &rusqlite::Connection,
    profile_id: &str,
    item_type: &str,
    exclude_id: &str,
    candidates: &[tmdb_service::SimilarCandidate],
    category: Option<&str>,
    genres: Option<&str>,
    limit: usize,
) -> AppResult<Vec<RecommendedItem>> {
    let mut items = match_recommendations(conn, profile_id, item_type, exclude_id, candidates, limit)?;
    if items.len() >= limit {
        return Ok(items);
    }

    let remaining = limit.saturating_sub(items.len());
    let fallback = if item_type == "movie" {
        crate::db::movies::list_related_movies(
            conn,
            profile_id,
            exclude_id,
            category,
            genres,
            remaining as i64,
        )?
    } else {
        crate::db::series::list_related_series(
            conn,
            profile_id,
            exclude_id,
            category,
            genres,
            remaining as i64,
        )?
    };

    let mut seen: HashSet<String> = items.iter().map(|item| item.id.clone()).collect();
    seen.insert(exclude_id.to_string());

    for item in fallback {
        if seen.insert(item.id.clone()) {
            items.push(item);
            if items.len() >= limit {
                break;
            }
        }
    }

    Ok(items)
}

#[tauri::command]
pub async fn get_movie(
    state: State<'_, SharedDb>,
    id: String,
    include_similar: Option<bool>,
    enrich: Option<bool>,
) -> AppResult<EnrichedMovie> {
    let include_similar = include_similar.unwrap_or(true);
    let enrich = enrich.unwrap_or(true);
    let mut movie = state.with_read(|conn| {
        crate::db::movies::get_movie(conn, &id)?
            .ok_or_else(|| AppError::msg("Filme não encontrado."))
    })?;

    let enrichment = if enrich {
        tmdb_service::enrich_movie(&mut movie, tmdb_service::api_key())
            .await
            .ok()
            .unwrap_or(tmdb_service::DetailEnrichment {
                cast: None,
                tmdb_id: None,
            })
    } else {
        tmdb_service::DetailEnrichment {
            cast: None,
            tmdb_id: None,
        }
    };

    let similar = if include_similar {
        let candidates = if enrich {
            if let Some(tmdb_id) = enrichment.tmdb_id {
                tmdb_service::fetch_similar_movie_candidates(tmdb_id, tmdb_service::api_key()).await
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let profile_id = movie.profile_id.clone();
        let movie_id = movie.id.clone();
        let category = movie.category.clone();
        let genres = movie.genres.clone();
        state
            .with_read(|conn| {
                build_similar_items(
                    conn,
                    &profile_id,
                    "movie",
                    &movie_id,
                    &candidates,
                    category.as_deref(),
                    genres.as_deref(),
                    12,
                )
            })
            .ok()
            .filter(|items| !items.is_empty())
    } else {
        None
    };

    Ok(EnrichedMovie {
        movie,
        cast: enrichment.cast,
        similar,
    })
}

#[tauri::command(async)]
pub fn list_series(
    state: State<'_, SharedDb>,
    profile_id: String,
    category: Option<String>,
    search: Option<String>,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> AppResult<SeriesPage> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(100).clamp(1, 500);
    state.with_read(|conn| {
        crate::db::series::list_series(
            conn,
            &profile_id,
            category.as_deref(),
            search.as_deref(),
            sort.as_deref(),
            offset,
            limit,
        )
    })
}

#[tauri::command(async)]
pub fn list_series_categories(
    state: State<'_, SharedDb>,
    profile_id: String,
) -> AppResult<Vec<CategoryCount>> {
    state.with_read(|conn| crate::db::series::list_series_categories(conn, &profile_id))
}

#[tauri::command]
pub async fn get_series_detail(
    state: State<'_, SharedDb>,
    id: String,
    enrich: Option<bool>,
    include_similar: Option<bool>,
    sync_episodes: Option<bool>,
) -> AppResult<SeriesDetail> {
    let enrich = enrich.unwrap_or(true);
    let include_similar = include_similar.unwrap_or(true);
    let sync_episodes = sync_episodes.unwrap_or(true);
    let db = state.inner().clone();
    if sync_episodes {
        let needs_sync = state.with_read(|conn| {
            Ok(crate::db::series::list_episodes_for_series(conn, &id)?.is_empty())
        })?;
        if needs_sync {
            let _ = series_sync::sync_episodes_on_demand(&db, &id).await;
        }
    }
    let mut detail = state.with_read(|conn| {
        crate::db::series::get_series_detail(conn, &id)?
            .ok_or_else(|| AppError::msg("Série não encontrada."))
    })?;

    let enrichment = if enrich {
        tmdb_service::enrich_series(&mut detail.series, tmdb_service::api_key())
            .await
            .ok()
            .unwrap_or(tmdb_service::DetailEnrichment {
                cast: None,
                tmdb_id: None,
            })
    } else {
        tmdb_service::DetailEnrichment {
            cast: None,
            tmdb_id: None,
        }
    };

    detail.cast = enrichment.cast;

    detail.similar = if enrich && include_similar {
        let candidates = if let Some(tmdb_id) = enrichment.tmdb_id {
            tmdb_service::fetch_similar_series_candidates(tmdb_id, tmdb_service::api_key()).await
        } else {
            Vec::new()
        };

        let profile_id = detail.series.profile_id.clone();
        let series_id = detail.series.id.clone();
        let category = detail.series.category.clone();
        let genres = detail.series.genres.clone();
        state
            .with_read(|conn| {
                build_similar_items(
                    conn,
                    &profile_id,
                    "series",
                    &series_id,
                    &candidates,
                    category.as_deref(),
                    genres.as_deref(),
                    12,
                )
            })
            .ok()
            .filter(|items| !items.is_empty())
    } else {
        None
    };

    Ok(detail)
}

#[tauri::command(async)]
pub fn list_recent(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    state.with_read(|conn| crate::db::movies::list_recent(conn, &profile_id, limit))
}

#[tauri::command(async)]
pub fn list_recent_movies(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    state.with_read(|conn| {
        crate::db::movies::list_recent_movies(
            conn,
            &profile_id,
            limit,
            recent_movies_since_ts(),
            releases_since_ts(),
        )
    })
}

#[tauri::command(async)]
pub fn list_recent_series(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let min_year = release_year_threshold();
    let since_ts = recent_movies_since_ts();
    state.with_read(|conn| {
        crate::db::movies::list_recent_series(conn, &profile_id, limit, min_year, since_ts)
    })
}

fn now_unix_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn releases_since_ts() -> i64 {
    now_unix_ts() - 30 * 24 * 60 * 60
}

/// Minimum release year that counts as a "Lançamento" for series (current year
/// and the previous one), since series have no Lançamentos folder to key off.
fn release_year_threshold() -> i32 {
    use chrono::Datelike;
    chrono::Utc::now().year() - 1
}

fn recent_movies_since_ts() -> i64 {
    now_unix_ts() - 60 * 24 * 60 * 60
}

fn updated_series_since_ts() -> i64 {
    now_unix_ts() - 60 * 24 * 60 * 60
}

#[tauri::command(async)]
pub fn list_releases(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let since_ts = releases_since_ts();
    state.with_read(|conn| crate::db::movies::list_releases(conn, &profile_id, limit, since_ts))
}

#[tauri::command(async)]
pub fn list_releases_movies(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let since_ts = releases_since_ts();
    state.with_read(|conn| {
        crate::db::movies::list_releases_movies(conn, &profile_id, limit, since_ts)
    })
}

#[tauri::command(async)]
pub fn list_releases_series(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let min_year = release_year_threshold();
    state.with_read(|conn| {
        crate::db::movies::list_release_series(conn, &profile_id, limit, min_year)
    })
}

#[tauri::command(async)]
pub fn list_featured_movies(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<CatalogItem>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    state.with_read(|conn| {
        crate::db::movies::list_featured_movies(conn, &profile_id, limit)
    })
}

#[tauri::command(async)]
pub fn get_home_catalog(
    state: State<'_, SharedDb>,
    profile_id: String,
    release_movies_limit: Option<i64>,
    catalog_limit: Option<i64>,
) -> AppResult<HomeCatalog> {
    let release_limit = release_movies_limit.unwrap_or(40).clamp(1, 200);
    let section_limit = catalog_limit.unwrap_or(20).clamp(1, 200);
    let since_ts = releases_since_ts();
    let recent_since = recent_movies_since_ts();
    let updated_since = updated_series_since_ts();
    let min_year = release_year_threshold();

    // Backfilling tracking data needs the write connection; only take it when
    // the profile actually lacks tracking rows, so opening the Home tab never
    // waits behind a running sync transaction.
    let needs_tracking: bool = state.with_read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM series_episode_tracking WHERE profile_id = ?1",
            [&profile_id],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    })?;
    if needs_tracking {
        state.with_conn(|conn| {
            crate::db::series_tracking::ensure_profile_tracking(conn, &profile_id)
        })?;
    }

    state.with_read(|conn| {
        let release_movies = crate::db::movies::list_releases_movies(
            conn,
            &profile_id,
            release_limit,
            since_ts,
        )?;
        let release_ids: HashSet<String> =
            release_movies.iter().map(|item| item.id.clone()).collect();
        Ok(HomeCatalog {
            recent_movies: crate::db::movies::list_recent_movies_excluding(
                conn,
                &profile_id,
                section_limit,
                recent_since,
                &release_ids,
            )?,
            release_movies,
            recent_series: crate::db::movies::list_recent_series(
                conn,
                &profile_id,
                section_limit,
                min_year,
                recent_since,
            )?,
            updated_series: crate::db::movies::list_updated_series(
                conn,
                &profile_id,
                section_limit,
                updated_since,
                min_year,
            )?,
            release_series: crate::db::movies::list_release_series(
                conn,
                &profile_id,
                section_limit,
                min_year,
            )?,
            featured_movies: crate::db::movies::list_featured_movies(
                conn,
                &profile_id,
                section_limit,
            )?,
        })
    })
}

#[tauri::command(async)]
pub fn save_history(
    state: State<'_, SharedDb>,
    profile_id: String,
    item_type: String,
    item_id: String,
    name: String,
    poster: Option<String>,
    stream_url: String,
    position: f64,
    duration: f64,
) -> AppResult<()> {
    if item_type != "movie" && item_type != "episode" && item_type != "live" {
        return Err(AppError::msg("Tipo de item inválido."));
    }
    let watched_at = now_unix_ts();
    state.with_conn(|conn| {
        if item_type == "live" {
            crate::db::history::upsert_live(
                conn,
                &profile_id,
                &item_id,
                &name,
                poster.as_deref(),
                &stream_url,
                watched_at,
            )
        } else {
            crate::db::history::upsert(
                conn,
                &profile_id,
                &item_type,
                &item_id,
                &name,
                poster.as_deref(),
                &stream_url,
                position.max(0.0),
                duration.max(0.0),
                watched_at,
            )
        }
    })
}

#[tauri::command(async)]
pub fn list_recent_channels(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<HistoryEntry>> {
    let limit = limit.unwrap_or(10).clamp(1, 20);
    state.with_read(|conn| crate::db::history::list_recent_channels(conn, &profile_id, limit))
}

#[tauri::command(async)]
pub fn list_continue_watching(
    state: State<'_, SharedDb>,
    profile_id: String,
    limit: Option<i64>,
) -> AppResult<Vec<HistoryEntry>> {
    let limit = limit.unwrap_or(20).clamp(1, 50);
    state.with_read(|conn| crate::db::history::list_continue_watching(conn, &profile_id, limit))
}

#[tauri::command(async)]
pub fn add_favorite(
    state: State<'_, SharedDb>,
    profile_id: String,
    item_type: String,
    item_id: String,
) -> AppResult<Favorite> {
    state.with_conn(|conn| {
        crate::db::favorites::add_favorite(conn, &profile_id, &item_type, &item_id)
    })
}

#[tauri::command(async)]
pub fn remove_favorite(
    state: State<'_, SharedDb>,
    profile_id: String,
    item_type: String,
    item_id: String,
) -> AppResult<()> {
    state.with_conn(|conn| {
        crate::db::favorites::remove_favorite(conn, &profile_id, &item_type, &item_id)
    })
}

#[tauri::command(async)]
pub fn list_favorites(
    state: State<'_, SharedDb>,
    profile_id: String,
) -> AppResult<Vec<FavoriteItem>> {
    state.with_read(|conn| crate::db::favorites::list_favorites(conn, &profile_id))
}

#[tauri::command(async)]
pub fn is_favorite(
    state: State<'_, SharedDb>,
    profile_id: String,
    item_type: String,
    item_id: String,
) -> AppResult<bool> {
    state.with_read(|conn| {
        crate::db::favorites::is_favorite(conn, &profile_id, &item_type, &item_id)
    })
}

#[tauri::command(async)]
pub fn search_catalog(
    state: State<'_, SharedDb>,
    profile_id: String,
    query: String,
    limit: Option<i64>,
) -> AppResult<Vec<SearchResult>> {
    let limit = limit.unwrap_or(60).clamp(1, 200);
    state.with_read(|conn| crate::db::search::search_catalog(conn, &profile_id, &query, limit))
}

#[tauri::command]
pub async fn get_channel_epg(
    state: State<'_, SharedDb>,
    profile_id: String,
    channel_id: String,
    limit: Option<i64>,
) -> AppResult<ChannelEpg> {
    epg_service::get_channel_epg(state.inner(), &profile_id, &channel_id, limit).await
}

#[tauri::command(async)]
pub fn get_app_settings(state: State<'_, SharedDb>) -> AppResult<AppSettings> {
    state.with_read(|conn| {
        Ok(AppSettings {
            auto_sync_enabled: settings::get_bool(conn, KEY_AUTO_SYNC, false)?,
            last_background_sync: settings::get_i64(conn, KEY_LAST_BACKGROUND_SYNC)?,
            parental_control_enabled: settings::get_bool(conn, KEY_PARENTAL_ENABLED, false)?,
            parental_pin_set: settings::get_string(conn, KEY_PARENTAL_PIN_HASH)?.is_some(),
        })
    })
}

#[tauri::command(async)]
pub fn update_app_settings(
    state: State<'_, SharedDb>,
    auto_sync_enabled: Option<bool>,
    parental_control_enabled: Option<bool>,
    parental_pin: Option<String>,
) -> AppResult<AppSettings> {
    state.with_conn(|conn| {
        if let Some(enabled) = auto_sync_enabled {
            settings::set_bool(conn, KEY_AUTO_SYNC, enabled)?;
        }
        if let Some(enabled) = parental_control_enabled {
            settings::set_bool(conn, KEY_PARENTAL_ENABLED, enabled)?;
            if enabled {
                let has_pin = settings::get_string(conn, KEY_PARENTAL_PIN_HASH)?.is_some();
                if !has_pin {
                    return Err(AppError::msg(
                        "Defina um PIN de 4 dígitos antes de ativar o controle parental.",
                    ));
                }
            }
        }
        if let Some(pin) = parental_pin {
            let trimmed = pin.trim();
            if trimmed.len() != 4 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
                return Err(AppError::msg("O PIN deve ter exatamente 4 dígitos."));
            }
            settings::set_string(
                conn,
                KEY_PARENTAL_PIN_HASH,
                Some(&parental_service::hash_pin(trimmed)),
            )?;
        }
        Ok(AppSettings {
            auto_sync_enabled: settings::get_bool(conn, KEY_AUTO_SYNC, false)?,
            last_background_sync: settings::get_i64(conn, KEY_LAST_BACKGROUND_SYNC)?,
            parental_control_enabled: settings::get_bool(conn, KEY_PARENTAL_ENABLED, false)?,
            parental_pin_set: settings::get_string(conn, KEY_PARENTAL_PIN_HASH)?.is_some(),
        })
    })
}

#[tauri::command(async)]
pub fn verify_parental_pin(state: State<'_, SharedDb>, pin: String) -> AppResult<bool> {
    state.with_read(|conn| {
        let Some(stored) = settings::get_string(conn, KEY_PARENTAL_PIN_HASH)? else {
            return Ok(false);
        };
        Ok(parental_service::verify_pin(&pin, &stored))
    })
}

#[tauri::command(async)]
pub fn is_adult_category(name: String) -> bool {
    parental_service::is_adult_category(&name)
}

#[tauri::command(async)]
pub fn get_device_fingerprint() -> String {
    license_service::device_fingerprint()
}

#[tauri::command(async)]
pub fn get_license_state(state: State<'_, SharedDb>) -> AppResult<license_service::LicenseState> {
    license_service::get_license_state(state.inner())
}

#[tauri::command]
pub async fn activate_license(
    state: State<'_, SharedDb>,
    api_base_url: String,
    license_key: String,
) -> AppResult<license_service::LicenseState> {
    license_service::activate_license(state.inner(), api_base_url, license_key).await
}

#[tauri::command]
pub async fn validate_license(
    state: State<'_, SharedDb>,
    api_base_url: String,
) -> AppResult<license_service::LicenseState> {
    license_service::ensure_license_valid(state.inner(), api_base_url).await
}

#[tauri::command(async)]
pub fn clear_license(state: State<'_, SharedDb>) -> AppResult<()> {
    license_service::clear_license(state.inner())
}

#[tauri::command(async)]
pub fn update_m3u_profile(
    state: State<'_, SharedDb>,
    profile_id: String,
    name: String,
    url: Option<String>,
    file_path: Option<String>,
) -> AppResult<Profile> {
    let profile = profile_service::update_m3u_profile(
        state.inner(),
        &profile_id,
        name,
        url,
        file_path,
    )?;
    Ok(strip_profile_secrets(profile))
}

#[tauri::command(async)]
pub fn update_xtream_profile(
    state: State<'_, SharedDb>,
    profile_id: String,
    name: String,
    url: String,
    username: String,
    password: String,
) -> AppResult<Profile> {
    let profile = profile_service::update_xtream_profile(
        state.inner(),
        &profile_id,
        name,
        url,
        username,
        password,
    )?;
    Ok(strip_profile_secrets(profile))
}

fn strip_profile_secrets(mut profile: Profile) -> Profile {
    profile.password = None;
    profile
}
