use crate::db::models::HistoryEntry;
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

fn history_id(profile_id: &str, item_id: &str) -> String {
    format!("{profile_id}:{item_id}")
}

fn map_history_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        item_type: row.get(2)?,
        item_id: row.get(3)?,
        name: row.get(4)?,
        poster: row.get(5)?,
        stream_url: row.get(6)?,
        position: row.get(7)?,
        duration: row.get(8)?,
        watched_at: row.get(9)?,
    })
}

pub fn upsert(
    conn: &Connection,
    profile_id: &str,
    item_type: &str,
    item_id: &str,
    name: &str,
    poster: Option<&str>,
    stream_url: &str,
    position: f64,
    duration: f64,
    watched_at: i64,
) -> AppResult<()> {
    let id = history_id(profile_id, item_id);
    conn.execute(
        "INSERT INTO history (id, profile_id, item_type, item_id, name, poster, stream_url, position, duration, watched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(profile_id, item_id) DO UPDATE SET
           name = excluded.name,
           poster = excluded.poster,
           stream_url = excluded.stream_url,
           position = excluded.position,
           duration = CASE WHEN excluded.duration > 0 THEN excluded.duration ELSE history.duration END,
           watched_at = excluded.watched_at",
        params![
            id,
            profile_id,
            item_type,
            item_id,
            name,
            poster,
            stream_url,
            position,
            duration,
            watched_at,
        ],
    )?;
    Ok(())
}

const MAX_LIVE_HISTORY: i64 = 20;

pub fn list_continue_watching(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
) -> AppResult<Vec<HistoryEntry>> {
    let sql = r#"
        SELECT id, profile_id, item_type, item_id, name, poster, stream_url, position, duration, watched_at
        FROM history
        WHERE profile_id = ?1
          AND item_type IN ('movie', 'episode')
          AND (duration <= 0 OR position < duration * 0.9)
        ORDER BY watched_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, limit], map_history_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(e))
}

pub fn list_recent_channels(
    conn: &Connection,
    profile_id: &str,
    limit: i64,
) -> AppResult<Vec<HistoryEntry>> {
    let sql = r#"
        SELECT id, profile_id, item_type, item_id, name, poster, stream_url, position, duration, watched_at
        FROM history
        WHERE profile_id = ?1
          AND item_type = 'live'
        ORDER BY watched_at DESC
        LIMIT ?2
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![profile_id, limit], map_history_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(e))
}

fn prune_old_live_entries(conn: &Connection, profile_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM history
         WHERE profile_id = ?1
           AND item_type = 'live'
           AND id NOT IN (
             SELECT id FROM history
             WHERE profile_id = ?1 AND item_type = 'live'
             ORDER BY watched_at DESC
             LIMIT ?2
           )",
        params![profile_id, MAX_LIVE_HISTORY],
    )?;
    Ok(())
}

/// One-time remap after channel ids became stable (stream_url hash).
pub fn remap_legacy_channel_history(conn: &Connection, profile_id: &str) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT id, stream_url FROM channels WHERE profile_id = ?1",
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (old_id, stream_url) = row?;
        let stable_id = crate::db::ids::stable_channel_id(profile_id, &stream_url);
        if old_id == stable_id {
            continue;
        }

        let new_history_id = history_id(profile_id, &stable_id);
        conn.execute(
            "UPDATE history
             SET id = ?1, item_id = ?2
             WHERE profile_id = ?3 AND item_type = 'live' AND item_id = ?4",
            params![new_history_id, stable_id, profile_id, old_id],
        )?;
    }

    Ok(())
}

