use crate::error::AppResult;
use rusqlite::{params, Connection};

pub const KEY_AUTO_SYNC: &str = "auto_sync_enabled";
pub const KEY_LAST_BACKGROUND_SYNC: &str = "last_background_sync";
pub const KEY_PARENTAL_ENABLED: &str = "parental_control_enabled";
pub const KEY_PARENTAL_PIN_HASH: &str = "parental_pin_hash";
pub const KEY_ACTIVE_PROFILE: &str = "active_profile_id";
pub const KEY_LICENSE_KEY: &str = "license_key";
pub const KEY_LICENSE_STATUS: &str = "license_status";
pub const KEY_LICENSE_EXPIRES_AT: &str = "license_expires_at";
pub const KEY_LICENSE_LAST_VALIDATED: &str = "license_last_validated_at";

pub fn get_bool(conn: &Connection, key: &str, default: bool) -> AppResult<bool> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok();
    Ok(match value.as_deref() {
        Some("true") | Some("1") => true,
        Some("false") | Some("0") => false,
        Some(_) => default,
        None => default,
    })
}

pub fn set_bool(conn: &Connection, key: &str, value: bool) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, if value { "true" } else { "false" }],
    )?;
    Ok(())
}

pub fn get_string(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok();
    Ok(value.filter(|v| !v.trim().is_empty()))
}

pub fn set_string(conn: &Connection, key: &str, value: Option<&str>) -> AppResult<()> {
    match value.filter(|v| !v.trim().is_empty()) {
        Some(v) => {
            conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, v.trim()],
            )?;
        }
        None => {
            conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        }
    }
    Ok(())
}

pub fn get_i64(conn: &Connection, key: &str) -> AppResult<Option<i64>> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok();
    Ok(value.and_then(|v| v.parse().ok()))
}

pub fn set_i64(conn: &Connection, key: &str, value: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )?;
    Ok(())
}
