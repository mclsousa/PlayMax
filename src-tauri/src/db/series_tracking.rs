use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection};

use crate::db::models::Episode;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpisodeSnapshot {
    pub episode_count: i64,
    pub max_season: i32,
    pub max_episode: i32,
}

pub fn snapshot_episodes(
    conn: &Connection,
    profile_id: &str,
) -> AppResult<HashMap<String, EpisodeSnapshot>> {
    let mut stmt = conn.prepare(
        "SELECT e.series_id, COUNT(*), MAX(e.season), MAX(e.episode)
         FROM episodes e
         INNER JOIN series s ON s.id = e.series_id
         WHERE s.profile_id = ?1
         GROUP BY e.series_id",
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            EpisodeSnapshot {
                episode_count: row.get(1)?,
                max_season: row.get(2)?,
                max_episode: row.get(3)?,
            },
        ))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let (series_id, snapshot) = row.map_err(AppError::Database)?;
        map.insert(series_id, snapshot);
    }
    Ok(map)
}

pub fn snapshot_series_ids(conn: &Connection, profile_id: &str) -> AppResult<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT id FROM series WHERE profile_id = ?1")?;
    let rows = stmt.query_map(params![profile_id], |row| row.get::<_, String>(0))?;

    let mut ids = HashSet::new();
    for row in rows {
        ids.insert(row.map_err(AppError::Database)?);
    }
    Ok(ids)
}

fn episode_stats(episodes: &[Episode], series_id: &str) -> Option<EpisodeSnapshot> {
    let mut count = 0_i64;
    let mut max_season = 0_i32;
    let mut max_episode = 0_i32;

    for episode in episodes.iter().filter(|ep| ep.series_id == series_id) {
        count += 1;
        if episode.season > max_season {
            max_season = episode.season;
            max_episode = episode.episode;
        } else if episode.season == max_season && episode.episode > max_episode {
            max_episode = episode.episode;
        }
    }

    if count == 0 {
        None
    } else {
        Some(EpisodeSnapshot {
            episode_count: count,
            max_season,
            max_episode,
        })
    }
}

fn has_new_episodes(previous: EpisodeSnapshot, current: EpisodeSnapshot) -> bool {
    if current.episode_count > previous.episode_count {
        return true;
    }
    current.max_season > previous.max_season
        || (current.max_season == previous.max_season && current.max_episode > previous.max_episode)
}

fn load_last_updates(conn: &Connection, profile_id: &str) -> AppResult<HashMap<String, i64>> {
    let mut stmt = conn.prepare(
        "SELECT series_id, last_episode_update
         FROM series_episode_tracking
         WHERE profile_id = ?1",
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let (series_id, updated_at) = row.map_err(AppError::Database)?;
        map.insert(series_id, updated_at);
    }
    Ok(map)
}

fn load_first_seen(conn: &Connection, profile_id: &str) -> AppResult<HashMap<String, i64>> {
    let mut stmt = conn.prepare(
        "SELECT series_id, first_seen_at
         FROM series_episode_tracking
         WHERE profile_id = ?1",
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let (series_id, first_seen) = row.map_err(AppError::Database)?;
        map.insert(series_id, first_seen);
    }
    Ok(map)
}

/// Wraps `f` in a transaction unless the connection is already inside one
/// (SQLite rejects nested BEGINs, e.g. when called from the sync transaction).
fn run_transactional<F>(conn: &Connection, f: F) -> AppResult<()>
where
    F: FnOnce(&Connection) -> AppResult<()>,
{
    if conn.is_autocommit() {
        let tx = conn.unchecked_transaction()?;
        f(&tx)?;
        tx.commit()?;
        Ok(())
    } else {
        f(conn)
    }
}

fn resolve_first_seen_at(
    series_id: &str,
    previous_series_ids: &HashSet<String>,
    prior_first_seen: &HashMap<String, i64>,
    now: i64,
) -> i64 {
    if let Some(&seen) = prior_first_seen.get(series_id) {
        if seen > 0 {
            return seen;
        }
    }

    if previous_series_ids.contains(series_id) {
        0
    } else {
        now
    }
}

