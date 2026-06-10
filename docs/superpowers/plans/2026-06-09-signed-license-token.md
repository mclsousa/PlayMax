# Licença com Token Assinado (Ed25519) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Substituir a validação local de licença baseada em texto puro no SQLite por um token assinado Ed25519 emitido pelo servidor e verificado no app com chave pública embutida.

**Architecture:** O portal Next.js assina um payload JSON (licenseKey, fingerprint, status, validade, iat, exp=iat+48h) com chave privada Ed25519 guardada em env var; as rotas `activate`/`validate` devolvem o token. O app Rust guarda apenas `license_key` + token no SQLite e só considera a licença válida se a assinatura verificar com a chave pública embutida no binário — editar o banco passa a não liberar nada.

**Tech Stack:** Next.js 15 (Node `crypto` nativo para assinar), Rust/Tauri 2 (`ed25519-dalek` 2 para verificar, `base64`/`serde_json`/`chrono` já existentes), tsx + `node:test` para o teste do portal.

**Spec:** `docs/superpowers/specs/2026-06-09-signed-license-token-design.md`

---

### Task 0: Inicializar repositório git

A pasta `c:\Users\User\Downloads\PlayMax` não é um repositório git. O plano usa commits frequentes, então inicialize um.

**Files:**
- Create: `.gitignore` (raiz)

- [ ] **Step 1: Criar `.gitignore` na raiz**

```gitignore
node_modules/
dist/
.next/
src-tauri/target/
.env.local
.env*.local
*.log
```

(`license-portal/.gitignore` já existe e continua valendo para a subpasta.)

- [ ] **Step 2: Inicializar e commitar estado atual**

```powershell
git init
git add -A
git commit -m "chore: estado inicial antes do token de licenca assinado"
```

Expected: commit criado sem erros. Confirme com `git log --oneline` (1 commit).

---

### Task 1: Script de geração de chaves Ed25519 (portal)

Gera o par de chaves uma única vez. A privada vai para env var; a pública vai para o Rust na Task 4.

**Files:**
- Create: `license-portal/scripts/generate-license-keys.mjs`
- Modify: `license-portal/.env.example`
- Modify: `license-portal/.env.local` (não commitado)
- Create: `docs/superpowers/license-public-key.rust.txt` (só a chave pública — pode ser commitada)

- [ ] **Step 1: Criar o script**

```javascript
// license-portal/scripts/generate-license-keys.mjs
import { generateKeyPairSync } from "node:crypto";

const { privateKey, publicKey } = generateKeyPairSync("ed25519");

const pkcs8 = privateKey.export({ format: "der", type: "pkcs8" });
// SPKI DER de Ed25519: os últimos 32 bytes são a chave pública crua.
const spki = publicKey.export({ format: "der", type: "spki" });
const raw = spki.subarray(spki.length - 32);

console.log("LICENSE_TOKEN_PRIVATE_KEY (cole no Vercel e no .env.local — NUNCA commitar):");
console.log(pkcs8.toString("base64"));
console.log("");
console.log("LICENSE_PUBLIC_KEY (cole em src-tauri/src/services/license_token.rs):");
console.log(`pub const LICENSE_PUBLIC_KEY: [u8; 32] = [${Array.from(raw).join(", ")}];`);
```

- [ ] **Step 2: Rodar o script**

```powershell
node license-portal/scripts/generate-license-keys.mjs
```

Expected: imprime uma linha base64 (privada) e uma linha `pub const LICENSE_PUBLIC_KEY: [u8; 32] = [...];` (pública).

- [ ] **Step 3: Guardar as chaves**

1. Adicione ao `license-portal/.env.local` (arquivo já existe, está no .gitignore):
   ```
   LICENSE_TOKEN_PRIVATE_KEY=<base64 impresso no passo 2>
   ```
2. Salve a linha `pub const LICENSE_PUBLIC_KEY...` em `docs/superpowers/license-public-key.rust.txt` (será colada no Rust na Task 4).

- [ ] **Step 4: Documentar no `.env.example`**

Adicione ao final de `license-portal/.env.example`:

```
# Assinatura de tokens de licença (gere com: node scripts/generate-license-keys.mjs)
LICENSE_TOKEN_PRIVATE_KEY=base64-pkcs8-der...
```

- [ ] **Step 5: Commit**

```powershell
git add license-portal/scripts/generate-license-keys.mjs license-portal/.env.example docs/superpowers/license-public-key.rust.txt
git commit -m "feat(portal): script de geracao de chaves Ed25519 para tokens de licenca"
```

(Confirme antes que `.env.local` NÃO aparece em `git status`.)

