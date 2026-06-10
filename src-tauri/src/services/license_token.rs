use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

/// Chave pública Ed25519 do servidor de licenças.
/// Gerada por license-portal/scripts/generate-license-keys.mjs.
pub const LICENSE_PUBLIC_KEY: [u8; 32] = [118, 215, 174, 37, 250, 6, 196, 218, 110, 33, 6, 240, 251, 87, 8, 0, 14, 193, 141, 5, 246, 105, 1, 171, 105, 249, 140, 48, 247, 142, 193, 78];

/// Tolerância para relógio adiantado em relação ao servidor (5 min).
pub const CLOCK_SKEW_SECS: i64 = 300;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenClaims {
    pub v: u32,
    #[serde(rename = "licenseKey")]
    pub license_key: String,
    pub fingerprint: String,
    pub status: String,
    #[serde(rename = "licenseExpiresAt")]
    pub license_expires_at: Option<i64>,
    pub iat: i64,
    pub exp: i64,
}

#[cfg(test)]
thread_local! {
    static TEST_PUBLIC_KEY: std::cell::Cell<Option<[u8; 32]>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub fn set_test_public_key(key: [u8; 32]) {
    TEST_PUBLIC_KEY.with(|k| k.set(Some(key)));
}

fn embedded_public_key() -> [u8; 32] {
    #[cfg(test)]
    {
        if let Some(key) = TEST_PUBLIC_KEY.with(|k| k.get()) {
            return key;
        }
    }
    LICENSE_PUBLIC_KEY
}

/// Verifica o token com a chave pública embutida e o relógio atual.
pub fn verify_token(token: &str, expected_fingerprint: &str) -> AppResult<TokenClaims> {
    verify_token_with_key(
        token,
        expected_fingerprint,
        chrono::Utc::now().timestamp(),
        &embedded_public_key(),
    )
}

/// Núcleo verificável: assinatura → fingerprint → exp → iat (rollback de
/// relógio) → status → vencimento real da licença.
pub fn verify_token_with_key(
    token: &str,
    expected_fingerprint: &str,
    now: i64,
    public_key: &[u8; 32],
) -> AppResult<TokenClaims> {
    let (payload_b64, signature_b64) = token
        .split_once('.')
        .ok_or_else(|| AppError::msg("Token de licença malformado."))?;
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| AppError::msg("Token de licença malformado."))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| AppError::msg("Token de licença malformado."))?;

    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| AppError::msg("Chave pública de licença inválida."))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| AppError::msg("Assinatura de licença malformada."))?;
    verifying_key
        .verify_strict(&payload_bytes, &signature)
        .map_err(|_| AppError::msg("Assinatura de licença inválida."))?;

    let claims: TokenClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|_| AppError::msg("Conteúdo do token de licença inválido."))?;

    if claims.v != 1 {
        return Err(AppError::msg("Versão de token de licença desconhecida."));
    }
    if claims.fingerprint != expected_fingerprint {
        return Err(AppError::msg("Token de licença pertence a outro dispositivo."));
    }
    if now >= claims.exp {
        return Err(AppError::msg("Token de licença expirado. Conecte-se para revalidar."));
    }
    if now < claims.iat - CLOCK_SKEW_SECS {
        return Err(AppError::msg("Relógio do sistema inconsistente com a licença."));
    }
    if claims.status != "active" && claims.status != "trial" {
        return Err(AppError::msg("Licença inativa."));
    }
    if let Some(expires) = claims.license_expires_at {
        if expires <= now {
            return Err(AppError::msg("Licença expirada."));
        }
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_keypair() -> (SigningKey, [u8; 32]) {
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let public = signing.verifying_key().to_bytes();
        (signing, public)
    }

    fn claims_json(now: i64) -> serde_json::Value {
        serde_json::json!({
            "v": 1,
            "licenseKey": "PLAY-AAAA-BBBB-CCCC",
            "fingerprint": "fp-teste",
            "status": "active",
            "licenseExpiresAt": now + 365 * 24 * 3600,
            "iat": now,
            "exp": now + 48 * 3600,
        })
    }

    fn make_token(payload: &serde_json::Value, key: &SigningKey) -> String {
        let bytes = serde_json::to_vec(payload).unwrap();
        let sig = key.sign(&bytes);
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&bytes),
            URL_SAFE_NO_PAD.encode(sig.to_bytes())
        )
    }

    #[test]
    fn token_valido_passa() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let token = make_token(&claims_json(now), &signing);
        let claims = verify_token_with_key(&token, "fp-teste", now + 60, &public).unwrap();
        assert_eq!(claims.license_key, "PLAY-AAAA-BBBB-CCCC");
        assert_eq!(claims.status, "active");
        assert_eq!(claims.iat, now);
    }

    #[test]
    fn payload_adulterado_falha() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let token = make_token(&claims_json(now), &signing);
        let (payload_b64, sig_b64) = token.split_once('.').unwrap();
        let mut bytes = URL_SAFE_NO_PAD.decode(payload_b64).unwrap();
        // troca um byte de "active" dentro do payload sem reassinar
        let pos = bytes.windows(6).position(|w| w == b"active").unwrap();
        bytes[pos + 4] = b'x';
        let tampered = format!("{}.{}", URL_SAFE_NO_PAD.encode(&bytes), sig_b64);
        assert!(verify_token_with_key(&tampered, "fp-teste", now + 60, &public).is_err());
    }

    #[test]
    fn fingerprint_divergente_falha() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let token = make_token(&claims_json(now), &signing);
        assert!(verify_token_with_key(&token, "fp-outro-pc", now + 60, &public).is_err());
    }

    #[test]
    fn token_expirado_falha() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let token = make_token(&claims_json(now), &signing);
        assert!(verify_token_with_key(&token, "fp-teste", now + 48 * 3600 + 1, &public).is_err());
    }

    #[test]
    fn relogio_antes_do_iat_falha() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let token = make_token(&claims_json(now), &signing);
        assert!(verify_token_with_key(&token, "fp-teste", now - 600, &public).is_err());
    }

    #[test]
    fn status_revogado_falha_mesmo_assinado() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let mut payload = claims_json(now);
        payload["status"] = serde_json::json!("revoked");
        let token = make_token(&payload, &signing);
        assert!(verify_token_with_key(&token, "fp-teste", now + 60, &public).is_err());
    }

    #[test]
    fn licenca_vencida_dentro_da_janela_falha() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let mut payload = claims_json(now);
        payload["licenseExpiresAt"] = serde_json::json!(now + 60);
        let token = make_token(&payload, &signing);
        assert!(verify_token_with_key(&token, "fp-teste", now + 120, &public).is_err());
    }

    #[test]
    fn versao_desconhecida_falha() {
        let (signing, public) = test_keypair();
        let now = 1_750_000_000;
        let mut payload = claims_json(now);
        payload["v"] = serde_json::json!(2);
        let token = make_token(&payload, &signing);
        assert!(verify_token_with_key(&token, "fp-teste", now + 60, &public).is_err());
    }

    #[test]
    fn chave_publica_errada_falha() {
        let (signing, _) = test_keypair();
        let other_public = SigningKey::from_bytes(&[9u8; 32]).verifying_key().to_bytes();
        let now = 1_750_000_000;
        let token = make_token(&claims_json(now), &signing);
        assert!(verify_token_with_key(&token, "fp-teste", now + 60, &other_public).is_err());
    }
}