pub fn sync_tracking_for_catalog(
    conn: &Connection,
    profile_id: &str,
    series_ids: &[String],
    episodes: &[Episode],
    previous_snapshots: &HashMap<String, EpisodeSnapshot>,
    previous_series_ids: &HashSet<String>,
    now: i64,
) -> AppResult<()> {
    let prior_updates = load_last_updates(conn, profile_id)?;
    let prior_first_seen = load_first_seen(conn, profile_id)?;

    run_transactional(conn, |tx| {
        let mut upsert = tx.prepare(
            "INSERT INTO series_episode_tracking
             (series_id, profile_id, episode_count, max_season, max_episode,
              last_episode_update, first_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(series_id) DO UPDATE SET
               profile_id = excluded.profile_id,
               episode_count = excluded.episode_count,
               max_season = excluded.max_season,
               max_episode = excluded.max_episode,
               last_episode_update = excluded.last_episode_update,
               first_seen_at = CASE
                 WHEN series_episode_tracking.first_seen_at > 0
                   THEN series_episode_tracking.first_seen_at
                 ELSE excluded.first_seen_at
               END",
        )?;

        for series_id in series_ids {
            let Some(current) = episode_stats(episodes, series_id) else {
                continue;
            };

            let first_seen_at = resolve_first_seen_at(
                series_id,
                previous_series_ids,
                &prior_first_seen,
                now,
            );

            // Only mark as "updated" when the series already existed and gained episodes.
            let last_update = if let Some(previous) = previous_snapshots.get(series_id) {
                if has_new_episodes(*previous, current) {
                    now
                } else {
                    prior_updates.get(series_id).copied().unwrap_or(0)
                }
            } else {
                0
            };

            upsert.execute(params![
                series_id,
                profile_id,
                current.episode_count,
                current.max_season,
                current.max_episode,
                last_update,
                first_seen_at,
            ])?;
        }
        drop(upsert);

        tx.execute(
            "DELETE FROM series_episode_tracking
             WHERE profile_id = ?1
               AND series_id NOT IN (
                 SELECT id FROM series WHERE profile_id = ?1
               )",
            params![profile_id],
        )?;
        Ok(())
    })
}

const VALID_UNIX_TS: i64 = 946_684_800;

pub fn backfill_from_existing_catalog(conn: &Connection) -> AppResult<()> {
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM series_episode_tracking",
        [],
        |row| row.get(0),
    )?;
    if existing > 0 {
        return Ok(());
    }

    let mut stmt = conn.prepare(
        "SELECT s.id, s.profile_id, COUNT(e.id), MAX(e.season), MAX(e.episode), MAX(e.added_at)
         FROM series s
         INNER JOIN episodes e ON e.series_id = s.id
         GROUP BY s.id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i32>(3)?,
            row.get::<_, i32>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;

    let tx = conn.unchecked_transaction()?;
    {
        let mut insert = tx.prepare(
            "INSERT INTO series_episode_tracking
             (series_id, profile_id, episode_count, max_season, max_episode,
              last_episode_update, first_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0)",
        )?;

        for row in rows {
            let (series_id, profile_id, count, max_season, max_episode, _) =
                row.map_err(AppError::Database)?;
            insert.execute(params![
                series_id,
                profile_id,
                count,
                max_season,
                max_episode,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn backfill_first_seen_from_series(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT t.series_id, s.added_at
         FROM series_episode_tracking t
         INNER JOIN series s ON s.id = t.series_id
         WHERE t.first_seen_at = 0",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let tx = conn.unchecked_transaction()?;
    {
        let mut update = tx.prepare(
            "UPDATE series_episode_tracking
             SET first_seen_at = ?1
             WHERE series_id = ?2 AND first_seen_at = 0",
        )?;

        for row in rows {
            let (series_id, added_at) = row.map_err(AppError::Database)?;
            if added_at >= VALID_UNIX_TS {
                update.execute(params![added_at, series_id])?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

/// Ensures tracking rows exist for a profile (episode stats only, no fake timestamps).
pub fn ensure_profile_tracking(conn: &Connection, profile_id: &str) -> AppResult<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM series_episode_tracking WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    if count > 0 {
        return Ok(());
    }

    let mut stmt = conn.prepare(
        "SELECT s.id, s.profile_id, COUNT(e.id), MAX(e.season), MAX(e.episode)
         FROM series s
         INNER JOIN episodes e ON e.series_id = s.id
         WHERE s.profile_id = ?1
         GROUP BY s.id",
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i32>(3)?,
            row.get::<_, i32>(4)?,
        ))
    })?;

    run_transactional(conn, |tx| {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO series_episode_tracking
             (series_id, profile_id, episode_count, max_season, max_episode,
              last_episode_update, first_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0)",
        )?;

        for row in rows {
            let (series_id, pid, count, max_season, max_episode) =
                row.map_err(AppError::Database)?;
            insert.execute(params![
                series_id,
                pid,
                count,
                max_season,
                max_episode,
            ])?;
        }
        Ok(())
    })
}