---

### Task 2: Módulo de assinatura no portal + teste roundtrip

**Files:**
- Create: `license-portal/src/lib/license-token.ts`
- Create: `license-portal/tests/license-token.test.ts`
- Modify: `license-portal/package.json`

- [ ] **Step 1: Instalar tsx e adicionar script de teste**

```powershell
cd license-portal; npm install -D tsx
```

Em `license-portal/package.json`, adicione em `"scripts"`:

```json
"test": "node --import tsx --test tests/license-token.test.ts"
```

- [ ] **Step 2: Escrever o teste (falhando)**

```typescript
// license-portal/tests/license-token.test.ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync, verify } from "node:crypto";
import type { LicenseRow } from "../src/lib/supabase";
// Import estático é seguro: o módulo só lê LICENSE_TOKEN_PRIVATE_KEY na
// primeira chamada de signLicenseToken (lazy), depois deste setup.
import { signLicenseToken } from "../src/lib/license-token";

// Gera par efêmero e injeta a privada antes de qualquer assinatura.
const { privateKey, publicKey } = generateKeyPairSync("ed25519");
process.env.LICENSE_TOKEN_PRIVATE_KEY = privateKey
  .export({ format: "der", type: "pkcs8" })
  .toString("base64");

function fakeLicense(overrides: Partial<LicenseRow> = {}): LicenseRow {
  return {
    id: "lic-1",
    customer_id: null,
    license_key: "PLAY-AAAA-BBBB-CCCC",
    status: "active",
    expires_at: "2030-01-01T00:00:00.000Z",
    stripe_subscription_id: null,
    max_devices: 1,
    notes: null,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

test("token assinado: payload correto e assinatura verifica", () => {
  const before = Math.floor(Date.now() / 1000);
  const token = signLicenseToken(fakeLicense(), "fp-123");
  const after = Math.floor(Date.now() / 1000);

  const [payloadB64, sigB64] = token.split(".");
  const payloadBytes = Buffer.from(payloadB64, "base64url");
  const sigBytes = Buffer.from(sigB64, "base64url");

  assert.equal(verify(null, payloadBytes, publicKey, sigBytes), true);

  const claims = JSON.parse(payloadBytes.toString("utf8"));
  assert.equal(claims.v, 1);
  assert.equal(claims.licenseKey, "PLAY-AAAA-BBBB-CCCC");
  assert.equal(claims.fingerprint, "fp-123");
  assert.equal(claims.status, "active");
  assert.equal(claims.licenseExpiresAt, Math.floor(Date.parse("2030-01-01T00:00:00.000Z") / 1000));
  assert.ok(claims.iat >= before && claims.iat <= after);
  assert.equal(claims.exp, claims.iat + 48 * 3600);
});

test("licenca sem expires_at gera licenseExpiresAt null", () => {
  const token = signLicenseToken(fakeLicense({ expires_at: null }), "fp-123");
  const claims = JSON.parse(Buffer.from(token.split(".")[0], "base64url").toString("utf8"));
  assert.equal(claims.licenseExpiresAt, null);
});

test("payload adulterado nao verifica", () => {
  const token = signLicenseToken(fakeLicense(), "fp-123");
  const [payloadB64, sigB64] = token.split(".");
  const tampered = Buffer.from(payloadB64, "base64url");
  tampered[0] ^= 0xff;
  assert.equal(verify(null, tampered, publicKey, Buffer.from(sigB64, "base64url")), false);
});
```

- [ ] **Step 3: Rodar e ver falhar**

```powershell
cd license-portal; npm test
```

Expected: FAIL — `Cannot find module '../src/lib/license-token'`.

- [ ] **Step 4: Implementar o módulo**

```typescript
// license-portal/src/lib/license-token.ts
import { createPrivateKey, sign, type KeyObject } from "node:crypto";
import type { LicenseRow } from "./supabase";

const GRACE_OFFLINE_SECS = 48 * 3600;

let cachedKey: KeyObject | null = null;

function getPrivateKey(): KeyObject {
  if (cachedKey) return cachedKey;
  const b64 = process.env.LICENSE_TOKEN_PRIVATE_KEY;
  if (!b64) {
    throw new Error("Missing LICENSE_TOKEN_PRIVATE_KEY");
  }
  cachedKey = createPrivateKey({
    key: Buffer.from(b64, "base64"),
    format: "der",
    type: "pkcs8",
  });
  return cachedKey;
}

export function signLicenseToken(license: LicenseRow, deviceFingerprint: string): string {
  const now = Math.floor(Date.now() / 1000);
  const payload = {
    v: 1,
    licenseKey: license.license_key,
    fingerprint: deviceFingerprint,
    status: license.status,
    licenseExpiresAt: license.expires_at
      ? Math.floor(new Date(license.expires_at).getTime() / 1000)
      : null,
    iat: now,
    exp: now + GRACE_OFFLINE_SECS,
  };
  const payloadBytes = Buffer.from(JSON.stringify(payload), "utf8");
  const signature = sign(null, payloadBytes, getPrivateKey());
  return `${payloadBytes.toString("base64url")}.${signature.toString("base64url")}`;
}
```

