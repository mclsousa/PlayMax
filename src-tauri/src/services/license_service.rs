use crate::db::settings::{self, KEY_LICENSE_KEY, KEY_LICENSE_TOKEN};
use crate::db::SharedDb;
use crate::error::{AppError, AppResult};
use crate::services::license_token::{self, TokenClaims};
use serde::{Deserialize, Serialize};
use uuid::{uuid, Uuid};

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
    token: Option<String>,
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
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "PC".to_string())
}

fn state_from_claims(license_key: Option<String>, claims: &TokenClaims) -> LicenseState {
    LicenseState {
        license_key,
        status: Some(claims.status.clone()),
        expires_at: claims.license_expires_at,
        last_validated_at: Some(claims.iat),
        valid: true,
        message: None,
    }
}

fn invalid_state(license_key: Option<String>) -> LicenseState {
    LicenseState {
        license_key,
        status: None,
        expires_at: None,
        last_validated_at: None,
        valid: false,
        message: None,
    }
}

/// A ÚNICA fonte de verdade local é o token assinado: sem token verificado,
/// o app está sem licença — valores soltos no SQLite não liberam nada.
fn read_local_state(db: &SharedDb) -> AppResult<LicenseState> {
    db.with_conn(|conn| {
        let license_key = settings::get_string(conn, KEY_LICENSE_KEY)?;
        let token = settings::get_string(conn, KEY_LICENSE_TOKEN)?;
        let state = match token.as_deref() {
            Some(token) => match license_token::verify_token(token, &device_fingerprint()) {
                Ok(claims) => state_from_claims(license_key, &claims),
                Err(_) => invalid_state(license_key),
            },
            None => invalid_state(license_key),
        };
        Ok(state)
    })
}

fn persist_license(conn: &rusqlite::Connection, license_key: &str, token: &str) -> AppResult<()> {
    settings::set_string(conn, KEY_LICENSE_KEY, Some(license_key))?;
    settings::set_string(conn, KEY_LICENSE_TOKEN, Some(token))?;
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

/// Exige token assinado na resposta e o verifica ANTES de persistir.
fn apply_api_response(
    db: &SharedDb,
    license_key: &str,
    payload: &LicenseApiResponse,
) -> AppResult<LicenseState> {
    if !payload.ok {
        // Rejeição autoritativa do servidor sobre esta licença/dispositivo:
        // o token local deixa de ser honrado imediatamente. Erros internos
        // do servidor (SERVER_ERROR) não derrubam o token — a janela offline
        // assinada continua cobrindo indisponibilidade do portal.
        const AUTHORITATIVE_REJECTIONS: [&str; 5] = [
            "LICENSE_REVOKED",
            "LICENSE_EXPIRED",
            "LICENSE_NOT_FOUND",
            "DEVICE_NOT_ACTIVATED",
            "DEVICE_LIMIT",
        ];
        let authoritative = payload
            .code
            .as_deref()
            .map(|code| AUTHORITATIVE_REJECTIONS.contains(&code))
            .unwrap_or(false);
        if authoritative {
            db.with_conn(|conn| settings::set_string(conn, KEY_LICENSE_TOKEN, None))?;
        }
        return Err(AppError::msg(
            payload
                .message
                .clone()
                .unwrap_or_else(|| "Licença inválida.".to_string()),
        ));
    }

    let Some(token) = payload.token.as_deref() else {
        return Err(AppError::msg(
            "Resposta do servidor de licenças sem token assinado. Atualize o portal.",
        ));
    };

    let claims = license_token::verify_token(token, &device_fingerprint())?;
    if claims.license_key != license_key {
        return Err(AppError::msg("Token assinado para outra licença."));
    }

    db.with_conn(|conn| persist_license(conn, license_key, token))?;

    Ok(state_from_claims(Some(license_key.to_string()), &claims))
}

pub async fn activate_license(
    db: &SharedDb,
    api_base_url: String,
    license_key: String,
) -> AppResult<LicenseState> {
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
        settings::set_string(conn, KEY_LICENSE_TOKEN, None)?;
        Ok(())
    })
}

