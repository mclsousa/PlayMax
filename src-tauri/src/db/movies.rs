use crate::db::catalog_filters::{
    filter_catalog_items, filter_recent_series_items, is_junk_series_name,
    is_valid_vod_movie, releases_since_ts, CatalogSort,
};
use crate::db::models::{CatalogItem, CategoryCount, Movie, MoviesPage, RecommendedItem};
use crate::error::{AppError, AppResult};
use crate::services::content_kind::latest_release_year;
use std::collections::HashSet;
use rusqlite::{params, Connection};

pub fn delete_by_profile(conn: &Connection, profile_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM movies WHERE profile_id = ?1",
        params![profile_id],
    )?;
    Ok(())
}

/// Deletes channel-brand and other non-VOD rows already stored in `movies`.
pub fn cleanup_stale_movies(conn: &Connection, profile_id: &str) -> AppResult<u64> {
    let mut stmt = conn.prepare("SELECT id, name FROM movies WHERE profile_id = ?1")?;
    let rows = stmt
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;

    let stale_ids: Vec<String> = rows
        .into_iter()
        .filter(|(_, name)| !is_valid_vod_movie(name))
        .map(|(id, _)| id)
        .collect();

    let mut deleted = 0_u64;
    for id in stale_ids {
        deleted += conn.execute("DELETE FROM movies WHERE id = ?1", params![id])? as u64;
    }
    Ok(deleted)
}

pub fn insert_batch(conn: &Connection, movies: &[Movie]) -> AppResult<()> {
    if movies.is_empty() {
        return Ok(());
    }
    let mut stmt = conn.prepare(
        "INSERT INTO movies (id, profile_id, name, poster, backdrop, plot, genres, rating, stream_url, category, added_at, sort_order, is_valid_vod)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;
    for movie in movies {
        stmt.execute(params![
            movie.id,
            movie.profile_id,
            movie.name,
            movie.poster,
            movie.backdrop,
            movie.plot,
            movie.genres,
            movie.rating,
            movie.stream_url,
            movie.category,
            movie.added_at,
            movie.sort_order,
            is_valid_vod_movie(&movie.name) as i64
        ])?;
    }
    Ok(())
}

/// Xtream encodes VOD URLs as `.../movie/...`. When a profile has such URLs,
/// rows without the marker are live channels that leaked into `movies`.
/// Runs as a finalization step after the catalog insert (and once at the
/// classification migration), so list queries never re-check URLs per row.
pub fn refine_xtream_vod_classification(conn: &Connection, profile_id: &str) -> AppResult<()> {
    let has_xtream: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM movies
            WHERE profile_id = ?1 AND lower(stream_url) LIKE '%/movie/%'
         )",
        params![profile_id],
        |row| row.get(0),
    )?;
    if has_xtream {
        conn.execute(
            "UPDATE movies SET is_valid_vod = 0
             WHERE profile_id = ?1
               AND is_valid_vod = 1
               AND lower(stream_url) NOT LIKE '%/movie/%'",
            params![profile_id],
        )?;
    }
    Ok(())
}

fn map_movie_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Movie> {
    Ok(Movie {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        name: row.get(2)?,
        poster: row.get(3)?,
        backdrop: row.get(4)?,
        plot: row.get(5)?,
        genres: row.get(6)?,
        rating: row.get(7)?,
        stream_url: row.get(8)?,
        category: row.get(9)?,
        added_at: row.get(10)?,
        sort_order: row.get(11)?,
    })
}

