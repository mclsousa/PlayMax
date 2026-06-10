use crate::db::catalog_filters::{is_valid_vod_series, releases_since_ts, CatalogSort};
use crate::db::models::{CategoryCount, Episode, RecommendedItem, Series, SeriesDetail, SeriesPage};
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

const MIN_SERIES_EPISODES: i64 = 2;

fn overfetch_limit(limit: i64) -> i64 {
    (limit.saturating_mul(5)).clamp(limit, 500)
}

pub fn delete_by_profile(conn: &Connection, profile_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM series WHERE profile_id = ?1",
        params![profile_id],
    )?;
    Ok(())
}

pub fn insert_series_batch(conn: &Connection, items: &[Series]) -> AppResult<()> {
    if items.is_empty() {
        return Ok(());
    }
    let mut stmt = conn.prepare(
        "INSERT INTO series (id, profile_id, name, poster, backdrop, plot, genres, rating, category, added_at, sort_order, is_valid_vod)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    for s in items {
        stmt.execute(params![
            s.id,
            s.profile_id,
            s.name,
            s.poster,
            s.backdrop,
            s.plot,
            s.genres,
            s.rating,
            s.category,
            s.added_at,
            s.sort_order,
            is_valid_vod_series(&s.name, s.category.as_deref()) as i64
        ])?;
    }
    Ok(())
}

/// A series without poster needs at least MIN_SERIES_EPISODES episodes to be
/// listable. Episode rows only exist after the episode batch insert, so this
/// runs as a finalization step of the sync transaction (and once at the
/// classification migration).
pub fn refine_series_listability(conn: &Connection, profile_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE series SET is_valid_vod = 0
         WHERE profile_id = ?1
           AND is_valid_vod = 1
           AND (poster IS NULL OR TRIM(poster) = '')
           AND (SELECT COUNT(*) FROM episodes e WHERE e.series_id = series.id) < ?2",
        params![profile_id, MIN_SERIES_EPISODES],
    )?;
    Ok(())
}

/// Re-evaluates a single series after an on-demand episode sync, so a
/// poster-less series that just gained episodes becomes listable again.
pub fn reclassify_series(conn: &Connection, series_id: &str) -> AppResult<()> {
    let Some(series) = get_series(conn, series_id)? else {
        return Ok(());
    };
    let name_valid = is_valid_vod_series(&series.name, series.category.as_deref());
    let listable = if name_valid {
        if has_series_poster(&series) {
            true
        } else {
            let episode_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM episodes WHERE series_id = ?1",
                params![series_id],
                |row| row.get(0),
            )?;
            episode_count >= MIN_SERIES_EPISODES
        }
    } else {
        false
    };
    conn.execute(
        "UPDATE series SET is_valid_vod = ?2 WHERE id = ?1",
        params![series_id, listable as i64],
    )?;
    Ok(())
}

pub fn insert_episodes_batch(conn: &Connection, episodes: &[Episode]) -> AppResult<()> {
    if episodes.is_empty() {
        return Ok(());
    }
    let mut stmt = conn.prepare(
        "INSERT INTO episodes (id, series_id, season, episode, title, plot, stream_url, duration, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for ep in episodes {
        stmt.execute(params![
            ep.id,
            ep.series_id,
            ep.season,
            ep.episode,
            ep.title,
            ep.plot,
            ep.stream_url,
            ep.duration,
            ep.added_at
        ])?;
    }
    Ok(())
}

fn map_series_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Series> {
    Ok(Series {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        name: row.get(2)?,
        poster: row.get(3)?,
        backdrop: row.get(4)?,
        plot: row.get(5)?,
        genres: row.get(6)?,
        rating: row.get(7)?,
        category: row.get(8)?,
        added_at: row.get(9)?,
        sort_order: row.get(10)?,
    })
}

fn map_series_row_with_effective_added_at(row: &rusqlite::Row<'_>) -> rusqlite::Result<Series> {
    Ok(Series {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        name: row.get(2)?,
        poster: row.get(3)?,
        backdrop: row.get(4)?,
        plot: row.get(5)?,
        genres: row.get(6)?,
        rating: row.get(7)?,
        category: row.get(8)?,
        added_at: row.get(9)?,
        sort_order: row.get(10)?,
    })
}

