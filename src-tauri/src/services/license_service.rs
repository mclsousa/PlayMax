use crate::db::settings::{self, KEY_LICENSE_EXPIRES_AT, KEY_LICENSE_KEY, KEY_LICENSE_LAST_VALIDATED, KEY_LICENSE_STATUS};
use crate::db::SharedDb;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use uuid::{uuid, Uuid};

pub const GRACE_OFFLINE_SECS: i64 = 48 * 3600;
pub const REVALIDATE_EVERY_SECS: i64 = 6 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseState {
    pub license_key: Option<String>,
    pub status: Option<String>,
    pub expires_at: Option<i64>,
    pub last_validated_at: Option<i64>,
    pub valid: bool,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseApiResponse {
    ok: bool,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default, rename = "expiresAt")]
    expires_at: Option<String>,
    #[serde(default)]
    valid: Option<bool>,
}

pub fn device_fingerprint() -> String {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".to_string());
    let ns = uuid!("f47ac10b-58cc-4372-a567-0e02b2c3d479");
    Uuid::new_v5(&ns, format!("playmax:{hostname}:{username}").as_bytes()).to_string()
}

pub fn device_name() -> String {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "PC".to_string());
    hostname
}

fn parse_expires_at(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.timestamp())
}

fn read_local_state(db: &SharedDb) -> AppResult<LicenseState> {
    db.with_conn(|conn| {
        let license_key = settings::get_string(conn, KEY_LICENSE_KEY)?;
        let status = settings::get_string(conn, KEY_LICENSE_STATUS)?;
        let expires_at = settings::get_i64(conn, KEY_LICENSE_EXPIRES_AT)?;
        let last_validated_at = settings::get_i64(conn, KEY_LICENSE_LAST_VALIDATED)?;

        let valid = is_locally_valid(&status, expires_at, last_validated_at);

        Ok(LicenseState {
            license_key,
            status,
            expires_at,
            last_validated_at,
            valid,
            message: None,
        })
    })
}

fn is_locally_valid(
    status: &Option<String>,
    expires_at: Option<i64>,
    last_validated_at: Option<i64>,
) -> bool {
    let Some(status) = status.as_deref() else {
        return false;
    };

    if status != "active" && status != "trial" {
        return false;
    }

    if let Some(expires_at) = expires_at {
        let now = chrono::Utc::now().timestamp();
        if expires_at <= now {
            return false;
        }
    }

    if let Some(last_validated_at) = last_validated_at {
        let now = chrono::Utc::now().timestamp();
        if now - last_validated_at > GRACE_OFFLINE_SECS {
            return false;
        }
    } else {
        return false;
    }

    true
}

fn persist_license(
    conn: &rusqlite::Connection,
    license_key: &str,
    status: &str,
    expires_at: Option<i64>,
) -> AppResult<()> {
    settings::set_string(conn, KEY_LICENSE_KEY, Some(license_key))?;
    settings::set_string(conn, KEY_LICENSE_STATUS, Some(status))?;
    if let Some(expires_at) = expires_at {
        settings::set_i64(conn, KEY_LICENSE_EXPIRES_AT, expires_at)?;
    } else {
        settings::set_string(conn, KEY_LICENSE_EXPIRES_AT, None)?;
    }
    settings::set_i64(
        conn,
        KEY_LICENSE_LAST_VALIDATED,
        chrono::Utc::now().timestamp(),
    )?;
    Ok(())
}

async fn post_license(
    api_base_url: &str,
    path: &str,
    license_key: &str,
) -> AppResult<LicenseApiResponse> {
    let base = api_base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(AppError::msg("URL da API de licenças não configurada."));
    }

    let url = format!("{base}{path}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| AppError::msg(format!("HTTP client: {err}")))?;

    let body = serde_json::json!({
        "licenseKey": license_key,
        "deviceFingerprint": device_fingerprint(),
        "deviceName": device_name(),
    });

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|err| AppError::msg(format!("Falha ao contactar servidor de licenças: {err}")))?;

    let payload: LicenseApiResponse = response
        .json()
        .await
        .map_err(|err| AppError::msg(format!("Resposta inválida do servidor de licenças: {err}")))?;

    Ok(payload)
}

fn apply_api_response(db: &SharedDb, license_key: &str, payload: &LicenseApiResponse) -> AppResult<LicenseState> {
    if !payload.ok {
        return Err(AppError::msg(
            payload
                .message
                .clone()
                .unwrap_or_else(|| "Licença inválida.".to_string()),
        ));
    }

    let status = payload
        .status
        .clone()
        .unwrap_or_else(|| "active".to_string());
    let expires_at = payload
        .expires_at
        .as_deref()
        .and_then(parse_expires_at);

    db.with_conn(|conn| persist_license(conn, license_key, &status, expires_at))?;

    read_local_state(db)
}