fn list_movies_raw(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: CatalogSort,
    releases_since: Option<i64>,
    offset: i64,
    limit: i64,
) -> AppResult<Vec<Movie>> {
    let mut conditions = vec!["profile_id = ?1".to_string(), "is_valid_vod = 1".to_string()];
    let mut bind_category: Option<String> = None;
    let mut bind_search: Option<String> = None;
    let mut bind_since: Option<i64> = None;
    let mut next_idx = 2_i32;

    if let Some(c) = category.filter(|v| !v.is_empty() && *v != "all") {
        conditions.push(format!("category = ?{next_idx}"));
        bind_category = Some(c.to_string());
        next_idx += 1;
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        conditions.push(format!("name LIKE ?{next_idx} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
        next_idx += 1;
    }

    if sort == CatalogSort::Releases {
        if let Some(since) = releases_since {
            conditions.push(format!("added_at >= ?{next_idx}"));
            bind_since = Some(since);
        }
    }

    let where_clause = conditions.join(" AND ");
    let order_by = sort.movies_order_by();

    let list_sql = format!(
        "SELECT id, profile_id, name, poster, backdrop, plot, genres, rating, stream_url, category, added_at, sort_order
         FROM movies WHERE {where_clause}
         ORDER BY {order_by}
         LIMIT ? OFFSET ?"
    );

    let mut stmt = conn.prepare(&list_sql)?;
    match (&bind_category, &bind_search, &bind_since) {
        (Some(c), Some(s), Some(since)) => stmt
            .query_map(params![profile_id, c, s, since, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (Some(c), Some(s), None) => stmt
            .query_map(params![profile_id, c, s, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (Some(c), None, Some(since)) => stmt
            .query_map(params![profile_id, c, since, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (Some(c), None, None) => stmt
            .query_map(params![profile_id, c, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, Some(s), Some(since)) => stmt
            .query_map(params![profile_id, s, since, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, Some(s), None) => stmt
            .query_map(params![profile_id, s, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, None, Some(since)) => stmt
            .query_map(params![profile_id, since, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, None, None) => stmt
            .query_map(params![profile_id, limit, offset], map_movie_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
    }
}

fn count_movies_raw(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: CatalogSort,
    releases_since: Option<i64>,
) -> AppResult<i64> {
    let mut conditions = vec!["profile_id = ?1".to_string(), "is_valid_vod = 1".to_string()];
    let mut bind_category: Option<String> = None;
    let mut bind_search: Option<String> = None;
    let mut bind_since: Option<i64> = None;
    let mut next_idx = 2_i32;

    if let Some(c) = category.filter(|v| !v.is_empty() && *v != "all") {
        conditions.push(format!("category = ?{next_idx}"));
        bind_category = Some(c.to_string());
        next_idx += 1;
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        conditions.push(format!("name LIKE ?{next_idx} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
        next_idx += 1;
    }

    if sort == CatalogSort::Releases {
        if let Some(since) = releases_since {
            conditions.push(format!("added_at >= ?{next_idx}"));
            bind_since = Some(since);
        }
    }

    let where_clause = conditions.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM movies WHERE {where_clause}");

    match (&bind_category, &bind_search, &bind_since) {
        (Some(c), Some(s), Some(since)) => {
            conn.query_row(&count_sql, params![profile_id, c, s, since], |r| r.get(0))
        }
        (Some(c), Some(s), None) => {
            conn.query_row(&count_sql, params![profile_id, c, s], |r| r.get(0))
        }
        (Some(c), None, Some(since)) => {
            conn.query_row(&count_sql, params![profile_id, c, since], |r| r.get(0))
        }
        (Some(c), None, None) => conn.query_row(&count_sql, params![profile_id, c], |r| r.get(0)),
        (None, Some(s), Some(since)) => {
            conn.query_row(&count_sql, params![profile_id, s, since], |r| r.get(0))
        }
        (None, Some(s), None) => conn.query_row(&count_sql, params![profile_id, s], |r| r.get(0)),
        (None, None, Some(since)) => {
            conn.query_row(&count_sql, params![profile_id, since], |r| r.get(0))
        }
        (None, None, None) => conn.query_row(&count_sql, params![profile_id], |r| r.get(0)),
    }
    .map_err(AppError::Database)
}

fn now_unix_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn list_movies(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: Option<&str>,
    offset: i64,
    limit: i64,
) -> AppResult<MoviesPage> {
    let sort = CatalogSort::parse(sort);
    let releases_since = if sort == CatalogSort::Releases {
        Some(releases_since_ts(now_unix_ts()))
    } else {
        None
    };
    let items = list_movies_raw(
        conn,
        profile_id,
        category,
        search,
        sort,
        releases_since,
        offset.max(0),
        limit,
    )?;
    let total = count_movies_raw(conn, profile_id, category, search, sort, releases_since)?;

    Ok(MoviesPage {
        items,
        total,
        offset,
        limit,
    })
}

#[cfg(test)]
pub(crate) fn list_movies_with_releases_since(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: CatalogSort,
    releases_since: i64,
    offset: i64,
    limit: i64,
) -> AppResult<MoviesPage> {
    let releases_since = if sort == CatalogSort::Releases {
        Some(releases_since)
    } else {
        None
    };
    let items = list_movies_raw(
        conn,
        profile_id,
        category,
        search,
        sort,
        releases_since,
        offset.max(0),
        limit,
    )?;
    let total = count_movies_raw(conn, profile_id, category, search, sort, releases_since)?;

    Ok(MoviesPage {
        items,
        total,
        offset,
        limit,
    })
}

pub fn list_movie_categories(conn: &Connection, profile_id: &str) -> AppResult<Vec<CategoryCount>> {
    // Validity (channel-name heuristics + the Xtream /movie/ URL rule) is
    // resolved at sync time into `is_valid_vod`, so this is a plain indexed
    // GROUP BY instead of a full-table scan per call.
    let sql = "SELECT category, COUNT(*) FROM movies
               WHERE profile_id = ?1 AND is_valid_vod = 1
                 AND category IS NOT NULL AND TRIM(category) != ''
               GROUP BY category";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok(CategoryCount {
            name: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    let mut categories = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;
    categories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(categories)
}

pub fn get_movie(conn: &Connection, id: &str) -> AppResult<Option<Movie>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, name, poster, backdrop, plot, genres, rating, stream_url, category, added_at, sort_order
         FROM movies WHERE id = ?1 AND is_valid_vod = 1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(map_movie_row(&row)?))
    } else {
        Ok(None)
    }
}

pub fn list_related_movies(
    conn: &Connection,
    profile_id: &str,
    exclude_id: &str,
    category: Option<&str>,
    genres: Option<&str>,
    limit: i64,
) -> AppResult<Vec<RecommendedItem>> {
    let fetch_limit = overfetch_limit(limit);
    let category = category.filter(|value| !value.trim().is_empty());
    let genre_token = genres
        .and_then(|value| value.split(',').map(str::trim).find(|part| !part.is_empty()));

    let mut stmt = if category.is_some() {
        conn.prepare(
            "SELECT id, name, poster, backdrop, category, genres
             FROM movies
             WHERE profile_id = ?1 AND id != ?2 AND is_valid_vod = 1 AND category = ?3
             ORDER BY added_at DESC, sort_order ASC
             LIMIT ?4",
        )?
    } else {
        conn.prepare(
            "SELECT id, name, poster, backdrop, category, genres
             FROM movies
             WHERE profile_id = ?1 AND id != ?2 AND is_valid_vod = 1
             ORDER BY added_at DESC, sort_order ASC
             LIMIT ?3",
        )?
    };

    let mut rows = if let Some(category) = category {
        stmt.query(params![profile_id, exclude_id, category, fetch_limit])?
    } else {
        stmt.query(params![profile_id, exclude_id, fetch_limit])?
    };

    let mut same_category = Vec::new();
    let mut genre_matches = Vec::new();
    let mut fallback = Vec::new();

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        if !is_valid_vod_movie(&name) {
            continue;
        }

        let item = RecommendedItem {
            id,
            name,
            poster: row.get(2)?,
            backdrop: row.get(3)?,
        };
        let row_category: Option<String> = row.get(4)?;
        let row_genres: Option<String> = row.get(5)?;

        let matches_genre = genre_token.is_some_and(|token| {
            row_genres
                .as_deref()
                .is_some_and(|genres| genres.to_lowercase().contains(&token.to_lowercase()))
        });

        if category.is_some() && row_category.as_deref() == category {
            same_category.push(item);
        } else if matches_genre {
            genre_matches.push(item);
        } else {
            fallback.push(item);
        }
    }

    let mut related = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(exclude_id.to_string());

    for bucket in [same_category, genre_matches, fallback] {
        for item in bucket {
            if seen.insert(item.id.clone()) {
                related.push(item);
                if related.len() as i64 >= limit {
                    return Ok(related);
                }
            }
        }
    }

    if category.is_some() && (related.len() as i64) < limit {
        let remaining = limit - related.len() as i64;
        let exclude: Vec<String> = seen.iter().cloned().collect();
        let mut extra_stmt = conn.prepare(
            "SELECT id, name, poster, backdrop, category, genres
             FROM movies
             WHERE profile_id = ?1
             ORDER BY added_at DESC, sort_order ASC
             LIMIT ?2",
        )?;
        let mut extra_rows = extra_stmt.query(params![profile_id, overfetch_limit(remaining)])?;
        while let Some(row) = extra_rows.next()? {
            let id: String = row.get(0)?;
            if exclude.contains(&id) {
                continue;
            }
            let name: String = row.get(1)?;
            if !is_valid_vod_movie(&name) {
                continue;
            }
            if seen.insert(id.clone()) {
                related.push(RecommendedItem {
                    id,
                    name,
                    poster: row.get(2)?,
                    backdrop: row.get(3)?,
                });
                if related.len() as i64 >= limit {
                    break;
                }
            }
        }
    }

    Ok(related)
}

fn map_catalog_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CatalogItem> {
    Ok(CatalogItem {
        id: row.get(0)?,
        item_type: row.get(1)?,
        name: row.get(2)?,
        poster: row.get(3)?,
        backdrop: row.get(4)?,
        added_at: row.get(5)?,
    })
}

const MIN_SERIES_EPISODES: i64 = 2;

fn overfetch_limit(limit: i64) -> i64 {
    (limit.saturating_mul(5)).clamp(limit, 500)
}

fn list_recent_movies_raw(
    conn: &Connection,
    profile_id: &str,
    since_ts: Option<i64>,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = if since_ts.is_some() {
        r#"
            SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
            FROM movies
            WHERE profile_id = ?1 AND is_valid_vod = 1 AND added_at >= ?2
            ORDER BY added_at DESC
            LIMIT ?3
        "#
    } else {
        r#"
            SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
            FROM movies
            WHERE profile_id = ?1 AND is_valid_vod = 1
            ORDER BY added_at DESC
            LIMIT ?2
        "#
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(since) = since_ts {
        stmt.query_map(params![profile_id, since, fetch_limit], map_catalog_row)?
    } else {
        stmt.query_map(params![profile_id, fetch_limit], map_catalog_row)?
    };
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn list_recent_movies(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    since_ts: i64,
    releases_since_ts: i64,
) -> AppResult<Vec<CatalogItem>> {
    // Exclude whatever the "releases" carousel shows so the two rows never
    // overlap. Uses the same fallback as list_releases_movies.
    let release_ids: HashSet<String> =
        list_releases_movies(conn, profile_id, limit, releases_since_ts)?
            .into_iter()
            .map(|item| item.id)
            .collect();
    list_recent_movies_excluding(conn, profile_id, limit, since_ts, &release_ids)
}

/// Same as `list_recent_movies`, but takes the "releases" ids the caller
/// already has so the home catalog doesn't run the releases query twice.
pub fn list_recent_movies_excluding(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    since_ts: i64,
    release_ids: &HashSet<String>,
) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit).saturating_mul(3);
    let items = list_recent_movies_raw(conn, profile_id, Some(since_ts), fetch_limit)?;
    let filtered: Vec<CatalogItem> = items
        .into_iter()
        .filter(|item| !release_ids.contains(&item.id))
        .collect();
    let result = filter_catalog_items(filtered, limit as usize);
    if !result.is_empty() {
        return Ok(result);
    }
    // M3U fallback: no timestamps fall in the window. Take the newest movies
    // that are NOT in the "Lançamentos" folder (those already have their own
    // row) and that aren't already shown under "releases".
    let newest = list_recent_non_lancamentos_raw(conn, profile_id, fetch_limit)?;
    let filtered: Vec<CatalogItem> = newest
        .into_iter()
        .filter(|item| !release_ids.contains(&item.id))
        .collect();
    Ok(filter_catalog_items(filtered, limit as usize))
}

fn map_recent_series_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(CatalogItem, Option<String>)> {
    Ok((
        CatalogItem {
            id: row.get(0)?,
            item_type: "series".to_string(),
            name: row.get(1)?,
            poster: row.get(2)?,
            backdrop: row.get(3)?,
            added_at: row.get(4)?,
        },
        row.get(5)?,
    ))
}

fn query_recent_series_candidates(
    conn: &Connection,
    profile_id: &str,
    since_ts: Option<i64>,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = if since_ts.is_some() {
        r#"
            SELECT s.id, s.name, s.poster, s.backdrop,
                   COALESCE(MIN(e.added_at), s.added_at) AS effective_added_at,
                   s.category
            FROM series s
            INNER JOIN episodes e ON e.series_id = s.id
            WHERE s.profile_id = ?1 AND s.is_valid_vod = 1
            GROUP BY s.id
            HAVING COUNT(e.id) >= ?4
               AND COALESCE(MIN(e.added_at), s.added_at) >= ?2
            ORDER BY effective_added_at DESC
            LIMIT ?3
        "#
    } else {
        r#"
            SELECT s.id, s.name, s.poster, s.backdrop,
                   COALESCE(MIN(e.added_at), s.added_at) AS effective_added_at,
                   s.category
            FROM series s
            INNER JOIN episodes e ON e.series_id = s.id
            WHERE s.profile_id = ?1 AND s.is_valid_vod = 1
            GROUP BY s.id
            HAVING COUNT(e.id) >= ?3
            ORDER BY effective_added_at DESC
            LIMIT ?2
        "#
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(since) = since_ts {
        stmt.query_map(
            params![profile_id, since, fetch_limit, MIN_SERIES_EPISODES],
            map_recent_series_row,
        )?
    } else {
        stmt.query_map(
            params![profile_id, fetch_limit, MIN_SERIES_EPISODES],
            map_recent_series_row,
        )?
    };

    let mut items = Vec::new();
    for row in rows {
        let (mut item, category) = row.map_err(AppError::Database)?;
        if is_junk_series_name(&item.name, category.as_deref()) {
            continue;
        }
        item.item_type = "series".to_string();
        items.push(item);
    }
    Ok(items)
}

/// How many series to pull before splitting into "Lançamentos" (recent release
/// year) vs "recently added" (older years). Series have no "Lançamentos" folder,
/// so the release year in the title is the signal; this must be large enough that
/// both buckets can be filled from the newest-added candidates.
const SERIES_SPLIT_FETCH: i64 = 1000;

fn is_release_year_series(name: &str, min_year: i32) -> bool {
    latest_release_year(name).is_some_and(|year| year >= min_year)
}

/// "Séries Lançamentos": series whose title carries a recent release year
/// (>= `min_year`), ordered by how recently they were added. Mirrors the movie
/// "Lançamentos" row, since the provider has no Lançamentos folder for series.
pub fn list_release_series(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    min_year: i32,
) -> AppResult<Vec<CatalogItem>> {
    let candidates = query_recent_series_candidates(conn, profile_id, None, SERIES_SPLIT_FETCH)?;
    let releases: Vec<CatalogItem> = candidates
        .into_iter()
        .filter(|item| is_release_year_series(&item.name, min_year))
        .collect();
    Ok(filter_recent_series_items(releases, limit as usize))
}

/// "Séries adicionadas recentemente": séries novas no catálogo (first_seen_at recente),
/// excluindo lançamentos por ano no título e séries que só ganharam episódios.
pub fn list_recent_series(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    min_year: i32,
    since_ts: i64,
) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit);
    let updated_ids = updated_series_ids(conn, profile_id, since_ts, min_year)?;

    let mut items = query_newly_added_series(conn, profile_id, since_ts, fetch_limit)?;

    if items.is_empty() {
        // M3U lists often have invalid added_at timestamps; fall back to catalog order
        // (same approach as before tracking) so the row is not empty on legacy data.
        items = query_recent_series_candidates(conn, profile_id, None, SERIES_SPLIT_FETCH)?;
    }

    items.retain(|item| {
        !is_release_year_series(&item.name, min_year) && !updated_ids.contains(&item.id)
    });

    Ok(filter_recent_series_items(items, limit as usize))
}

fn updated_series_ids(
    conn: &Connection,
    profile_id: &str,
    since_ts: i64,
    min_year: i32,
) -> AppResult<HashSet<String>> {
    let items = query_updated_series_with_timestamp(
        conn,
        profile_id,
        since_ts,
        SERIES_SPLIT_FETCH,
        min_year,
    )?;
    Ok(items.into_iter().map(|item| item.id).collect())
}

fn query_newly_added_series(
    conn: &Connection,
    profile_id: &str,
    since_ts: i64,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = r#"
        SELECT s.id, s.name, s.poster, s.backdrop,
               t.first_seen_at AS latest_episode_at,
               s.category
        FROM series s
        INNER JOIN series_episode_tracking t ON t.series_id = s.id
        WHERE s.profile_id = ?1
          AND s.is_valid_vod = 1
          AND t.first_seen_at > 0
          AND (?2 = 0 OR t.first_seen_at >= ?2)
          AND t.episode_count >= ?4
          AND NOT (
            t.last_episode_update > 0
            AND t.first_seen_at > 0
            AND t.last_episode_update > t.first_seen_at
          )
        ORDER BY t.first_seen_at DESC
        LIMIT ?3
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        params![profile_id, since_ts, fetch_limit, MIN_SERIES_EPISODES],
        map_recent_series_row,
    )?;

    let mut items = Vec::new();
    for row in rows {
        let (mut item, category) = row.map_err(AppError::Database)?;
        if is_junk_series_name(&item.name, category.as_deref()) {
            continue;
        }
        item.item_type = "series".to_string();
        items.push(item);
    }
    Ok(items)
}

/// Séries existentes que ganharam novos episódios na sincronização.
pub fn list_updated_series(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    since_ts: i64,
    min_year: i32,
) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit);
    let items = query_updated_series_with_timestamp(
        conn,
        profile_id,
        since_ts,
        fetch_limit,
        min_year,
    )?;

    Ok(filter_recent_series_items(items, limit as usize))
}

fn filter_updated_series_candidates(
    rows: impl Iterator<Item = AppResult<(CatalogItem, Option<String>)>>,
    min_year: i32,
) -> AppResult<Vec<CatalogItem>> {
    let mut items = Vec::new();
    for row in rows {
        let (mut item, category) = row?;
        if is_junk_series_name(&item.name, category.as_deref()) {
            continue;
        }
        if is_release_year_series(&item.name, min_year) {
            continue;
        }
        item.item_type = "series".to_string();
        items.push(item);
    }
    Ok(items)
}

fn query_updated_series_with_timestamp(
    conn: &Connection,
    profile_id: &str,
    since_ts: i64,
    fetch_limit: i64,
    min_year: i32,
) -> AppResult<Vec<CatalogItem>> {
    let sql = r#"
        SELECT s.id, s.name, s.poster, s.backdrop,
               t.last_episode_update AS latest_episode_at,
               s.category
        FROM series s
        INNER JOIN series_episode_tracking t ON t.series_id = s.id
        WHERE s.profile_id = ?1
          AND s.is_valid_vod = 1
          AND t.last_episode_update >= ?2
          AND t.last_episode_update > 0
          AND t.episode_count >= ?4
          AND (
            t.first_seen_at = 0
            OR t.last_episode_update > t.first_seen_at
          )
        ORDER BY t.last_episode_update DESC
        LIMIT ?3
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        params![profile_id, since_ts, fetch_limit, MIN_SERIES_EPISODES],
        map_recent_series_row,
    )?;

    filter_updated_series_candidates(rows.map(|r| r.map_err(AppError::Database)), min_year)
}

pub fn list_recent(conn: &Connection, profile_id: &str, limit: i64) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit);
    let sql = r#"
        SELECT id, item_type, name, poster, backdrop, added_at FROM (
            SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
            FROM movies WHERE profile_id = ?1 AND is_valid_vod = 1
            UNION ALL
            SELECT id, 'series' AS item_type, name, poster, backdrop, added_at
            FROM series WHERE profile_id = ?1 AND is_valid_vod = 1
        )
        ORDER BY added_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, fetch_limit], map_catalog_row)?;
    let items = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;
    Ok(filter_catalog_items(items, limit as usize))
}

fn list_releases_movies_raw(
    conn: &Connection,
    profile_id: &str,
    since_ts: i64,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let movie_sql = r#"
        SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
        FROM movies
        WHERE profile_id = ?1 AND is_valid_vod = 1 AND added_at >= ?2
        ORDER BY added_at DESC
        LIMIT ?3
    "#;
    let mut movie_stmt = conn.prepare(movie_sql)?;
    let movie_rows =
        movie_stmt.query_map(params![profile_id, since_ts, fetch_limit], map_catalog_row)?;
    movie_rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

/// Movies that live in the provider's "Lançamentos"/"Estreias" folder, newest
/// first. This is the authoritative "releases" signal for both M3U and Xtream.
fn list_lancamentos_movies_raw(
    conn: &Connection,
    profile_id: &str,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = r#"
        SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
        FROM movies
        WHERE profile_id = ?1
          AND is_valid_vod = 1
          AND category IS NOT NULL
          AND (
            LOWER(category) LIKE '%lançamento%'
            OR LOWER(category) LIKE '%lancamento%'
            OR LOWER(category) LIKE '%estreia%'
          )
        ORDER BY added_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, fetch_limit], map_catalog_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

/// Newest movies that are NOT in the "Lançamentos" folder, so the "recently
/// added" row stays distinct from the "Lançamentos" row.
fn list_recent_non_lancamentos_raw(
    conn: &Connection,
    profile_id: &str,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = r#"
        SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
        FROM movies
        WHERE profile_id = ?1
          AND is_valid_vod = 1
          AND (
            category IS NULL
            OR (
              LOWER(category) NOT LIKE '%lançamento%'
              AND LOWER(category) NOT LIKE '%lancamento%'
              AND LOWER(category) NOT LIKE '%estreia%'
            )
          )
        ORDER BY added_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, fetch_limit], map_catalog_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn list_releases_movies(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    since_ts: i64,
) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit);
    // 1) The provider's real "Lançamentos"/"Estreias" folder (newest first).
    let by_category = list_lancamentos_movies_raw(conn, profile_id, fetch_limit)?;
    let result = filter_catalog_items(by_category, limit as usize);
    if !result.is_empty() {
        return Ok(result);
    }
    // 2) Xtream lists with real timestamps: items added inside the time window.
    let windowed = list_releases_movies_raw(conn, profile_id, since_ts, fetch_limit)?;
    let result = filter_catalog_items(windowed, limit as usize);
    if !result.is_empty() {
        return Ok(result);
    }
    // 3) Last resort (no folder, no timestamps): newest items by added_at.
    let newest = list_recent_movies_raw(conn, profile_id, None, fetch_limit.saturating_mul(3))?;
    Ok(filter_catalog_items(newest, limit as usize))
}

pub fn list_releases_series(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    since_ts: i64,
) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit);
    let windowed = query_recent_series_candidates(conn, profile_id, Some(since_ts), fetch_limit)?;
    let result = filter_recent_series_items(windowed, limit as usize);
    if !result.is_empty() {
        return Ok(result);
    }
    // M3U fallback: newest series by effective added_at, ignoring the window.
    let newest = query_recent_series_candidates(conn, profile_id, None, fetch_limit)?;
    Ok(filter_recent_series_items(newest, limit as usize))
}

