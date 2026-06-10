use crate::db::models::{Channel, Movie, Series};
use crate::error::AppResult;
use rusqlite::{params, Connection};

pub fn fts_available(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='catalog_fts'",
        [],
        |row| row.get(0),
    )
    .unwrap_or(false)
}

pub fn delete_by_profile(conn: &Connection, profile_id: &str) -> AppResult<()> {
    if !fts_available(conn) {
        return Ok(());
    }
    conn.execute(
        "DELETE FROM catalog_fts WHERE profile_id = ?1",
        params![profile_id],
    )?;
    Ok(())
}

/// Bulk FTS rebuild from catalog tables (much faster than per-row Rust inserts).
pub fn reindex_profile_from_db(conn: &Connection, profile_id: &str) -> AppResult<()> {
    if !fts_available(conn) {
        return Ok(());
    }

    delete_by_profile(conn, profile_id)?;

    conn.execute(
        "INSERT INTO catalog_fts (item_id, profile_id, item_type, poster, name, category)
         SELECT id, profile_id, 'channel', logo, name, group_name
         FROM channels WHERE profile_id = ?1",
        params![profile_id],
    )?;
    conn.execute(
        "INSERT INTO catalog_fts (item_id, profile_id, item_type, poster, name, category)
         SELECT id, profile_id, 'movie', poster, name, category
         FROM movies WHERE profile_id = ?1",
        params![profile_id],
    )?;
    conn.execute(
        "INSERT INTO catalog_fts (item_id, profile_id, item_type, poster, name, category)
         SELECT id, profile_id, 'series', poster, name, category
         FROM series WHERE profile_id = ?1",
        params![profile_id],
    )?;
    Ok(())
}

pub fn reindex_profile(
    conn: &Connection,
    profile_id: &str,
    channels: &[Channel],
    movies: &[Movie],
    series: &[Series],
) -> AppResult<()> {
    if !fts_available(conn) {
        return Ok(());
    }

    delete_by_profile(conn, profile_id)?;

    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO catalog_fts (item_id, profile_id, item_type, poster, name, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;

        for channel in channels {
            stmt.execute(params![
                channel.id,
                profile_id,
                "channel",
                channel.logo,
                channel.name,
                channel.group_name,
            ])?;
        }
        for movie in movies {
            stmt.execute(params![
                movie.id,
                profile_id,
                "movie",
                movie.poster,
                movie.name,
                movie.category,
            ])?;
        }
        for item in series {
            stmt.execute(params![
                item.id,
                profile_id,
                "series",
                item.poster,
                item.name,
                item.category,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn profile_has_entries(conn: &Connection, profile_id: &str) -> AppResult<bool> {
    if !fts_available(conn) {
        return Ok(false);
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM catalog_fts WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
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

    #[test]
    fn reindex_and_query_profile_entries() {
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

        reindex_profile(
            &conn,
            "p1",
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
            &[],
            &[],
        )
        .unwrap();

        assert!(profile_has_entries(&conn, "p1").unwrap());
        delete_by_profile(&conn, "p1").unwrap();
        assert!(!profile_has_entries(&conn, "p1").unwrap());
    }
}