pub async fn ensure_license_valid(db: &SharedDb, api_base_url: String) -> AppResult<LicenseState> {
    let state = read_local_state(db)?;

    if !state.valid {
        // Sem token válido (instalação migrada, expirado ou adulterado):
        // a única saída é validar online com a chave salva.
        if state.license_key.is_some() {
            return validate_license_online(db, api_base_url).await;
        }
        return Ok(LicenseState {
            message: Some("Ative sua licença PLAY-XXXX para continuar.".to_string()),
            ..state
        });
    }

    let now = chrono::Utc::now().timestamp();
    let should_revalidate = state
        .last_validated_at
        .map(|iat| now - iat >= REVALIDATE_EVERY_SECS)
        .unwrap_or(true);

    if !should_revalidate {
        return Ok(state);
    }

    match validate_license_online(db, api_base_url.clone()).await {
        Ok(updated) => Ok(updated),
        Err(err) => {
            // Offline: segue válido enquanto o token assinado não expirar (48h).
            let recheck = read_local_state(db)?;
            if recheck.valid {
                Ok(LicenseState {
                    message: Some(format!("Modo offline: {err}")),
                    ..recheck
                })
            } else {
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::Profile;
    use crate::db::profiles;
    use crate::db::settings::{KEY_LICENSE_KEY, KEY_LICENSE_TOKEN};
    use crate::services::license_token;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};
    use rusqlite::Connection;
    use std::sync::Arc;

    fn test_db() -> SharedDb {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run(&conn).expect("migrations");
        Arc::new(crate::db::DbState::single(conn))
    }

    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn make_valid_token(signing: &SigningKey) -> String {
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "v": 1,
            "licenseKey": "PLAY-AAAA-BBBB-CCCC",
            "fingerprint": device_fingerprint(),
            "status": "active",
            "licenseExpiresAt": now + 365 * 24 * 3600,
            "iat": now,
            "exp": now + 48 * 3600,
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let sig = signing.sign(&bytes);
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&bytes),
            URL_SAFE_NO_PAD.encode(sig.to_bytes())
        )
    }

    #[test]
    fn estado_sem_token_e_invalido_mesmo_com_status_editado_no_sqlite() {
        let db = test_db();
        // Cenário do ataque: usuário grava status='active' e data futura direto no banco.
        db.with_conn(|conn| {
            settings::set_string(conn, KEY_LICENSE_KEY, Some("PLAY-AAAA-BBBB-CCCC"))?;
            conn.execute(
                "INSERT INTO app_settings (key, value) VALUES
                 ('license_status', 'active'),
                 ('license_expires_at', '99999999999'),
                 ('license_last_validated_at', '99999999999')",
                [],
            )?;
            Ok(())
        })
        .expect("seed ataque");

        let state = get_license_state(&db).expect("state");
        assert!(!state.valid, "valores texto-puro nao podem liberar o app");
    }

    #[test]
    fn token_adulterado_e_invalido() {
        let db = test_db();
        let signing = test_signing_key();
        license_token::set_test_public_key(signing.verifying_key().to_bytes());

        let token = make_valid_token(&signing);
        let (payload_b64, sig_b64) = token.split_once('.').unwrap();
        let mut bytes = URL_SAFE_NO_PAD.decode(payload_b64).unwrap();
        let pos = bytes.windows(6).position(|w| w == b"active").unwrap();
        bytes[pos] = b'x';
        let tampered = format!("{}.{}", URL_SAFE_NO_PAD.encode(&bytes), sig_b64);

        db.with_conn(|conn| {
            settings::set_string(conn, KEY_LICENSE_KEY, Some("PLAY-AAAA-BBBB-CCCC"))?;
            settings::set_string(conn, KEY_LICENSE_TOKEN, Some(&tampered))?;
            Ok(())
        })
        .expect("seed");

        let state = get_license_state(&db).expect("state");
        assert!(!state.valid);
    }

    #[test]
    fn token_valido_libera_e_preenche_estado() {
        let db = test_db();
        let signing = test_signing_key();
        license_token::set_test_public_key(signing.verifying_key().to_bytes());

        let token = make_valid_token(&signing);
        db.with_conn(|conn| {
            settings::set_string(conn, KEY_LICENSE_KEY, Some("PLAY-AAAA-BBBB-CCCC"))?;
            settings::set_string(conn, KEY_LICENSE_TOKEN, Some(&token))?;
            Ok(())
        })
        .expect("seed");

        let state = get_license_state(&db).expect("state");
        assert!(state.valid);
        assert_eq!(state.status.as_deref(), Some("active"));
        assert!(state.expires_at.is_some());
        assert!(state.last_validated_at.is_some());
    }

    #[test]
    fn resposta_de_api_sem_token_e_erro_e_nada_persiste() {
        let db = test_db();
        let payload = LicenseApiResponse {
            ok: true,
            code: None,
            message: None,
            token: None,
        };
        let result = apply_api_response(&db, "PLAY-AAAA-BBBB-CCCC", &payload);
        assert!(result.is_err());

        let token = db
            .with_conn(|conn| settings::get_string(conn, KEY_LICENSE_TOKEN))
            .expect("read token");
        assert!(token.is_none());
    }

    #[test]
    fn clear_license_remove_token_e_nao_apaga_perfis() {
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
            settings::set_string(conn, KEY_LICENSE_TOKEN, Some("abc.def"))?;
            Ok(())
        })
        .expect("seed license");

        clear_license(&db).expect("clear license");

        let profiles_left = db
            .with_conn(profiles::list_profiles)
            .expect("list profiles");
        assert_eq!(profiles_left.len(), 1);

        let (key, token) = db
            .with_conn(|conn| {
                Ok((
                    settings::get_string(conn, KEY_LICENSE_KEY)?,
                    settings::get_string(conn, KEY_LICENSE_TOKEN)?,
                ))
            })
            .expect("read");
        assert!(key.is_none());
        assert!(token.is_none());
    }

    #[test]
    fn rejeicao_autoritativa_limpa_token_local() {
        let db = test_db();
        let signing = test_signing_key();
        license_token::set_test_public_key(signing.verifying_key().to_bytes());

        let token = make_valid_token(&signing);
        db.with_conn(|conn| {
            settings::set_string(conn, KEY_LICENSE_KEY, Some("PLAY-AAAA-BBBB-CCCC"))?;
            settings::set_string(conn, KEY_LICENSE_TOKEN, Some(&token))?;
            Ok(())
        })
        .expect("seed");

        let payload = LicenseApiResponse {
            ok: false,
            code: Some("LICENSE_REVOKED".to_string()),
            message: Some("Licença bloqueada.".to_string()),
            token: None,
        };
        let result = apply_api_response(&db, "PLAY-AAAA-BBBB-CCCC", &payload);
        assert!(result.is_err());

        let stored = db
            .with_conn(|conn| settings::get_string(conn, KEY_LICENSE_TOKEN))
            .expect("read token");
        assert!(stored.is_none(), "token deve ser removido em revogacao");

        let state = get_license_state(&db).expect("state");
        assert!(!state.valid);
    }

    #[test]
    fn erro_interno_do_servidor_preserva_token_local() {
        let db = test_db();
        let signing = test_signing_key();
        license_token::set_test_public_key(signing.verifying_key().to_bytes());

        let token = make_valid_token(&signing);
        db.with_conn(|conn| {
            settings::set_string(conn, KEY_LICENSE_KEY, Some("PLAY-AAAA-BBBB-CCCC"))?;
            settings::set_string(conn, KEY_LICENSE_TOKEN, Some(&token))?;
            Ok(())
        })
        .expect("seed");

        let payload = LicenseApiResponse {
            ok: false,
            code: Some("SERVER_ERROR".to_string()),
            message: Some("Erro interno.".to_string()),
            token: None,
        };
        let result = apply_api_response(&db, "PLAY-AAAA-BBBB-CCCC", &payload);
        assert!(result.is_err());

        let state = get_license_state(&db).expect("state");
        assert!(state.valid, "token integro deve continuar valendo em erro interno do servidor");
    }
}