pub fn list_releases(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
    since_ts: i64,
) -> AppResult<Vec<CatalogItem>> {
    let movies = list_releases_movies(conn, profile_id, limit, since_ts)?;
    let series = list_releases_series(conn, profile_id, limit, since_ts)?;
    let mut items = movies;
    items.extend(series);
    items.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    items.truncate(limit as usize);
    Ok(items)
}

fn list_featured_movies_raw(
    conn: &Connection,
    profile_id: &str,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = r#"
        SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
        FROM movies
        WHERE profile_id = ?1
          AND is_valid_vod = 1
          AND category IS NOT NULL
          AND (
            LOWER(category) LIKE '%lançamento%'
            OR LOWER(category) LIKE '%lancamento%'
            OR LOWER(category) LIKE '%destaque%'
          )
        ORDER BY added_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, fetch_limit], map_catalog_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

fn list_top_rated_movies_raw(
    conn: &Connection,
    profile_id: &str,
    fetch_limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let sql = r#"
        SELECT id, 'movie' AS item_type, name, poster, backdrop, added_at
        FROM movies
        WHERE profile_id = ?1
          AND is_valid_vod = 1
          AND rating IS NOT NULL
          AND TRIM(rating) != ''
        ORDER BY CAST(rating AS REAL) DESC, added_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, fetch_limit], map_catalog_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn list_featured_movies(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
) -> AppResult<Vec<CatalogItem>> {
    let fetch_limit = overfetch_limit(limit);
    let category_items = list_featured_movies_raw(conn, profile_id, fetch_limit)?;
    let items = if category_items.is_empty() {
        list_top_rated_movies_raw(conn, profile_id, fetch_limit)?
    } else {
        category_items
    };
    Ok(filter_catalog_items(items, limit as usize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::Profile;
    use crate::db::profiles;
    use crate::db::series;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn insert_test_profile(conn: &Connection, id: &str) {
        let profile = Profile {
            id: id.to_string(),
            name: "Test".to_string(),
            profile_type: "m3u".to_string(),
            url: None,
            file_path: None,
            username: None,
            password: None,
            last_sync: None,
        };
        profiles::insert_profile(conn, &profile).unwrap();
    }

    #[test]
    fn list_recent_movies_filters_channel_like_entries() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m1".to_string(),
                    profile_id: "p1".to_string(),
                    name: "TC PREMIUM".to_string(),
                    poster: Some("http://example.com/tc.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/tc".to_string(),
                    category: None,
                    added_at: 500,
                    sort_order: 0,
                },
                Movie {
                    id: "m-megapix".to_string(),
                    profile_id: "p1".to_string(),
                    name: "MEGAPIX FHD".to_string(),
                    poster: Some("http://example.com/megapix.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/megapix".to_string(),
                    category: None,
                    added_at: 450,
                    sort_order: 0,
                },
                Movie {
                    id: "m-universal".to_string(),
                    profile_id: "p1".to_string(),
                    name: "UNIVERSAL TV".to_string(),
                    poster: Some("http://example.com/universal.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/universal".to_string(),
                    category: None,
                    added_at: 400,
                    sort_order: 0,
                },
                Movie {
                    id: "m-studio".to_string(),
                    profile_id: "p1".to_string(),
                    name: "STUDIO UNIVERSAL".to_string(),
                    poster: Some("http://example.com/studio.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/studio".to_string(),
                    category: None,
                    added_at: 350,
                    sort_order: 0,
                },
                Movie {
                    id: "m2".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Real Movie".to_string(),
                    poster: Some("http://example.com/movie.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/m2".to_string(),
                    category: None,
                    added_at: 100,
                    sort_order: 0,
                },
                Movie {
                    id: "m3".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Another Real Film (2020)".to_string(),
                    poster: Some("http://example.com/another.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/m3".to_string(),
                    category: None,
                    added_at: 600,
                    sort_order: 0,
                },
            ],
        )
        .unwrap();

        // With no real timestamps the newest valid movie surfaces under
        // "releases"; "recent" gets the next newest. Channel-like entries
        // (TC PREMIUM, MEGAPIX FHD, ...) must never appear in either row.
        let releases = list_releases_movies(&conn, "p1", 1, i64::MAX).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "Another Real Film (2020)");

        let recent = list_recent_movies(&conn, "p1", 1, 0, i64::MAX).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "Real Movie");
        assert!(
            !recent
                .iter()
                .any(|m| m.name.contains("PREMIUM") || m.name.contains("MEGAPIX")),
            "channel-like entries must not appear in recent"
        );
    }

    #[test]
    fn list_recent_returns_movies_and_series_ordered_by_added_at() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[Movie {
                id: "m1".to_string(),
                profile_id: "p1".to_string(),
                name: "Old Movie".to_string(),
                poster: Some("http://example.com/old.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/m1".to_string(),
                category: None,
                added_at: 100,
                sort_order: 0,
            }],
        )
        .unwrap();

        series::insert_series_batch(
            &conn,
            &[crate::db::models::Series {
                id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "New Series".to_string(),
                poster: Some("http://example.com/new-series.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: None,
                added_at: 300,
                sort_order: 0,
            }],
        )
        .unwrap();

        insert_batch(
            &conn,
            &[Movie {
                id: "m2".to_string(),
                profile_id: "p1".to_string(),
                name: "Mid Movie".to_string(),
                poster: Some("http://example.com/mid.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/m2".to_string(),
                category: None,
                added_at: 200,
                sort_order: 0,
            }],
        )
        .unwrap();

        let recent = list_recent(&conn, "p1", 10).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "s1");
        assert_eq!(recent[0].item_type, "series");
        assert_eq!(recent[1].id, "m2");
        assert_eq!(recent[1].item_type, "movie");
        assert_eq!(recent[2].id, "m1");
    }

    fn insert_test_series_with_episodes(
        conn: &Connection,
        series: &crate::db::models::Series,
        episode_count: i32,
        latest_episode_added_at: i64,
    ) {
        series::insert_series_batch(conn, std::slice::from_ref(series)).unwrap();
        let episodes: Vec<crate::db::models::Episode> = (0..episode_count)
            .map(|index| crate::db::models::Episode {
                id: format!("{}-ep-{index}", series.id),
                series_id: series.id.clone(),
                season: 1,
                episode: index + 1,
                title: format!("Episode {}", index + 1),
                plot: None,
                stream_url: format!("http://example.com/{}/ep-{index}", series.id),
                duration: None,
                added_at: if index + 1 == episode_count {
                    latest_episode_added_at
                } else {
                    latest_episode_added_at - i64::from(episode_count - index)
                },
            })
            .collect();
        series::insert_episodes_batch(conn, &episodes).unwrap();
    }

    #[test]
    fn list_recent_series_dedupes_and_sorts_by_latest_episode() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-multi".to_string(),
                profile_id: "p1".to_string(),
                name: "Breaking Bad".to_string(),
                poster: Some("http://example.com/bb.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 10,
                sort_order: 0,
            },
            3,
            500,
        );

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-one-off".to_string(),
                profile_id: "p1".to_string(),
                name: "Contratempo (2023)".to_string(),
                poster: Some("http://example.com/ct.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 900,
                sort_order: 0,
            },
            1,
            900,
        );

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-dup-a".to_string(),
                profile_id: "p1".to_string(),
                name: "Pacificador".to_string(),
                poster: Some("http://example.com/pac-a.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 300,
                sort_order: 0,
            },
            2,
            400,
        );
        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-dup-b".to_string(),
                profile_id: "p1".to_string(),
                name: "PACIFICADOR".to_string(),
                poster: Some("http://example.com/pac-b.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 450,
                sort_order: 0,
            },
            2,
            450,
        );

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-junk".to_string(),
                profile_id: "p1".to_string(),
                name: "Séries".to_string(),
                poster: Some("http://example.com/group.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 1000,
                sort_order: 0,
            },
            4,
            1000,
        );

        // Ordered by when each series first appeared (MIN episode added_at).
        let recent = list_recent_series(&conn, "p1", 20, 2025, 0).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].name, "Breaking Bad");
        assert_eq!(recent[0].added_at, 497);
        assert_eq!(recent[1].name, "PACIFICADOR");
        assert_eq!(recent[1].added_at, 448);
        assert!(recent.iter().all(|item| item.item_type == "series"));
        assert!(!recent.iter().any(|item| item.name == "Séries"));
        assert!(!recent.iter().any(|item| item.name == "Contratempo (2023)"));
    }

    #[test]
    fn list_recent_series_includes_titles_with_release_year() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-vikings".to_string(),
                profile_id: "p1".to_string(),
                name: "Vikings (2013)".to_string(),
                poster: Some("http://example.com/vikings.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 100,
                sort_order: 0,
            },
            5,
            800,
        );

        // 2013 is older than the threshold, so it belongs in "recently added".
        let recent = list_recent_series(&conn, "p1", 10, 2025, 0).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "Vikings (2013)");
    }

    #[test]
    fn list_updated_series_shows_series_with_new_episodes_only() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");
        let now = chrono::Utc::now().timestamp();
        let old = now - 90 * 24 * 60 * 60;
        let recent = now - 2 * 24 * 60 * 60;

        series::insert_series_batch(
            &conn,
            &[crate::db::models::Series {
                id: "s-updated".to_string(),
                profile_id: "p1".to_string(),
                name: "The Last of Us".to_string(),
                poster: Some("http://example.com/tlou.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: old,
                sort_order: 0,
            }],
        )
        .unwrap();
        series::insert_episodes_batch(
            &conn,
            &[
                crate::db::models::Episode {
                    id: "s-updated-ep-1".to_string(),
                    series_id: "s-updated".to_string(),
                    season: 1,
                    episode: 1,
                    title: "Episode 1".to_string(),
                    plot: None,
                    stream_url: "http://example.com/1".to_string(),
                    duration: None,
                    added_at: old,
                },
                crate::db::models::Episode {
                    id: "s-updated-ep-2".to_string(),
                    series_id: "s-updated".to_string(),
                    season: 1,
                    episode: 2,
                    title: "Episode 2".to_string(),
                    plot: None,
                    stream_url: "http://example.com/2".to_string(),
                    duration: None,
                    added_at: recent,
                },
            ],
        )
        .unwrap();

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-new-batch".to_string(),
                profile_id: "p1".to_string(),
                name: "Nova Série Completa".to_string(),
                poster: Some("http://example.com/new.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: recent,
                sort_order: 0,
            },
            2,
            recent,
        );

        conn.execute(
            "INSERT INTO series_episode_tracking
             (series_id, profile_id, episode_count, max_season, max_episode,
              last_episode_update, first_seen_at)
             VALUES ('s-updated', 'p1', 2, 1, 2, ?1, ?2)",
            rusqlite::params![recent, old],
        )
        .unwrap();

        let since_ts = now - 30 * 24 * 60 * 60;
        let updated = list_updated_series(&conn, "p1", 10, since_ts, 2025).unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].name, "The Last of Us");
        assert_eq!(updated[0].added_at, recent);
    }

    #[test]
    fn list_updated_and_recent_series_do_not_overlap() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");
        let now = chrono::Utc::now().timestamp();
        let old = now - 90 * 24 * 60 * 60;
        let recent = now - 2 * 24 * 60 * 60;

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-existing".to_string(),
                profile_id: "p1".to_string(),
                name: "Ozark".to_string(),
                poster: Some("http://example.com/oz.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: old,
                sort_order: 0,
            },
            3,
            old,
        );
        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-new".to_string(),
                profile_id: "p1".to_string(),
                name: "Fire Country".to_string(),
                poster: Some("http://example.com/fc.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: recent,
                sort_order: 0,
            },
            2,
            recent,
        );

        conn.execute(
            "INSERT INTO series_episode_tracking
             (series_id, profile_id, episode_count, max_season, max_episode,
              last_episode_update, first_seen_at)
             VALUES ('s-existing', 'p1', 3, 1, 3, ?1, ?2)",
            rusqlite::params![recent, old],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO series_episode_tracking
             (series_id, profile_id, episode_count, max_season, max_episode,
              last_episode_update, first_seen_at)
             VALUES ('s-new', 'p1', 2, 1, 2, 0, ?1)",
            rusqlite::params![recent],
        )
        .unwrap();

        let since_ts = now - 30 * 24 * 60 * 60;
        let updated = list_updated_series(&conn, "p1", 10, since_ts, 2025).unwrap();
        let recent_series = list_recent_series(&conn, "p1", 10, 2025, since_ts).unwrap();

        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].name, "Ozark");
        assert_eq!(recent_series.len(), 1);
        assert_eq!(recent_series[0].name, "Fire Country");
    }

    #[test]
    fn release_series_split_by_year() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        // Recent release (year in title >= threshold), added most recently.
        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-new".to_string(),
                profile_id: "p1".to_string(),
                name: "A Dama [2026]".to_string(),
                poster: Some("http://example.com/dama.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("SERIES A".to_string()),
                added_at: 9000,
                sort_order: 0,
            },
            3,
            9000,
        );
        // Old show, also added recently (catalog backfill) but old release year.
        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-old".to_string(),
                profile_id: "p1".to_string(),
                name: "Sessão de Terapia [2012]".to_string(),
                poster: Some("http://example.com/sessao.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("SERIES S".to_string()),
                added_at: 8000,
                sort_order: 0,
            },
            3,
            8000,
        );

        let releases = list_release_series(&conn, "p1", 10, 2025).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "A Dama [2026]");

        let recent = list_recent_series(&conn, "p1", 10, 2025, 0).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "Sessão de Terapia [2012]");
    }

    #[test]
    fn list_recent_movies_excludes_release_window_items() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        let releases_since = 1_700_000_000_i64;
        let recent_since = releases_since - 60 * 24 * 60 * 60;

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-release".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Fresh Release (2026)".to_string(),
                    poster: Some("http://example.com/fresh.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/fresh".to_string(),
                    category: None,
                    added_at: releases_since + 100,
                    sort_order: 0,
                },
                Movie {
                    id: "m-older".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Older Addition (2025)".to_string(),
                    poster: Some("http://example.com/older.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/older".to_string(),
                    category: None,
                    added_at: releases_since - 10,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let releases = list_releases_movies(&conn, "p1", 10, releases_since).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].id, "m-release");

        let recent = list_recent_movies(&conn, "p1", 10, recent_since, releases_since).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "m-older");
        assert!(!recent.iter().any(|item| item.id == "m-release"));
    }

    #[test]
    fn list_featured_movies_falls_back_to_top_rated() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-low".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Low Rated".to_string(),
                    poster: Some("http://example.com/low.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: Some("4.0".to_string()),
                    stream_url: "http://example.com/low".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 100,
                    sort_order: 0,
                },
                Movie {
                    id: "m-high".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Top Rated".to_string(),
                    poster: Some("http://example.com/high.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: Some("9.5".to_string()),
                    stream_url: "http://example.com/high".to_string(),
                    category: Some("Action".to_string()),
                    added_at: 50,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let featured = list_featured_movies(&conn, "p1", 1).unwrap();
        assert_eq!(featured.len(), 1);
        assert_eq!(featured[0].name, "Top Rated");
    }

    #[test]
    fn list_featured_movies_prefers_launch_category() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-featured".to_string(),
                    profile_id: "p1".to_string(),
                    name: "A CASA DE VERAO (2026)".to_string(),
                    poster: Some("http://example.com/casa.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/casa".to_string(),
                    category: Some("Lançamentos".to_string()),
                    added_at: 1_700_000_100,
                    sort_order: 0,
                },
                Movie {
                    id: "m-other".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Old Drama".to_string(),
                    poster: Some("http://example.com/drama.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/drama".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 1_700_000_200,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let featured = list_featured_movies(&conn, "p1", 10).unwrap();
        assert_eq!(featured.len(), 1);
        assert_eq!(featured[0].name, "A CASA DE VERAO (2026)");
    }

    #[test]
    fn list_releases_filters_by_since_ts() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        let since_ts = 1_700_000_000_i64;

        insert_batch(
            &conn,
            &[Movie {
                id: "m-old".to_string(),
                profile_id: "p1".to_string(),
                name: "Old".to_string(),
                poster: Some("http://example.com/old-release.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/old".to_string(),
                category: None,
                added_at: since_ts - 1000,
                sort_order: 0,
            }],
        )
        .unwrap();

        insert_batch(
            &conn,
            &[Movie {
                id: "m-new".to_string(),
                profile_id: "p1".to_string(),
                name: "New Release".to_string(),
                poster: Some("http://example.com/new-release.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/new".to_string(),
                category: None,
                added_at: since_ts + 100,
                sort_order: 0,
            }],
        )
        .unwrap();

        insert_test_series_with_episodes(
            &conn,
            &crate::db::models::Series {
                id: "s-new".to_string(),
                profile_id: "p1".to_string(),
                name: "Recent Series".to_string(),
                poster: Some("http://example.com/recent-series.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: since_ts + 10,
                sort_order: 0,
            },
            2,
            since_ts + 50,
        );

        let releases = list_releases(&conn, "p1", 10, since_ts).unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].id, "m-new");
        assert_eq!(releases[1].id, "s-new");
        assert_eq!(releases[1].item_type, "series");
    }

    #[test]
    fn list_releases_excludes_channel_brands() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        let since_ts = 1_700_000_000_i64;

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-channel".to_string(),
                    profile_id: "p1".to_string(),
                    name: "MEGAPIX FHD".to_string(),
                    poster: Some("http://example.com/megapix.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/megapix".to_string(),
                    category: Some("Lançamentos".to_string()),
                    added_at: since_ts + 200,
                    sort_order: 0,
                },
                Movie {
                    id: "m-real".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Dune: Part Two (2024)".to_string(),
                    poster: Some("http://example.com/dune.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/dune".to_string(),
                    category: Some("Lançamentos".to_string()),
                    added_at: since_ts + 100,
                    sort_order: 0,
                },
            ],
        )
        .unwrap();

        let releases = list_releases(&conn, "p1", 10, since_ts).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].id, "m-real");
        assert_eq!(releases[0].item_type, "movie");
    }

    #[test]
    fn releases_prefers_lancamentos_folder_and_recent_excludes_it() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                // Newest by added_at, but NOT in the Lançamentos folder.
                Movie {
                    id: "m-new-drama".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Novo Drama (2025)".to_string(),
                    poster: Some("http://example.com/drama.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://host/movie/u/p/9000.mp4".to_string(),
                    category: Some("DRAMA".to_string()),
                    added_at: 9000,
                    sort_order: 0,
                },
                // In the Lançamentos folder, older added_at.
                Movie {
                    id: "m-lanc".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Estreia Quente (2026)".to_string(),
                    poster: Some("http://example.com/estreia.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://host/movie/u/p/5000.mp4".to_string(),
                    category: Some("LANCAMENTOS".to_string()),
                    added_at: 5000,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        // No real timestamps fall in the window, so the folder must win.
        let releases = list_releases_movies(&conn, "p1", 10, i64::MAX).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "Estreia Quente (2026)");

        // "Recently added" must not repeat the Lançamentos item.
        let recent = list_recent_movies(&conn, "p1", 10, 0, i64::MAX).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "Novo Drama (2025)");
    }

    #[test]
    fn list_movies_rejects_short_titles_without_year() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-short".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Old".to_string(),
                    poster: Some("http://example.com/old.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/old".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 200,
                    sort_order: 0,
                },
                Movie {
                    id: "m-real".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Inception (2010)".to_string(),
                    poster: Some("http://example.com/inception.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/inception".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 100,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let page = list_movies(&conn, "p1", None, None, None, 0, 10).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "Inception (2010)");
    }

    #[test]
    fn list_movies_excludes_channel_brands() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-tc".to_string(),
                    profile_id: "p1".to_string(),
                    name: "TC ACTION".to_string(),
                    poster: Some("http://example.com/tc.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/tc".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 500,
                    sort_order: 0,
                },
                Movie {
                    id: "m-megapix".to_string(),
                    profile_id: "p1".to_string(),
                    name: "MEGAPIX FHD*".to_string(),
                    poster: Some("http://example.com/megapix.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/megapix".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 450,
                    sort_order: 0,
                },
                Movie {
                    id: "m-usa".to_string(),
                    profile_id: "p1".to_string(),
                    name: "USA HDᴮᴿ".to_string(),
                    poster: Some("http://example.com/usa.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/usa".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 400,
                    sort_order: 0,
                },
                Movie {
                    id: "m-universal".to_string(),
                    profile_id: "p1".to_string(),
                    name: "UNIVERSAL PREMIER".to_string(),
                    poster: Some("http://example.com/universal.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/universal".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 350,
                    sort_order: 0,
                },
                Movie {
                    id: "m-real".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Inception (2010)".to_string(),
                    poster: Some("http://example.com/inception.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/inception".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 100,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let page = list_movies(&conn, "p1", None, None, None, 0, 10).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "Inception (2010)");

        let categories = list_movie_categories(&conn, "p1").unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Acao");
        assert_eq!(categories[0].count, 1);
    }

    #[test]
    fn cleanup_stale_movies_purges_channel_brands_from_db() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-megapix".to_string(),
                    profile_id: "p1".to_string(),
                    name: "MEGAPIX FHDᴮᴿ".to_string(),
                    poster: Some("http://example.com/megapix.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/megapix".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 500,
                    sort_order: 0,
                },
                Movie {
                    id: "m-usa".to_string(),
                    profile_id: "p1".to_string(),
                    name: "USA FHD*".to_string(),
                    poster: Some("http://example.com/usa.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/usa".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 450,
                    sort_order: 0,
                },
                Movie {
                    id: "m-universal".to_string(),
                    profile_id: "p1".to_string(),
                    name: "UNIVERSAL PREMIER".to_string(),
                    poster: Some("http://example.com/universal.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/universal".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 400,
                    sort_order: 0,
                },
                Movie {
                    id: "m-real".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Inception (2010)".to_string(),
                    poster: Some("http://example.com/inception.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/inception".to_string(),
                    category: Some("Acao".to_string()),
                    added_at: 100,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let deleted = cleanup_stale_movies(&conn, "p1").unwrap();
        assert_eq!(deleted, 3);

        let page = list_movies(&conn, "p1", None, None, None, 0, 10).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].name, "Inception (2010)");
    }

    #[test]
    fn delete_by_profile_removes_movies() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[Movie {
                id: "m1".to_string(),
                profile_id: "p1".to_string(),
                name: "Movie".to_string(),
                poster: None,
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/m1".to_string(),
                category: None,
                added_at: 100,
                sort_order: 0,
            }],
        )
        .unwrap();

        delete_by_profile(&conn, "p1").unwrap();
        let page = list_movies(&conn, "p1", None, None, None, 0, 10).unwrap();
        assert_eq!(page.total, 0);
    }

    #[test]
    fn list_movies_sort_az_orders_by_name() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-z".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Zulu Movie (2020)".to_string(),
                    poster: Some("http://example.com/z.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/z".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 500,
                    sort_order: 0,
                },
                Movie {
                    id: "m-a".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Alpha Movie (2020)".to_string(),
                    poster: Some("http://example.com/a.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/a".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 100,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let page =
            list_movies_with_releases_since(&conn, "p1", None, None, CatalogSort::Az, 0, 0, 10)
                .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].name, "Alpha Movie (2020)");
        assert_eq!(page.items[1].name, "Zulu Movie (2020)");
    }

    #[test]
    fn list_movies_sort_recent_orders_by_added_at_desc() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-old".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Older Movie (2019)".to_string(),
                    poster: Some("http://example.com/old.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/old".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 100,
                    sort_order: 0,
                },
                Movie {
                    id: "m-new".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Newer Movie (2024)".to_string(),
                    poster: Some("http://example.com/new.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/new".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: 900,
                    sort_order: 1,
                },
            ],
        )
        .unwrap();

        let page = list_movies_with_releases_since(
            &conn,
            "p1",
            None,
            None,
            CatalogSort::Recent,
            0,
            0,
            10,
        )
        .unwrap();
        assert_eq!(page.items[0].name, "Newer Movie (2024)");
        assert_eq!(page.items[1].name, "Older Movie (2019)");
    }

    #[test]
    fn list_movies_sort_releases_filters_by_added_at_window() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");
        let since = 1_000_i64;

        insert_batch(
            &conn,
            &[
                Movie {
                    id: "m-in".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Fresh Release (2026)".to_string(),
                    poster: Some("http://example.com/in.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/in".to_string(),
                    category: Some("Lancamentos".to_string()),
                    added_at: since + 50,
                    sort_order: 0,
                },
                Movie {
                    id: "m-out".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Old Release (2020)".to_string(),
                    poster: Some("http://example.com/out.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/out".to_string(),
                    category: Some("Drama".to_string()),
                    added_at: since - 50,
                    sort_order: 0,
                },
            ],
        )
        .unwrap();

        let page = list_movies_with_releases_since(
            &conn,
            "p1",
            None,
            None,
            CatalogSort::Releases,
            since,
            0,
            10,
        )
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "Fresh Release (2026)");
    }
}