- [ ] **Step 5: Rodar e ver passar**

```powershell
cd license-portal; npm test
```

Expected: 3 tests PASS.

- [ ] **Step 6: Commit**

```powershell
git add license-portal/src/lib/license-token.ts license-portal/tests/license-token.test.ts license-portal/package.json license-portal/package-lock.json
git commit -m "feat(portal): assinatura Ed25519 de tokens de licenca"
```

---

### Task 3: Rotas activate/validate devolvem o token

**Files:**
- Modify: `license-portal/src/lib/license.ts` (funções `activateLicense` e `validateLicense`)

- [ ] **Step 1: Incluir token nos retornos**

Em `license-portal/src/lib/license.ts`, adicione o import no topo:

```typescript
import { signLicenseToken } from "./license-token";
```

Em `activateLicense`, troque os DOIS retornos:

```typescript
// antes (ocorre 2x — ativação existente e nova):
return licensePayload(license);
// depois:
return {
  ...licensePayload(license),
  token: signLicenseToken(license, deviceFingerprint),
};
```

Em `validateLicense`, troque o retorno final:

```typescript
// antes:
return licensePayload(license);
// depois:
return {
  ...licensePayload(license),
  token: signLicenseToken(license, deviceFingerprint),
};
```

As rotas `route.ts` já fazem `{ ok: true, ...result }` — nada a mudar nelas. Versões antigas do app ignoram o campo extra `token` (compatível).

- [ ] **Step 2: Verificar tipos e testes**

```powershell
cd license-portal; npx tsc --noEmit; npm test
```

Expected: tsc sem erros; 3 tests PASS.

- [ ] **Step 3: Commit**

```powershell
git add license-portal/src/lib/license.ts
git commit -m "feat(portal): rotas activate/validate retornam token assinado"
```

---

### Task 4: Módulo Rust de verificação (`license_token.rs`)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/services/license_token.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Adicionar dependência**

Em `src-tauri/Cargo.toml`, na seção `[dependencies]`, adicione:

```toml
ed25519-dalek = "2"
```

- [ ] **Step 2: Registrar o módulo**

Em `src-tauri/src/services/mod.rs`, adicione (em ordem alfabética):

```rust
pub mod license_token;
```

- [ ] **Step 3: Criar o módulo com testes (TDD: testes escritos junto, rodados antes de qualquer integração)**

Crie `src-tauri/src/services/license_token.rs`. Em `LICENSE_PUBLIC_KEY`, cole o array gerado na Task 1 (arquivo `docs/superpowers/license-public-key.rust.txt`) — NÃO deixe o placeholder:

```rust
use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

/// Chave pública Ed25519 do servidor de licenças.
/// Gerada por license-portal/scripts/generate-license-keys.mjs (Task 1).
pub const LICENSE_PUBLIC_KEY: [u8; 32] = [/* COLE AQUI o array de docs/superpowers/license-public-key.rust.txt */];

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
        // troca "active" por "actixe" dentro do payload sem reassinar
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
```

- [ ] **Step 4: Rodar os testes**

```powershell
cd src-tauri; cargo test license_token
```

Expected: 9 tests PASS (primeira execução compila `ed25519-dalek`).

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/services/mod.rs src-tauri/src/services/license_token.rs
git commit -m "feat(app): verificacao Ed25519 de tokens de licenca"
```

---

### Task 5: `license_service.rs` passa a confiar só no token

**Files:**
- Modify: `src-tauri/src/db/settings.rs` (consts de licença)
- Modify: `src-tauri/src/db/migrations.rs:471` (função `run` — limpeza de chaves legadas)
- Modify: `src-tauri/src/services/license_service.rs` (reescrita do estado local)

- [ ] **Step 1: Atualizar consts em `settings.rs`**

Em `src-tauri/src/db/settings.rs`, substitua:

```rust
pub const KEY_LICENSE_KEY: &str = "license_key";
pub const KEY_LICENSE_STATUS: &str = "license_status";
pub const KEY_LICENSE_EXPIRES_AT: &str = "license_expires_at";
pub const KEY_LICENSE_LAST_VALIDATED: &str = "license_last_validated_at";
```

por:

```rust
pub const KEY_LICENSE_KEY: &str = "license_key";
pub const KEY_LICENSE_TOKEN: &str = "license_token";
```

- [ ] **Step 2: Limpar chaves legadas na migração**

Em `src-tauri/src/db/migrations.rs`, dentro de `pub fn run` (linha ~471), adicione imediatamente antes do `Ok(())` final da função (idempotente, roda em todo start):

```rust
    // Migração para token assinado: os valores texto-puro de licença deixaram
    // de ser fonte de verdade e são removidos.
    conn.execute(
        "DELETE FROM app_settings WHERE key IN ('license_status', 'license_expires_at', 'license_last_validated_at')",
        [],
    )?;
