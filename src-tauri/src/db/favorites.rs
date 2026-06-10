use crate::db::ids::stable_channel_id;
use crate::db::models::{Favorite, FavoriteItem};
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

fn now_unix_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn add_favorite(
    conn: &Connection,
    profile_id: &str,
    item_type: &str,
    item_id: &str,
) -> AppResult<Favorite> {
    if item_type != "movie" && item_type != "series" && item_type != "channel" {
        return Err(AppError::msg("Tipo de favorito inválido."));
    }

    let id = format!("{profile_id}:{item_type}:{item_id}");
    let created_at = now_unix_ts();
    conn.execute(
        "INSERT INTO favorites (id, profile_id, item_type, item_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(profile_id, item_type, item_id) DO UPDATE SET created_at = excluded.created_at",
        params![id, profile_id, item_type, item_id, created_at],
    )?;

    Ok(Favorite {
        id,
        profile_id: profile_id.to_string(),
        item_type: item_type.to_string(),
        item_id: item_id.to_string(),
        created_at,
    })
}

pub fn remove_favorite(
    conn: &Connection,
    profile_id: &str,
    item_type: &str,
    item_id: &str,
) -> AppResult<()> {
    conn.execute(
        "DELETE FROM favorites WHERE profile_id = ?1 AND item_type = ?2 AND item_id = ?3",
        params![profile_id, item_type, item_id],
    )?;
    Ok(())
}