pub async fn activate_license(db: &SharedDb, api_base_url: String, license_key: String) -> AppResult<LicenseState> {
    let normalized = license_key.trim().to_uppercase();
    if normalized.is_empty() {
        return Err(AppError::msg("Informe a chave PLAY-XXXX."));
    }

    let payload = post_license(&api_base_url, "/api/license/activate", &normalized).await?;
    apply_api_response(db, &normalized, &payload)
}

pub async fn validate_license_online(db: &SharedDb, api_base_url: String) -> AppResult<LicenseState> {
    let local = read_local_state(db)?;
    let Some(license_key) = local.license_key.clone() else {
        return Ok(LicenseState {
            valid: false,
            message: Some("Nenhuma licença ativada.".to_string()),
            ..local
        });
    };

    let payload = post_license(&api_base_url, "/api/license/validate", &license_key).await?;
    apply_api_response(db, &license_key, &payload)
}

pub fn get_license_state(db: &SharedDb) -> AppResult<LicenseState> {
    read_local_state(db)
}

pub fn clear_license(db: &SharedDb) -> AppResult<()> {
    db.with_conn(|conn| {
        settings::set_string(conn, KEY_LICENSE_KEY, None)?;
        settings::set_string(conn, KEY_LICENSE_STATUS, None)?;
        settings::set_string(conn, KEY_LICENSE_EXPIRES_AT, None)?;
        settings::set_i64(conn, KEY_LICENSE_LAST_VALIDATED, 0)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::Profile;
    use crate::db::profiles;
    use crate::db::settings::{KEY_LICENSE_KEY, KEY_LICENSE_LAST_VALIDATED, KEY_LICENSE_STATUS};
    use rusqlite::Connection;
    use std::sync::Arc;

    fn test_db() -> SharedDb {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run(&conn).expect("migrations");
        Arc::new(crate::db::DbState::single(conn))
    }

    #[test]
    fn clear_license_does_not_delete_profiles() {
        let db = test_db();
        db.with_conn(|conn| {
            profiles::insert_profile(
                conn,
                &Profile {
                    id: "p1".to_string(),
                    name: "Lista".to_string(),
                    profile_type: "m3u".to_string(),
                    url: Some("http://example.com/list.m3u".to_string()),
                    file_path: None,
                    username: None,
                    password: None,
                    last_sync: Some(1),
                },
            )
        })
        .expect("insert profile");

        db.with_conn(|conn| {
            settings::set_string(conn, KEY_LICENSE_KEY, Some("PLAY-TEST-TEST-TEST"))?;
            settings::set_string(conn, KEY_LICENSE_STATUS, Some("active"))?;
            settings::set_i64(
                conn,
                KEY_LICENSE_LAST_VALIDATED,
                chrono::Utc::now().timestamp(),
            )?;
            Ok(())
        })
        .expect("seed license");

        clear_license(&db).expect("clear license");

        let profiles_left = db
            .with_conn(profiles::list_profiles)
            .expect("list profiles");
        assert_eq!(profiles_left.len(), 1);
        assert_eq!(profiles_left[0].name, "Lista");

        let license_key = db
            .with_conn(|conn| settings::get_string(conn, KEY_LICENSE_KEY))
            .expect("read license");
        assert!(license_key.is_none());
    }
}

pub async fn ensure_license_valid(db: &SharedDb, api_base_url: String) -> AppResult<LicenseState> {
    let state = read_local_state(db)?;
    if !state.valid {
        return Ok(LicenseState {
            message: Some("Ative sua licença PLAY-XXXX para continuar.".to_string()),
            ..state
        });
    }

    let now = chrono::Utc::now().timestamp();
    let should_revalidate = state
        .last_validated_at
        .map(|last| now - last >= REVALIDATE_EVERY_SECS)
        .unwrap_or(true);

    if !should_revalidate {
        return Ok(state);
    }

    match validate_license_online(db, api_base_url).await {
        Ok(updated) => Ok(updated),
        Err(err) => {
            if is_locally_valid(&state.status, state.expires_at, state.last_validated_at) {
                Ok(LicenseState {
                    message: Some(format!("Modo offline: {err}")),
                    ..state
                })
            } else {
                Err(err)
            }
        }
    }
}
