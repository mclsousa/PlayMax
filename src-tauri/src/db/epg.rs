use crate::db::models::EpgProgram;
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};
use uuid::Uuid;

pub const EPG_CACHE_TTL_SECS: i64 = 2 * 60 * 60;

pub fn cache_fresh(conn: &Connection, profile_id: &str, channel_id: &str, now: i64) -> AppResult<bool> {
    let fetched_at: Option<i64> = conn
        .query_row(
            "SELECT MAX(fetched_at) FROM epg_programs WHERE profile_id = ?1 AND channel_id = ?2",
            params![profile_id, channel_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();
    Ok(fetched_at
        .map(|ts| now.saturating_sub(ts) < EPG_CACHE_TTL_SECS)
        .unwrap_or(false))
}

pub fn list_cached_programs(
    conn: &Connection,
    profile_id: &str,
    channel_id: &str,
    limit: i64,
    now: i64,
) -> AppResult<Vec<EpgProgram>> {
    let mut stmt = conn.prepare(
        "SELECT title, description, start_ts, end_ts, epg_id
         FROM epg_programs
         WHERE profile_id = ?1 AND channel_id = ?2
         ORDER BY start_ts ASC
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![profile_id, channel_id, limit], |row| {
        let start_ts: i64 = row.get(2)?;
        let end_ts: i64 = row.get(3)?;
        Ok(EpgProgram {
            title: row.get(0)?,
            description: row.get(1)?,
            start_ts,
            end_ts,
            epg_id: row.get(4)?,
            is_now: now >= start_ts && now < end_ts,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn replace_channel_programs(
    conn: &Connection,
    profile_id: &str,
    channel_id: &str,
    programs: &[EpgProgram],
    fetched_at: i64,
) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM epg_programs WHERE profile_id = ?1 AND channel_id = ?2",
        params![profile_id, channel_id],
    )?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO epg_programs
             (id, profile_id, channel_id, title, description, start_ts, end_ts, epg_id, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for program in programs {
            stmt.execute(params![
                Uuid::new_v4().to_string(),
                profile_id,
                channel_id,
                program.title,
                program.description,
                program.start_ts,
                program.end_ts,
                program.epg_id,
                fetched_at,
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

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let profile = Profile {
            id: "p1".to_string(),
            name: "Test".to_string(),
            profile_type: "xtream".to_string(),
            url: Some("http://example.com".to_string()),
            file_path: None,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            last_sync: None,
        };
        profiles::insert_profile(&conn, &profile).unwrap();
        conn
    }

    #[test]
    fn cache_fresh_respects_ttl() {
        let conn = setup();
        let now = 1_700_000_000_i64;
        replace_channel_programs(
            &conn,
            "p1",
            "ch1",
            &[EpgProgram {
                title: "News".to_string(),
                description: None,
                start_ts: now,
                end_ts: now + 3600,
                epg_id: Some("1".to_string()),
                is_now: true,
            }],
            now,
        )
        .unwrap();
        assert!(cache_fresh(&conn, "p1", "ch1", now + 100).unwrap());
        assert!(cache_fresh(&conn, "p1", "ch1", now + EPG_CACHE_TTL_SECS - 1).unwrap());
        assert!(!cache_fresh(&conn, "p1", "ch1", now + EPG_CACHE_TTL_SECS).unwrap());
    }

    #[test]
    fn replace_and_list_programs() {
        let conn = setup();
        let now = 1_700_000_000_i64;
        replace_channel_programs(
            &conn,
            "p1",
            "ch1",
            &[
                EpgProgram {
                    title: "Morning".to_string(),
                    description: Some("Desc".to_string()),
                    start_ts: now,
                    end_ts: now + 1800,
                    epg_id: Some("a".to_string()),
                    is_now: true,
                },
                EpgProgram {
                    title: "Afternoon".to_string(),
                    description: None,
                    start_ts: now + 1800,
                    end_ts: now + 3600,
                    epg_id: Some("b".to_string()),
                    is_now: false,
                },
            ],
            now,
        )
        .unwrap();
        let programs = list_cached_programs(&conn, "p1", "ch1", 10, now + 900).unwrap();
        assert_eq!(programs.len(), 2);
        assert_eq!(programs[0].title, "Morning");
        assert!(programs[0].is_now);
        assert!(!programs[1].is_now);
    }
}