pub fn is_favorite(
    conn: &Connection,
    profile_id: &str,
    item_type: &str,
    item_id: &str,
) -> AppResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM favorites
         WHERE profile_id = ?1 AND item_type = ?2 AND item_id = ?3",
        params![profile_id, item_type, item_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn map_favorite_item_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavoriteItem> {
    Ok(FavoriteItem {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        item_type: row.get(2)?,
        item_id: row.get(3)?,
        name: row.get(4)?,
        poster: row.get(5)?,
        category: row.get(6)?,
        created_at: row.get(7)?,
    })
}

pub fn list_favorites(conn: &Connection, profile_id: &str) -> AppResult<Vec<FavoriteItem>> {
    let sql = r#"
        SELECT f.id, f.profile_id, f.item_type, f.item_id,
               COALESCE(m.name, s.name, c.name, f.item_id) AS name,
               COALESCE(m.poster, s.poster, c.logo) AS poster,
               COALESCE(m.category, s.category, c.group_name) AS category,
               f.created_at
        FROM favorites f
        LEFT JOIN movies m ON f.item_type = 'movie' AND m.id = f.item_id AND m.profile_id = f.profile_id
        LEFT JOIN series s ON f.item_type = 'series' AND s.id = f.item_id AND s.profile_id = f.profile_id
        LEFT JOIN channels c ON f.item_type = 'channel' AND c.id = f.item_id AND c.profile_id = f.profile_id
        WHERE f.profile_id = ?1
        ORDER BY f.created_at DESC
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id], map_favorite_item_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

/// One-time remap after channel ids became stable (stream_url hash).
/// Matches legacy random UUID favorites to the new deterministic channel id.
pub fn remap_legacy_channel_favorites(conn: &Connection, profile_id: &str) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT id, stream_url FROM channels WHERE profile_id = ?1",
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (old_id, stream_url) = row?;
        let stable_id = stable_channel_id(profile_id, &stream_url);
        if old_id == stable_id {
            continue;
        }

        let new_favorite_id = format!("{profile_id}:channel:{stable_id}");
        conn.execute(
            "UPDATE favorites
             SET id = ?1, item_id = ?2
             WHERE profile_id = ?3 AND item_type = 'channel' AND item_id = ?4",
            params![new_favorite_id, stable_id, profile_id, old_id],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::channels;
    use crate::db::models::{Movie, Profile, Series, Channel};
    use crate::db::movies;
    use crate::db::profiles;
    use crate::db::series;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn insert_profile(conn: &Connection, id: &str) {
        profiles::insert_profile(
            conn,
            &Profile {
                id: id.to_string(),
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
    }

    #[test]
    fn add_list_and_remove_favorite() {
        let conn = setup_db();
        insert_profile(&conn, "p1");

        movies::insert_batch(
            &conn,
            &[Movie {
                id: "m1".to_string(),
                profile_id: "p1".to_string(),
                name: "Inception".to_string(),
                poster: Some("http://example.com/inception.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/m1".to_string(),
                category: Some("Acao".to_string()),
                added_at: 100,
                sort_order: 0,
            }],
        )
        .unwrap();

        let fav = add_favorite(&conn, "p1", "movie", "m1").unwrap();
        assert_eq!(fav.item_type, "movie");
        assert!(is_favorite(&conn, "p1", "movie", "m1").unwrap());

        let items = list_favorites(&conn, "p1").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Inception");
        assert_eq!(
            items[0].poster.as_deref(),
            Some("http://example.com/inception.png")
        );

        remove_favorite(&conn, "p1", "movie", "m1").unwrap();
        assert!(!is_favorite(&conn, "p1", "movie", "m1").unwrap());
        assert!(list_favorites(&conn, "p1").unwrap().is_empty());
    }

    #[test]
    fn favorites_unique_per_profile_type_and_item() {
        let conn = setup_db();
        insert_profile(&conn, "p1");

        series::insert_series_batch(
            &conn,
            &[Series {
                id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "Show".to_string(),
                poster: None,
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: None,
                added_at: 1,
                sort_order: 0,
            }],
        )
        .unwrap();

        add_favorite(&conn, "p1", "series", "s1").unwrap();
        add_favorite(&conn, "p1", "series", "s1").unwrap();

        assert_eq!(list_favorites(&conn, "p1").unwrap().len(), 1);
    }

    #[test]
    fn add_list_and_remove_channel_favorite() {
        let conn = setup_db();
        insert_profile(&conn, "p1");

        let stream_url = "http://example.com/globo.m3u8";
        let channel_id = stable_channel_id("p1", stream_url);

        channels::insert_batch(
            &conn,
            &[Channel {
                id: channel_id.clone(),
                profile_id: "p1".to_string(),
                name: "Globo SP".to_string(),
                logo: Some("http://example.com/globo.png".to_string()),
                group_name: Some("Abertos".to_string()),
                stream_url: stream_url.to_string(),
                tvg_id: None,
                sort_order: 0,
            }],
        )
        .unwrap();

        add_favorite(&conn, "p1", "channel", &channel_id).unwrap();
        assert!(is_favorite(&conn, "p1", "channel", &channel_id).unwrap());

        let items = list_favorites(&conn, "p1").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, "channel");
        assert_eq!(items[0].name, "Globo SP");
        assert_eq!(items[0].poster.as_deref(), Some("http://example.com/globo.png"));
        assert_eq!(items[0].category.as_deref(), Some("Abertos"));

        remove_favorite(&conn, "p1", "channel", &channel_id).unwrap();
        assert!(!is_favorite(&conn, "p1", "channel", &channel_id).unwrap());
    }

    #[test]
    fn remap_legacy_channel_favorites_updates_item_id() {
        let conn = setup_db();
        insert_profile(&conn, "p1");

        let legacy_id = "legacy-random-uuid";
        let stream_url = "http://example.com/live/u/p/99.ts";
        let stable_id = stable_channel_id("p1", stream_url);

        channels::insert_batch(
            &conn,
            &[Channel {
                id: legacy_id.to_string(),
                profile_id: "p1".to_string(),
                name: "Globo SP".to_string(),
                logo: None,
                group_name: Some("Abertos".to_string()),
                stream_url: stream_url.to_string(),
                tvg_id: None,
                sort_order: 0,
            }],
        )
        .unwrap();

        add_favorite(&conn, "p1", "channel", legacy_id).unwrap();
        remap_legacy_channel_favorites(&conn, "p1").unwrap();

        // After remap, persist_catalog re-inserts channels with stable ids.
        conn.execute(
            "UPDATE channels SET id = ?1 WHERE profile_id = ?2 AND id = ?3",
            params![stable_id, "p1", legacy_id],
        )
        .unwrap();

        assert!(is_favorite(&conn, "p1", "channel", &stable_id).unwrap());
        assert!(!is_favorite(&conn, "p1", "channel", legacy_id).unwrap());

        let items = list_favorites(&conn, "p1").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, stable_id);
        assert_eq!(items[0].name, "Globo SP");
    }
}
