use crate::db::models::Profile;
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

fn map_profile_row(row: &rusqlite::Row<'_>, include_password: bool) -> rusqlite::Result<Profile> {
    Ok(Profile {
        id: row.get(0)?,
        name: row.get(1)?,
        profile_type: row.get(2)?,
        url: row.get(3)?,
        file_path: row.get(4)?,
        username: row.get(5)?,
        password: if include_password { row.get(6)? } else { None },
        last_sync: if include_password {
            row.get(7)?
        } else {
            row.get(6)?
        },
    })
}

pub fn list_profiles(conn: &Connection) -> AppResult<Vec<Profile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, url, file_path, username, last_sync FROM profiles ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], |row| map_profile_row(row, false))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(e))
}

pub fn get_profile(conn: &Connection, id: &str) -> AppResult<Option<Profile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, url, file_path, username, password, last_sync FROM profiles WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(map_profile_row(&row, true)?))
    } else {
        Ok(None)
    }
}

pub fn insert_profile(conn: &Connection, profile: &Profile) -> AppResult<()> {
    conn.execute(
        "INSERT INTO profiles (id, name, type, url, file_path, username, password, last_sync) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            profile.id,
            profile.name,
            profile.profile_type,
            profile.url,
            profile.file_path,
            profile.username,
            profile.password,
            profile.last_sync
        ],
    )?;
    Ok(())
}

pub fn delete_profile(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM profiles WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn update_last_sync(conn: &Connection, id: &str, last_sync: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE profiles SET last_sync = ?1 WHERE id = ?2",
        params![last_sync, id],
    )?;
    Ok(())
}

pub fn update_profile(conn: &Connection, profile: &Profile) -> AppResult<()> {
    conn.execute(
        "UPDATE profiles SET name = ?2, type = ?3, url = ?4, file_path = ?5, username = ?6, password = ?7 WHERE id = ?1",
        params![
            profile.id,
            profile.name,
            profile.profile_type,
            profile.url,
            profile.file_path,
            profile.username,
            profile.password,
        ],
    )?;
    Ok(())
}