fn list_series_releases_raw(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    releases_since: i64,
    offset: i64,
    limit: i64,
) -> AppResult<Vec<Series>> {
    let mut conditions = vec!["s.profile_id = ?1".to_string(), "s.is_valid_vod = 1".to_string()];
    let mut bind_category: Option<String> = None;
    let mut bind_search: Option<String> = None;
    let mut next_idx = 2_i32;

    if let Some(c) = category.filter(|v| !v.is_empty() && *v != "all") {
        conditions.push(format!("s.category = ?{next_idx}"));
        bind_category = Some(c.to_string());
        next_idx += 1;
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        conditions.push(format!("s.name LIKE ?{next_idx} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
        next_idx += 1;
    }

    let since_placeholder = next_idx;
    next_idx += 1;
    let min_episodes_placeholder = next_idx;

    let where_clause = conditions.join(" AND ");
    let list_sql = format!(
        "SELECT s.id, s.profile_id, s.name, s.poster, s.backdrop, s.plot, s.genres, s.rating, s.category,
                COALESCE(MIN(e.added_at), s.added_at) AS effective_added_at, s.sort_order
         FROM series s
         INNER JOIN episodes e ON e.series_id = s.id
         WHERE {where_clause}
         GROUP BY s.id
         HAVING COUNT(e.id) >= ?{min_episodes_placeholder}
            AND COALESCE(MIN(e.added_at), s.added_at) >= ?{since_placeholder}
         ORDER BY effective_added_at DESC
         LIMIT ? OFFSET ?"
    );

    let mut stmt = conn.prepare(&list_sql)?;
    match (&bind_category, &bind_search) {
        (Some(c), Some(s)) => stmt
            .query_map(
                params![profile_id, c, s, releases_since, MIN_SERIES_EPISODES, limit, offset],
                map_series_row_with_effective_added_at,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (Some(c), None) => stmt
            .query_map(
                params![profile_id, c, releases_since, MIN_SERIES_EPISODES, limit, offset],
                map_series_row_with_effective_added_at,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, Some(s)) => stmt
            .query_map(
                params![profile_id, s, releases_since, MIN_SERIES_EPISODES, limit, offset],
                map_series_row_with_effective_added_at,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, None) => stmt
            .query_map(
                params![profile_id, releases_since, MIN_SERIES_EPISODES, limit, offset],
                map_series_row_with_effective_added_at,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
    }
}

fn count_series_releases_raw(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    releases_since: i64,
) -> AppResult<i64> {
    let mut conditions = vec!["s.profile_id = ?1".to_string(), "s.is_valid_vod = 1".to_string()];
    let mut bind_category: Option<String> = None;
    let mut bind_search: Option<String> = None;
    let mut next_idx = 2_i32;

    if let Some(c) = category.filter(|v| !v.is_empty() && *v != "all") {
        conditions.push(format!("s.category = ?{next_idx}"));
        bind_category = Some(c.to_string());
        next_idx += 1;
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        conditions.push(format!("s.name LIKE ?{next_idx} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
        next_idx += 1;
    }

    let since_placeholder = next_idx;
    next_idx += 1;
    let min_episodes_placeholder = next_idx;

    let where_clause = conditions.join(" AND ");
    let count_sql = format!(
        "SELECT COUNT(*) FROM (
            SELECT s.id
            FROM series s
            INNER JOIN episodes e ON e.series_id = s.id
            WHERE {where_clause}
            GROUP BY s.id
            HAVING COUNT(e.id) >= ?{min_episodes_placeholder}
               AND COALESCE(MIN(e.added_at), s.added_at) >= ?{since_placeholder}
         )"
    );

    match (&bind_category, &bind_search) {
        (Some(c), Some(s)) => conn.query_row(
            &count_sql,
            params![profile_id, c, s, releases_since, MIN_SERIES_EPISODES],
            |r| r.get(0),
        ),
        (Some(c), None) => conn.query_row(
            &count_sql,
            params![profile_id, c, releases_since, MIN_SERIES_EPISODES],
            |r| r.get(0),
        ),
        (None, Some(s)) => conn.query_row(
            &count_sql,
            params![profile_id, s, releases_since, MIN_SERIES_EPISODES],
            |r| r.get(0),
        ),
        (None, None) => conn.query_row(
            &count_sql,
            params![profile_id, releases_since, MIN_SERIES_EPISODES],
            |r| r.get(0),
        ),
    }
    .map_err(AppError::Database)
}

fn list_series_raw(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: CatalogSort,
    releases_since: Option<i64>,
    offset: i64,
    limit: i64,
) -> AppResult<Vec<Series>> {
    if sort == CatalogSort::Releases {
        return list_series_releases_raw(
            conn,
            profile_id,
            category,
            search,
            releases_since.unwrap_or(0),
            offset,
            limit,
        );
    }

    let mut conditions = vec!["profile_id = ?1".to_string(), "is_valid_vod = 1".to_string()];
    let mut bind_category: Option<String> = None;
    let mut bind_search: Option<String> = None;
    let mut next_idx = 2_i32;

    if let Some(c) = category.filter(|v| !v.is_empty() && *v != "all") {
        conditions.push(format!("category = ?{next_idx}"));
        bind_category = Some(c.to_string());
        next_idx += 1;
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        conditions.push(format!("name LIKE ?{next_idx} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
    }

    let where_clause = conditions.join(" AND ");
    let order_by = sort.series_order_by();

    let list_sql = format!(
        "SELECT id, profile_id, name, poster, backdrop, plot, genres, rating, category, added_at, sort_order
         FROM series WHERE {where_clause}
         ORDER BY {order_by}
         LIMIT ? OFFSET ?"
    );

    let mut stmt = conn.prepare(&list_sql)?;
    match (&bind_category, &bind_search) {
        (Some(c), Some(s)) => stmt
            .query_map(params![profile_id, c, s, limit, offset], map_series_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (Some(c), None) => stmt
            .query_map(params![profile_id, c, limit, offset], map_series_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, Some(s)) => stmt
            .query_map(params![profile_id, s, limit, offset], map_series_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
        (None, None) => stmt
            .query_map(params![profile_id, limit, offset], map_series_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database),
    }
}

fn count_series_raw(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: CatalogSort,
    releases_since: Option<i64>,
) -> AppResult<i64> {
    if sort == CatalogSort::Releases {
        return count_series_releases_raw(
            conn,
            profile_id,
            category,
            search,
            releases_since.unwrap_or(0),
        );
    }

    let mut conditions = vec!["profile_id = ?1".to_string(), "is_valid_vod = 1".to_string()];
    let mut bind_category: Option<String> = None;
    let mut bind_search: Option<String> = None;
    let mut next_idx = 2_i32;

    if let Some(c) = category.filter(|v| !v.is_empty() && *v != "all") {
        conditions.push(format!("category = ?{next_idx}"));
        bind_category = Some(c.to_string());
        next_idx += 1;
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        conditions.push(format!("name LIKE ?{next_idx} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
    }

    let where_clause = conditions.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM series WHERE {where_clause}");

    match (&bind_category, &bind_search) {
        (Some(c), Some(s)) => conn.query_row(&count_sql, params![profile_id, c, s], |r| r.get(0)),
        (Some(c), None) => conn.query_row(&count_sql, params![profile_id, c], |r| r.get(0)),
        (None, Some(s)) => conn.query_row(&count_sql, params![profile_id, s], |r| r.get(0)),
        (None, None) => conn.query_row(&count_sql, params![profile_id], |r| r.get(0)),
    }
    .map_err(AppError::Database)
}

fn has_series_poster(series: &Series) -> bool {
    series
        .poster
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .is_some()
}

fn now_unix_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn list_series(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: Option<&str>,
    offset: i64,
    limit: i64,
) -> AppResult<SeriesPage> {
    let sort = CatalogSort::parse(sort);
    let releases_since = if sort == CatalogSort::Releases {
        Some(releases_since_ts(now_unix_ts()))
    } else {
        None
    };
    let items = list_series_raw(
        conn,
        profile_id,
        category,
        search,
        sort,
        releases_since,
        offset.max(0),
        limit,
    )?;
    let total = count_series_raw(conn, profile_id, category, search, sort, releases_since)?;

    Ok(SeriesPage {
        items,
        total,
        offset,
        limit,
    })
}

#[cfg(test)]
pub(crate) fn list_series_with_releases_since(
    conn: &Connection,
    profile_id: &str,
    category: Option<&str>,
    search: Option<&str>,
    sort: CatalogSort,
    releases_since: i64,
    offset: i64,
    limit: i64,
) -> AppResult<SeriesPage> {
    let releases_since = if sort == CatalogSort::Releases {
        Some(releases_since)
    } else {
        None
    };
    let items = list_series_raw(
        conn,
        profile_id,
        category,
        search,
        sort,
        releases_since,
        offset.max(0),
        limit,
    )?;
    let total = count_series_raw(conn, profile_id, category, search, sort, releases_since)?;

    Ok(SeriesPage {
        items,
        total,
        offset,
        limit,
    })
}

pub fn list_series_categories(conn: &Connection, profile_id: &str) -> AppResult<Vec<CategoryCount>> {
    // Listability (junk-name heuristics + poster/episode-count rule) is
    // resolved at sync time into `is_valid_vod`, so this is a plain indexed
    // GROUP BY instead of a full-table scan per call.
    let sql = "SELECT category, COUNT(*) FROM series
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

pub fn get_series(conn: &Connection, id: &str) -> AppResult<Option<Series>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, name, poster, backdrop, plot, genres, rating, category, added_at, sort_order
         FROM series WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(map_series_row(&row)?))
    } else {
        Ok(None)
    }
}

fn map_episode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Episode> {
    Ok(Episode {
        id: row.get(0)?,
        series_id: row.get(1)?,
        season: row.get(2)?,
        episode: row.get(3)?,
        title: row.get(4)?,
        plot: row.get(5)?,
        stream_url: row.get(6)?,
        duration: row.get(7)?,
        added_at: row.get(8)?,
    })
}

pub fn list_episodes_for_series(conn: &Connection, series_id: &str) -> AppResult<Vec<Episode>> {
    let mut stmt = conn.prepare(
        "SELECT id, series_id, season, episode, title, plot, stream_url, duration, added_at
         FROM episodes WHERE series_id = ?1
         ORDER BY season ASC, episode ASC",
    )?;
    let rows = stmt.query_map(params![series_id], map_episode_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(e))
}

pub fn get_series_detail(conn: &Connection, id: &str) -> AppResult<Option<SeriesDetail>> {
    let series = match get_series(conn, id)? {
        Some(s) => s,
        None => return Ok(None),
    };
    let episodes = list_episodes_for_series(conn, id)?;
    Ok(Some(SeriesDetail {
        series,
        episodes,
        cast: None,
        similar: None,
    }))
}

pub fn list_related_series(
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
             FROM series
             WHERE profile_id = ?1 AND id != ?2 AND is_valid_vod = 1 AND category = ?3
             ORDER BY added_at DESC, sort_order ASC
             LIMIT ?4",
        )?
    } else {
        conn.prepare(
            "SELECT id, name, poster, backdrop, category, genres
             FROM series
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
        let row_category: Option<String> = row.get(4)?;
        let row_genres: Option<String> = row.get(5)?;

        if !is_valid_vod_series(&name, row_category.as_deref()) {
            continue;
        }

        let item = RecommendedItem {
            id,
            name,
            poster: row.get(2)?,
            backdrop: row.get(3)?,
        };

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
             FROM series
             WHERE profile_id = ?1 AND is_valid_vod = 1
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
            let row_category: Option<String> = row.get(4)?;
            if !is_valid_vod_series(&name, row_category.as_deref()) {
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

pub fn update_series_metadata(conn: &Connection, series: &Series) -> AppResult<()> {
    conn.execute(
        "UPDATE series SET name = ?2, poster = ?3, backdrop = ?4, plot = ?5, genres = ?6, rating = ?7, category = ?8, added_at = ?9
         WHERE id = ?1",
        params![
            series.id,
            series.name,
            series.poster,
            series.backdrop,
            series.plot,
            series.genres,
            series.rating,
            series.category,
            series.added_at,
        ],
    )?;
    Ok(())
}

pub fn replace_episodes_for_series(
    conn: &Connection,
    series_id: &str,
    episodes: &[Episode],
) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM episodes WHERE series_id = ?1",
        params![series_id],
    )?;
    if !episodes.is_empty() {
        let mut stmt = tx.prepare(
            "INSERT INTO episodes (id, series_id, season, episode, title, plot, stream_url, duration, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for ep in episodes {
            stmt.execute(params![
                ep.id,
                ep.series_id,
                ep.season,
                ep.episode,
                ep.title,
                ep.plot,
                ep.stream_url,
                ep.duration,
                ep.added_at
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::Profile;
    use crate::db::profiles;

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

    fn insert_test_series_with_episodes(
        conn: &Connection,
        series: &Series,
        episode_count: i32,
    ) {
        insert_series_batch(conn, std::slice::from_ref(series)).unwrap();
        let episodes: Vec<Episode> = (0..episode_count)
            .map(|index| Episode {
                id: format!("{}-ep-{index}", series.id),
                series_id: series.id.clone(),
                season: 1,
                episode: index + 1,
                title: format!("Episode {}", index + 1),
                plot: None,
                stream_url: format!("http://example.com/{}/ep-{index}", series.id),
                duration: None,
                added_at: 100 + i64::from(index),
            })
            .collect();
        insert_episodes_batch(conn, &episodes).unwrap();
    }

    #[test]
    fn list_series_excludes_channels_and_requires_poster_or_episodes() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s-good".to_string(),
                profile_id: "p1".to_string(),
                name: "Breaking Bad".to_string(),
                poster: Some("http://example.com/bb.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: 10,
                sort_order: 0,
            },
            3,
        );

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s-channel".to_string(),
                profile_id: "p1".to_string(),
                name: "TC PREMIUM".to_string(),
                poster: Some("http://example.com/tc.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 20,
                sort_order: 0,
            },
            5,
        );

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s-no-poster-one-ep".to_string(),
                profile_id: "p1".to_string(),
                name: "Orphan Series".to_string(),
                poster: None,
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: 30,
                sort_order: 0,
            },
            1,
        );

        insert_series_batch(
            &conn,
            &[Series {
                id: "s-poster-no-eps".to_string(),
                profile_id: "p1".to_string(),
                name: "Poster Only".to_string(),
                poster: Some("http://example.com/poster.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Comedia".to_string()),
                added_at: 40,
                sort_order: 0,
            }],
        )
        .unwrap();

        // The sync pipeline finalizes listability after inserting episodes.
        refine_series_listability(&conn, "p1").unwrap();

        let page = list_series(&conn, "p1", None, None, None, 0, 100).unwrap();
        let ids: Vec<&str> = page.items.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(page.total, 2);
        assert!(ids.contains(&"s-good"));
        assert!(ids.contains(&"s-poster-no-eps"));
        assert!(!ids.contains(&"s-channel"));
        assert!(!ids.contains(&"s-no-poster-one-ep"));
    }

    #[test]
    fn list_series_sort_az_orders_by_name() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s-z".to_string(),
                profile_id: "p1".to_string(),
                name: "Zulu Show (2020)".to_string(),
                poster: Some("http://example.com/z.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: 500,
                sort_order: 0,
            },
            2,
        );
        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s-a".to_string(),
                profile_id: "p1".to_string(),
                name: "Alpha Show (2020)".to_string(),
                poster: Some("http://example.com/a.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: 100,
                sort_order: 1,
            },
            2,
        );

        let page =
            list_series_with_releases_since(&conn, "p1", None, None, CatalogSort::Az, 0, 0, 10)
                .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].name, "Alpha Show (2020)");
        assert_eq!(page.items[1].name, "Zulu Show (2020)");
    }

    #[test]
    fn list_series_sort_releases_uses_effective_episode_added_at() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");
        let since = 1_000_i64;

        insert_series_batch(
            &conn,
            &[Series {
                id: "s-fresh".to_string(),
                profile_id: "p1".to_string(),
                name: "Fresh Series (2026)".to_string(),
                poster: Some("http://example.com/fresh.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: since - 500,
                sort_order: 0,
            }],
        )
        .unwrap();
        insert_episodes_batch(
            &conn,
            &(0..2)
                .map(|index| Episode {
                    id: format!("s-fresh-ep-{index}"),
                    series_id: "s-fresh".to_string(),
                    season: 1,
                    episode: index + 1,
                    title: format!("Episode {}", index + 1),
                    plot: None,
                    stream_url: format!("http://example.com/s-fresh/ep-{index}"),
                    duration: None,
                    added_at: since + 50 + i64::from(index),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s-old".to_string(),
                profile_id: "p1".to_string(),
                name: "Old Series (2020)".to_string(),
                poster: Some("http://example.com/old.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Drama".to_string()),
                added_at: since - 100,
                sort_order: 0,
            },
            2,
        );

        let page = list_series_with_releases_since(
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
        assert_eq!(page.items[0].name, "Fresh Series (2026)");
    }

    #[test]
    fn list_series_categories_excludes_junk() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "Real Show".to_string(),
                poster: Some("http://example.com/show.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Acao".to_string()),
                added_at: 10,
                sort_order: 0,
            },
            2,
        );

        insert_test_series_with_episodes(
            &conn,
            &Series {
                id: "s2".to_string(),
                profile_id: "p1".to_string(),
                name: "MEGAPIX FHD".to_string(),
                poster: Some("http://example.com/megapix.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Acao".to_string()),
                added_at: 20,
                sort_order: 0,
            },
            3,
        );

        let categories = list_series_categories(&conn, "p1").unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Acao");
        assert_eq!(categories[0].count, 1);
    }
}
