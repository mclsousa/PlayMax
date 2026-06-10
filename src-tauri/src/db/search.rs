use crate::db::catalog_fts;
use crate::db::models::SearchResult;
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

fn escape_like(query: &str) -> String {
    format!(
        "%{}%",
        query
            .trim()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

fn prepare_fts_query(query: &str) -> String {
    query
        .trim()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| {
            let escaped = token.replace('"', "\"\"");
            format!("\"{escaped}\"*")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn map_search_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchResult> {
    Ok(SearchResult {
        id: row.get(0)?,
        item_type: row.get(1)?,
        name: row.get(2)?,
        poster: row.get(3)?,
        category: row.get(4)?,
    })
}

fn search_catalog_fts(
    conn: &Connection,
    profile_id: &str,
    query: &str,
    limit: i64,
) -> AppResult<Vec<SearchResult>> {
    let fts_query = prepare_fts_query(query);
    if fts_query.is_empty() {
        return Ok(Vec::new());
    }

    let per_type_limit = (limit / 3).max(1).min(limit);
    let sql = r#"
        SELECT item_id, item_type, name, poster, category FROM catalog_fts
        WHERE profile_id = ?1 AND catalog_fts MATCH ?2
        ORDER BY bm25(catalog_fts)
        LIMIT ?3
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, fts_query, limit * 2], map_search_row)?;

    let mut channels = 0_i64;
    let mut movies = 0_i64;
    let mut series = 0_i64;
    let mut results = Vec::new();

    for row in rows {
        let item = row.map_err(AppError::Database)?;
        let count = match item.item_type.as_str() {
            "channel" => {
                if channels >= per_type_limit {
                    continue;
                }
                channels += 1;
                channels
            }
            "movie" => {
                if movies >= per_type_limit {
                    continue;
                }
                movies += 1;
                movies
            }
            "series" => {
                if series >= per_type_limit {
                    continue;
                }
                series += 1;
                series
            }
            _ => continue,
        };
        let _ = count;
        results.push(item);
        if results.len() as i64 >= limit {
            break;
        }
    }

    Ok(results)
}

fn search_catalog_like(
    conn: &Connection,
    profile_id: &str,
    query: &str,
    limit: i64,
) -> AppResult<Vec<SearchResult>> {
    let pattern = escape_like(query);
    let per_type_limit = (limit / 3).max(1).min(limit);

    let sql = r#"
        SELECT id, item_type, name, poster, category FROM (
            SELECT id, 'channel' AS item_type, name, logo AS poster, group_name AS category
            FROM channels
            WHERE profile_id = ?1 AND name LIKE ?2 ESCAPE '\'
            ORDER BY name COLLATE NOCASE ASC
            LIMIT ?3
        )
        UNION ALL
        SELECT id, item_type, name, poster, category FROM (
            SELECT id, 'movie' AS item_type, name, poster, category
            FROM movies
            WHERE profile_id = ?1 AND name LIKE ?2 ESCAPE '\'
            ORDER BY name COLLATE NOCASE ASC
            LIMIT ?3
        )
        UNION ALL
        SELECT id, item_type, name, poster, category FROM (
            SELECT id, 'series' AS item_type, name, poster, category
            FROM series
            WHERE profile_id = ?1 AND name LIKE ?2 ESCAPE '\'
            ORDER BY name COLLATE NOCASE ASC
            LIMIT ?3
        )
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        params![profile_id, pattern, per_type_limit],
        map_search_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn search_catalog(
    conn: &Connection,
    profile_id: &str,
    query: &str,
    limit: i64,
) -> AppResult<Vec<SearchResult>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if catalog_fts::fts_available(conn) && catalog_fts::profile_has_entries(conn, profile_id)? {
        let fts_results = search_catalog_fts(conn, profile_id, trimmed, limit)?;
        if !fts_results.is_empty() {
            return Ok(fts_results);
        }
    }

    search_catalog_like(conn, profile_id, trimmed, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::catalog_fts;
    use crate::db::channels;
    use crate::db::migrations;
    use crate::db::models::{Channel, Movie, Profile, Series};
    use crate::db::movies;
    use crate::db::profiles;
    use crate::db::series;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn seed_catalog(conn: &Connection) {
        profiles::insert_profile(
            &conn,
            &Profile {
                id: "p1".to_string(),
                name: "Test".to_string(),
                profile_type: "m3u".to_string(),
                url: None,
                file_path: None,
                username: None,
                password: None,
                last_sync: None,
            },
        )
        .unwrap();

        let channel = Channel {
            id: "c1".to_string(),
            profile_id: "p1".to_string(),
            name: "Globo News".to_string(),
            logo: None,
            group_name: Some("Noticias".to_string()),
            stream_url: "http://example.com/globo".to_string(),
            tvg_id: None,
            sort_order: 0,
        };
        let movie = Movie {
            id: "m1".to_string(),
            profile_id: "p1".to_string(),
            name: "Globo Filme".to_string(),
            poster: None,
            backdrop: None,
            plot: None,
            genres: None,
            rating: None,
            stream_url: "http://example.com/m1".to_string(),
            category: None,
            added_at: 1,
            sort_order: 0,
        };
        let series_item = Series {
            id: "s1".to_string(),
            profile_id: "p1".to_string(),
            name: "Globo Series".to_string(),
            poster: None,
            backdrop: None,
            plot: None,
            genres: None,
            rating: None,
            category: None,
            added_at: 1,
            sort_order: 0,
        };

        channels::insert_batch(&conn, &[channel.clone()]).unwrap();
        movies::insert_batch(&conn, &[movie.clone()]).unwrap();
        series::insert_series_batch(&conn, &[series_item.clone()]).unwrap();
        catalog_fts::reindex_profile(&conn, "p1", &[channel], &[movie], &[series_item]).unwrap();
    }

    #[test]
    fn search_finds_channels_movies_and_series() {
        let conn = setup_db();
        seed_catalog(&conn);

        let results = search_catalog(&conn, "p1", "globo", 30).unwrap();
        assert_eq!(results.len(), 3);
        let types: Vec<&str> = results.iter().map(|r| r.item_type.as_str()).collect();
        assert!(types.contains(&"channel"));
        assert!(types.contains(&"movie"));
        assert!(types.contains(&"series"));
    }

    #[test]
    fn search_uses_fts_when_indexed() {
        let conn = setup_db();
        seed_catalog(&conn);

        let results = search_catalog(&conn, "p1", "globo news", 30).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item_type, "channel");
        assert_eq!(results[0].name, "Globo News");
    }

    #[test]
    fn search_falls_back_to_like_without_fts_index() {
        let conn = setup_db();
        profiles::insert_profile(
            &conn,
            &Profile {
                id: "p1".to_string(),
                name: "Test".to_string(),
                profile_type: "m3u".to_string(),
                url: None,
                file_path: None,
                username: None,
                password: None,
                last_sync: None,
            },
        )
        .unwrap();

        channels::insert_batch(
            &conn,
            &[Channel {
                id: "c1".to_string(),
                profile_id: "p1".to_string(),
                name: "Globo News".to_string(),
                logo: None,
                group_name: Some("Noticias".to_string()),
                stream_url: "http://example.com/globo".to_string(),
                tvg_id: None,
                sort_order: 0,
            }],
        )
        .unwrap();

        let results = search_catalog(&conn, "p1", "globo", 30).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item_type, "channel");
    }

    #[test]
    fn search_empty_query_returns_nothing() {
        let conn = setup_db();
        let results = search_catalog(&conn, "p1", "  ", 10).unwrap();
        assert!(results.is_empty());
    }
}
