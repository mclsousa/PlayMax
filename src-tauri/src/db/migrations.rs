use crate::error::AppResult;
use rusqlite::Connection;

const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK(type IN ('m3u')),
  url TEXT,
  file_path TEXT,
  last_sync INTEGER
);

CREATE TABLE IF NOT EXISTS channels (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  logo TEXT,
  group_name TEXT,
  stream_url TEXT NOT NULL,
  tvg_id TEXT,
  sort_order INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_channels_profile ON channels(profile_id);
CREATE INDEX IF NOT EXISTS idx_channels_group ON channels(profile_id, group_name);
"#;

const MIGRATION_002: &str = r#"
CREATE TABLE IF NOT EXISTS profiles_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK(type IN ('m3u', 'xtream')),
  url TEXT,
  file_path TEXT,
  username TEXT,
  password TEXT,
  last_sync INTEGER
);
INSERT INTO profiles_new (id, name, type, url, file_path, username, password, last_sync)
  SELECT id, name, type, url, file_path, NULL, NULL, last_sync FROM profiles;
DROP TABLE profiles;
ALTER TABLE profiles_new RENAME TO profiles;

CREATE TABLE IF NOT EXISTS movies (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  poster TEXT,
  backdrop TEXT,
  plot TEXT,
  genres TEXT,
  rating TEXT,
  stream_url TEXT NOT NULL,
  category TEXT,
  added_at INTEGER NOT NULL,
  sort_order INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS series (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  poster TEXT,
  backdrop TEXT,
  plot TEXT,
  genres TEXT,
  rating TEXT,
  category TEXT,
  added_at INTEGER NOT NULL,
  sort_order INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS episodes (
  id TEXT PRIMARY KEY,
  series_id TEXT NOT NULL REFERENCES series(id) ON DELETE CASCADE,
  season INTEGER NOT NULL,
  episode INTEGER NOT NULL,
  title TEXT NOT NULL,
  plot TEXT,
  stream_url TEXT NOT NULL,
  duration INTEGER,
  added_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_movies_profile_added ON movies(profile_id, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_series_profile_added ON series(profile_id, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_episodes_series ON episodes(series_id, season, episode);
"#;

const MIGRATION_003: &str = r#"
CREATE TABLE IF NOT EXISTS history (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK(item_type IN ('movie', 'episode')),
  item_id TEXT NOT NULL,
  name TEXT NOT NULL,
  poster TEXT,
  stream_url TEXT NOT NULL,
  position REAL NOT NULL DEFAULT 0,
  duration REAL NOT NULL DEFAULT 0,
  watched_at INTEGER NOT NULL,
  UNIQUE(profile_id, item_id)
);

CREATE INDEX IF NOT EXISTS idx_history_profile_watched ON history(profile_id, watched_at DESC);
"#;

const MIGRATION_004: &str = r#"
CREATE TABLE IF NOT EXISTS history_new (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK(item_type IN ('movie', 'episode', 'live')),
  item_id TEXT NOT NULL,
  name TEXT NOT NULL,
  poster TEXT,
  stream_url TEXT NOT NULL,
  position REAL NOT NULL DEFAULT 0,
  duration REAL NOT NULL DEFAULT 0,
  watched_at INTEGER NOT NULL,
  UNIQUE(profile_id, item_id)
);

INSERT INTO history_new (id, profile_id, item_type, item_id, name, poster, stream_url, position, duration, watched_at)
  SELECT id, profile_id, item_type, item_id, name, poster, stream_url, position, duration, watched_at
  FROM history;

DROP TABLE history;
ALTER TABLE history_new RENAME TO history;

CREATE INDEX IF NOT EXISTS idx_history_profile_watched ON history(profile_id, watched_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_profile_live ON history(profile_id, item_type, watched_at DESC);
"#;

const MIGRATION_005: &str = r#"
CREATE TABLE IF NOT EXISTS favorites (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK(item_type IN ('movie', 'series', 'channel')),
  item_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(profile_id, item_type, item_id)
);

CREATE INDEX IF NOT EXISTS idx_favorites_profile ON favorites(profile_id, created_at DESC);
"#;

const MIGRATION_006: &str = r#"
CREATE TABLE IF NOT EXISTS epg_programs (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  channel_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  start_ts INTEGER NOT NULL,
  end_ts INTEGER NOT NULL,
  epg_id TEXT,
  fetched_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_epg_programs_channel ON epg_programs(profile_id, channel_id, start_ts);
CREATE INDEX IF NOT EXISTS idx_epg_programs_fetched ON epg_programs(profile_id, channel_id, fetched_at DESC);
"#;

const MIGRATION_007_SETTINGS: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
"#;

const MIGRATION_007_FTS: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS catalog_fts USING fts5(
  item_id UNINDEXED,
  profile_id UNINDEXED,
  item_type UNINDEXED,
  poster UNINDEXED,
  name,
  category,
  tokenize='unicode61 remove_diacritics 2'
);
"#;

const MIGRATION_007_FTS_FALLBACK: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS catalog_fts USING fts5(
  item_id UNINDEXED,
  profile_id UNINDEXED,
  item_type UNINDEXED,
  poster UNINDEXED,
  name,
  category,
  tokenize='unicode61'
);
"#;

fn migration_002_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='movies'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn migration_003_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='history'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn migration_004_applied(conn: &Connection) -> AppResult<bool> {
    let check: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='history'",
            [],
            |row| row.get(0),
        )
        .ok();
    Ok(check
        .map(|sql| sql.contains("'live'"))
        .unwrap_or(false))
}

fn migration_005_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='favorites'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn migration_006_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='epg_programs'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn migration_007_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='catalog_fts'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn apply_migration_007(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(MIGRATION_007_SETTINGS)?;
    if migration_007_applied(conn)? {
        return Ok(());
    }

    if conn.execute_batch(MIGRATION_007_FTS).is_err() {
        eprintln!(
            "[migration] FTS5 remove_diacritics unavailable; falling back to unicode61 tokenizer"
        );
        conn.execute_batch(MIGRATION_007_FTS_FALLBACK)?;
    }
    Ok(())
}

const MIGRATION_008: &str = r#"
CREATE INDEX IF NOT EXISTS idx_movies_profile_category_added
  ON movies(profile_id, category, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_series_profile_category_added
  ON series(profile_id, category, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_channels_profile_sort
  ON channels(profile_id, sort_order, name);
"#;

fn migration_008_applied(conn: &Connection) -> AppResult<bool> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_movies_profile_category_added'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists > 0)
}

const MIGRATION_009: &str = r#"
CREATE TABLE IF NOT EXISTS series_episode_tracking (
  series_id TEXT PRIMARY KEY REFERENCES series(id) ON DELETE CASCADE,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  episode_count INTEGER NOT NULL DEFAULT 0,
  max_season INTEGER NOT NULL DEFAULT 0,
  max_episode INTEGER NOT NULL DEFAULT 0,
  last_episode_update INTEGER NOT NULL DEFAULT 0,
  first_seen_at INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_series_episode_tracking_profile_updated
  ON series_episode_tracking(profile_id, last_episode_update DESC);
"#;

fn migration_009_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='series_episode_tracking'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn migration_010_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM app_settings WHERE key = 'migration_010_series_tracking_repair'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

const MIGRATION_011: &str = r#"
ALTER TABLE series_episode_tracking ADD COLUMN first_seen_at INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_series_episode_tracking_profile_first_seen
  ON series_episode_tracking(profile_id, first_seen_at DESC);
"#;

fn migration_011_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM app_settings WHERE key = 'migration_011_series_first_seen'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Indexes that let list/count queries page valid content in pure SQL.
const MIGRATION_012_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_movies_valid_added
  ON movies(profile_id, is_valid_vod, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_movies_valid_category
  ON movies(profile_id, is_valid_vod, category, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_series_valid_added
  ON series(profile_id, is_valid_vod, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_series_valid_category
  ON series(profile_id, is_valid_vod, category, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_channels_live_group
  ON channels(profile_id, is_live, group_name);
CREATE INDEX IF NOT EXISTS idx_channels_live_sort
  ON channels(profile_id, is_live, sort_order, name);
"#;

fn migration_012_applied(conn: &Connection) -> AppResult<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM app_settings WHERE key = 'migration_012_content_classification'",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    let count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?1"),
        [column],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn backfill_movie_classification(conn: &Connection) -> AppResult<()> {
    use crate::db::catalog_filters::is_valid_vod_movie;

    let mut stmt = conn.prepare("SELECT id, name FROM movies")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let invalid_ids: Vec<String> = rows
        .into_iter()
        .filter(|(_, name)| !is_valid_vod_movie(name))
        .map(|(id, _)| id)
        .collect();

    let mut update = conn.prepare("UPDATE movies SET is_valid_vod = 0 WHERE id = ?1")?;
    for id in invalid_ids {
        update.execute([id])?;
    }

    let mut profiles_stmt = conn.prepare("SELECT DISTINCT profile_id FROM movies")?;
    let profile_ids = profiles_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for profile_id in profile_ids {
        crate::db::movies::refine_xtream_vod_classification(conn, &profile_id)?;
    }
    Ok(())
}

fn backfill_series_classification(conn: &Connection) -> AppResult<()> {
    use crate::db::catalog_filters::is_valid_vod_series;

    let mut stmt = conn.prepare("SELECT id, name, category FROM series")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let invalid_ids: Vec<String> = rows
        .into_iter()
        .filter(|(_, name, category)| !is_valid_vod_series(name, category.as_deref()))
        .map(|(id, _, _)| id)
        .collect();

    let mut update = conn.prepare("UPDATE series SET is_valid_vod = 0 WHERE id = ?1")?;
    for id in invalid_ids {
        update.execute([id])?;
    }

    let mut profiles_stmt = conn.prepare("SELECT DISTINCT profile_id FROM series")?;
    let profile_ids = profiles_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for profile_id in profile_ids {
        crate::db::series::refine_series_listability(conn, &profile_id)?;
    }
    Ok(())
}

fn apply_migration_012(conn: &Connection) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    if !table_has_column(&tx, "movies", "is_valid_vod")? {
        tx.execute_batch("ALTER TABLE movies ADD COLUMN is_valid_vod INTEGER NOT NULL DEFAULT 1;")?;
    }
    if !table_has_column(&tx, "series", "is_valid_vod")? {
        tx.execute_batch("ALTER TABLE series ADD COLUMN is_valid_vod INTEGER NOT NULL DEFAULT 1;")?;
    }
    if !table_has_column(&tx, "channels", "is_live")? {
        tx.execute_batch("ALTER TABLE channels ADD COLUMN is_live INTEGER NOT NULL DEFAULT 1;")?;
    }

    tx.execute(
        "UPDATE channels SET group_name = NULLIF(TRIM(group_name), '') WHERE group_name IS NOT NULL",
        [],
    )?;
    tx.execute(
        "UPDATE channels SET is_live = CASE
            WHEN lower(stream_url) LIKE '%/movie/%' OR lower(stream_url) LIKE '%/series/%' THEN 0
            ELSE 1
         END",
        [],
    )?;

    backfill_movie_classification(&tx)?;
    backfill_series_classification(&tx)?;

    tx.execute_batch(MIGRATION_012_INDEXES)?;
    tx.execute(
        "INSERT INTO app_settings (key, value) VALUES ('migration_012_content_classification', '1')",
        [],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(MIGRATION_001)?;
    if !migration_002_applied(conn)? {
        conn.execute_batch(MIGRATION_002)?;
    }
    if !migration_003_applied(conn)? {
        conn.execute_batch(MIGRATION_003)?;
    }
    if migration_003_applied(conn)? && !migration_004_applied(conn)? {
        conn.execute_batch(MIGRATION_004)?;
    }
    if !migration_005_applied(conn)? {
        conn.execute_batch(MIGRATION_005)?;
    }
    if !migration_006_applied(conn)? {
        conn.execute_batch(MIGRATION_006)?;
    }
    if !migration_007_applied(conn)? {
        apply_migration_007(conn)?;
    }
    if !migration_008_applied(conn)? {
        conn.execute_batch(MIGRATION_008)?;
    }
    if !migration_009_applied(conn)? {
        conn.execute_batch(MIGRATION_009)?;
        crate::db::series_tracking::backfill_from_existing_catalog(conn)?;
    }
    if !migration_010_applied(conn)? {
        let tracking_rows: i64 = conn.query_row(
            "SELECT COUNT(*) FROM series_episode_tracking",
            [],
            |row| row.get(0),
        )?;
        if tracking_rows == 0 {
            crate::db::series_tracking::backfill_from_existing_catalog(conn)?;
        }
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('migration_010_series_tracking_repair', '1')",
            [],
        )?;
    }
    if !migration_011_applied(conn)? {
        let has_first_seen: bool = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('series_episode_tracking') WHERE name = 'first_seen_at'",
            [],
            |row| {
                let count: i64 = row.get(0)?;
                Ok(count > 0)
            },
        )?;
        if !has_first_seen {
            conn.execute_batch(MIGRATION_011)?;
        } else {
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_series_episode_tracking_profile_first_seen
                   ON series_episode_tracking(profile_id, first_seen_at DESC)",
                [],
            )?;
        }
        conn.execute(
            "UPDATE series_episode_tracking SET last_episode_update = 0",
            [],
        )?;
        crate::db::series_tracking::backfill_first_seen_from_series(conn)?;
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('migration_011_series_first_seen', '1')",
            [],
        )?;
    }
    if !migration_012_applied(conn)? {
        apply_migration_012(conn)?;
    }
    Ok(())
}

#[cfg(test)]
mod real_db_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn migrations_leave_connection_in_autocommit() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        assert!(conn.is_autocommit(), "migrations must not leave an open transaction");
    }

    #[test]
    fn migrations_succeed_on_existing_app_db() {
        let path = PathBuf::from(std::env::var("USERPROFILE").unwrap())
            .join(r"AppData\Roaming\com.playmax.app\playmax.db");
        if !path.exists() {
            return;
        }
        let conn = Connection::open(path).expect("open existing app db");
        run(&conn).expect("migrations on existing app db");

        let (movies, valid_movies, series, valid_series, channels, live): (
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM movies),
                        (SELECT COUNT(*) FROM movies WHERE is_valid_vod = 1),
                        (SELECT COUNT(*) FROM series),
                        (SELECT COUNT(*) FROM series WHERE is_valid_vod = 1),
                        (SELECT COUNT(*) FROM channels),
                        (SELECT COUNT(*) FROM channels WHERE is_live = 1)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("classification counts");
        eprintln!(
            "[real-db] movies {valid_movies}/{movies} valid, series {valid_series}/{series} valid, channels {live}/{channels} live"
        );
    }
}