```

- [ ] **Step 3: Escrever os testes novos (falhando) em `license_service.rs`**

Substitua o `mod tests` existente em `src-tauri/src/services/license_service.rs` por (o teste de perfis é mantido, adaptado ao token):

```rust
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
}
```

- [ ] **Step 4: Rodar e ver falhar**

```powershell
cd src-tauri; cargo test license_service
```

Expected: FAIL de compilação (`KEY_LICENSE_TOKEN` ainda não usado no service, `LicenseApiResponse` sem campo `token`, etc.).

- [ ] **Step 5: Reescrever o corpo de `license_service.rs`**

Substitua o conteúdo do arquivo ANTES do `mod tests` (que foi escrito no Step 3) e a função `ensure_license_valid` do final do arquivo por:

```rust
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
```

Notas para o executor:
- `GRACE_OFFLINE_SECS` e `is_locally_valid` foram REMOVIDOS — o grace agora é o `exp` assinado. Se `GRACE_OFFLINE_SECS` for referenciado em outro lugar, rode `cargo build` e corrija (hoje só o service usa).
- `ensure_license_valid` muda de comportamento na migração: com `license_key` salva e sem token, ele tenta validar online em vez de só pedir ativação (transparente para usuários migrando com internet).
- O frontend não muda: `LicenseState` mantém o mesmo shape serializado.

- [ ] **Step 6: Rodar todos os testes**

```powershell
cd src-tauri; cargo test
```

Expected: todos PASS, incluindo os 5 novos de `license_service` e os 9 de `license_token`; nenhum warning de import não usado.

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/src/db/settings.rs src-tauri/src/db/migrations.rs src-tauri/src/services/license_service.rs
git commit -m "feat(app): licenca local valida somente com token assinado do servidor"
```

---

### Task 6: Configuração de produção e verificação final

**Files:** nenhum código novo — configuração e verificação.

- [ ] **Step 1: Configurar a chave privada no Vercel**

No dashboard do Vercel (projeto do license-portal) → Settings → Environment Variables, crie `LICENSE_TOKEN_PRIVATE_KEY` com o base64 da Task 1 (Production + Preview). Depois faça redeploy do portal.

⚠️ Sem isso, `activate`/`validate` em produção retornam 500 (fail closed) e NENHUM app consegue validar. Faça este passo antes de distribuir o app novo.

- [ ] **Step 2: Build completo do portal**

```powershell
cd license-portal; npm run build
```

Expected: build sem erros.

- [ ] **Step 3: Build e testes completos do app**

```powershell
cd src-tauri; cargo build; cargo test
```

Expected: build limpo, todos os testes PASS.

- [ ] **Step 4: Teste E2E manual (com o portal rodando com a chave no .env.local)**

1. `cd license-portal; npm run dev`
2. Rode o app (`npm run tauri dev` na raiz) e ative uma licença de teste → app libera.
3. Confira no `playmax.db` (tabela `app_settings`): existem `license_key` e `license_token`; NÃO existem `license_status`/`license_expires_at`/`license_last_validated_at`.
4. **Reproduza o ataque**: feche o app, insira via editor SQLite `license_status='active'` e `license_expires_at='99999999999'`, apague `license_token`. Abra o app → deve pedir validação/ativação (e revalidar online se houver internet). O ataque não libera mais nada.
5. Adultere 1 caractere do `license_token` no banco → app inválido até revalidar online.

- [ ] **Step 5: Commit final e atualização do spec**

Marque o spec como implementado (linha `**Status:**` → `Implementado em 2026-06-09`) e commite:

```powershell
git add docs/superpowers/specs/2026-06-09-signed-license-token-design.md docs/superpowers/plans/2026-06-09-signed-license-token.md
git commit -m "docs: spec e plano do token de licenca assinado"
```