pub fn upsert_live(
    conn: &Connection,
    profile_id: &str,
    channel_id: &str,
    name: &str,
    logo: Option<&str>,
    stream_url: &str,
    watched_at: i64,
) -> AppResult<()> {
    upsert(
        conn,
        profile_id,
        "live",
        channel_id,
        name,
        logo,
        stream_url,
        0.0,
        0.0,
        watched_at,
    )?;
    prune_old_live_entries(conn, profile_id)
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

    #[test]
    fn upsert_creates_and_updates_entry() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        upsert(
            &conn,
            "p1",
            "movie",
            "m1",
            "Movie One",
            Some("http://poster/1.jpg"),
            "http://stream/1",
            100.0,
            1000.0,
            1000,
        )
        .unwrap();

        upsert(
            &conn,
            "p1",
            "movie",
            "m1",
            "Movie One Updated",
            None,
            "http://stream/1",
            500.0,
            1000.0,
            2000,
        )
        .unwrap();

        let items = list_continue_watching(&conn, "p1", 10).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Movie One Updated");
        assert!((items[0].position - 500.0).abs() < f64::EPSILON);
        assert_eq!(items[0].watched_at, 2000);
    }

    #[test]
    fn list_continue_watching_excludes_near_complete() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        upsert(
            &conn,
            "p1",
            "movie",
            "m-done",
            "Finished",
            None,
            "http://stream/done",
            950.0,
            1000.0,
            3000,
        )
        .unwrap();

        upsert(
            &conn,
            "p1",
            "episode",
            "e1",
            "Episode",
            None,
            "http://stream/ep",
            400.0,
            1000.0,
            2000,
        )
        .unwrap();

        let items = list_continue_watching(&conn, "p1", 10).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, "e1");
    }

    #[test]
    fn list_continue_watching_orders_by_watched_at_desc() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        upsert(
            &conn,
            "p1",
            "movie",
            "m-old",
            "Old",
            None,
            "http://stream/old",
            100.0,
            1000.0,
            1000,
        )
        .unwrap();

        upsert(
            &conn,
            "p1",
            "movie",
            "m-new",
            "New",
            None,
            "http://stream/new",
            200.0,
            1000.0,
            3000,
        )
        .unwrap();

        let items = list_continue_watching(&conn, "p1", 10).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].item_id, "m-new");
        assert_eq!(items[1].item_id, "m-old");
    }

    #[test]
    fn upsert_live_and_list_recent_channels() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        upsert_live(
            &conn,
            "p1",
            "ch-1",
            "Globo",
            Some("http://logo/globo.png"),
            "http://stream/globo",
            1000,
        )
        .unwrap();

        upsert_live(
            &conn,
            "p1",
            "ch-2",
            "SBT",
            None,
            "http://stream/sbt",
            2000,
        )
        .unwrap();

        upsert_live(
            &conn,
            "p1",
            "ch-1",
            "Globo",
            Some("http://logo/globo.png"),
            "http://stream/globo",
            3000,
        )
        .unwrap();

        let channels = list_recent_channels(&conn, "p1", 10).unwrap();
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].item_id, "ch-1");
        assert_eq!(channels[0].watched_at, 3000);
        assert_eq!(channels[1].item_id, "ch-2");
    }

    #[test]
    fn remap_legacy_channel_history_updates_item_id() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        let legacy_id = "legacy-random-uuid";
        let stream_url = "http://example.com/live/u/p/99.ts";
        let stable_id = crate::db::ids::stable_channel_id("p1", stream_url);

        crate::db::channels::insert_batch(
            &conn,
            &[crate::db::models::Channel {
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

        upsert_live(
            &conn,
            "p1",
            legacy_id,
            "Globo SP",
            None,
            stream_url,
            1000,
        )
        .unwrap();

        remap_legacy_channel_history(&conn, "p1").unwrap();

        let channels = list_recent_channels(&conn, "p1", 10).unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].item_id, stable_id);
        assert_eq!(channels[0].id, history_id("p1", &stable_id));
    }

    #[test]
    fn list_continue_watching_excludes_live() {
        let conn = setup_db();
        insert_test_profile(&conn, "p1");

        upsert_live(
            &conn,
            "p1",
            "ch-1",
            "Globo",
            None,
            "http://stream/globo",
            5000,
        )
        .unwrap();

        upsert(
            &conn,
            "p1",
            "movie",
            "m1",
            "Movie",
            None,
            "http://stream/m1",
            100.0,
            1000.0,
            4000,
        )
        .unwrap();

        let items = list_continue_watching(&conn, "p1", 10).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, "movie");
    }
}
