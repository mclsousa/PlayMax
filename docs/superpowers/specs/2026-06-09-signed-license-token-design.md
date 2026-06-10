# Design: Licença com token assinado (Ed25519)

**Data:** 2026-06-09
**Status:** Implementado em 2026-06-10 (branch feat/signed-license-token)

## Problema

Em `src-tauri/src/services/license_service.rs`, `is_locally_valid` confia em valores
gravados em texto puro no SQLite local (`playmax.db`, tabela `app_settings`):
`status`, `expires_at`, `last_validated_at`. Qualquer usuário com um editor de
SQLite escreve `status='active'` e uma data futura e tem o app liberado para
sempre, offline, sem nunca falar com o servidor. Nada prova que os valores
locais vieram do servidor.

## Objetivo

Nenhum estado local libera o app sem uma assinatura criptográfica do servidor.
Adulterar o SQLite deve resultar, no máximo, em "sem licença — ative novamente".

## Decisões tomadas (com o usuário)

1. **Token curto renovado a cada validação**: validade de 48h (a janela offline
   atual), reemitido a cada validação online (que ocorre a cada 6h). A janela
   offline inteira passa a ser assinada pelo servidor.
2. **Migração sem período de transição**: dados antigos em texto puro deixam de
   valer imediatamente. Primeira execução da nova versão revalida online com a
   `license_key` salva; offline nesse momento = tela de ativação até conectar.
3. **Formato próprio Ed25519** (não JWT, não PASETO): payload JSON compacto +
   assinatura, sem dependência nova no servidor e com `ed25519-dalek` no app.

## Arquitetura

Par de chaves Ed25519 gerado uma única vez:

- **Privada**: só no servidor, env var `LICENSE_TOKEN_PRIVATE_KEY` (base64,
  PKCS#8 DER). Nunca commitada.
- **Pública** (32 bytes): constante embutida no binário Rust.

Fluxo:

```
App ──POST /api/license/{activate,validate} {licenseKey, deviceFingerprint}──▶ Portal
Portal ──valida no Supabase──▶ assina token ──▶ {ok, status, expiresAt, valid, token}
App ──verifica assinatura com chave pública──▶ grava license_key + token no SQLite
Liberação do app = SOMENTE token verificado e não expirado
```

## Formato do token

```
base64url(payload_json) "." base64url(assinatura_ed25519_de_payload_json)
```

A assinatura cobre exatamente os bytes do JSON serializado (sem re-serialização
no verificador: o app decodifica o base64url, verifica a assinatura sobre os
bytes crus e só então faz o parse do JSON).

Payload:

```json
{
  "v": 1,
  "licenseKey": "PLAY-XXXX-XXXX-XXXX",
  "fingerprint": "<deviceFingerprint da requisição>",
  "status": "active",
  "licenseExpiresAt": 1767225600,
  "iat": 1749470000,
  "exp": 1749642800
}
```

- `v`: versão do formato (1). Verificador rejeita versões desconhecidas.
- `fingerprint`: amarra o token à máquina; copiar para outro PC falha.
- `status`: `active` ou `trial` (o servidor só emite token para esses estados).
- `licenseExpiresAt`: epoch segundos do vencimento real da licença; `null` para
  licenças sem vencimento. Verificado mesmo dentro da janela de 48h.
- `iat`/`exp`: epoch segundos; `exp = iat + 48h` (constante `GRACE_OFFLINE_SECS`
  existente).

## Servidor (license-portal / Next.js)

### Novo: `src/lib/license-token.ts`

- `signLicenseToken(license: LicenseRow, deviceFingerprint: string): string`
- Usa `crypto.sign(null, data, privateKey)` (Ed25519 nativo do Node) — zero
  dependência nova.
- Lê `LICENSE_TOKEN_PRIVATE_KEY` (base64 → `crypto.createPrivateKey` com PKCS#8
  DER). Ausente/inválida → lança erro (fail closed → 500 nas rotas).

### Novo: `scripts/generate-license-keys.mjs`

Gera o par Ed25519 e imprime:
1. a chave privada em base64 (para o env do Vercel / `.env.local`);
2. a chave pública como array de 32 bytes em sintaxe Rust (para colar na
   constante do app).

### Alterado: rotas `activate` e `validate`

Após sucesso, resposta passa a incluir `token`:
`{ ok: true, ...licensePayload(license), token: signLicenseToken(license, deviceFingerprint) }`.
Campos existentes permanecem (compatibilidade com versões antigas do app, que
ignoram `token`).

## App (src-tauri / Rust)

### Nova dependência

`ed25519-dalek = "2"` (verificação apenas). `base64`, `serde_json`, `chrono` já
existem.

### Novo: `src/services/license_token.rs`

- `pub const LICENSE_PUBLIC_KEY: [u8; 32]` — colada do script de geração.
- `pub struct TokenClaims { v, license_key, fingerprint, status, license_expires_at: Option<i64>, iat, exp }`
- `pub fn verify_token(token: &str, expected_fingerprint: &str) -> Result<TokenClaims>`
  com checagens nesta ordem:
  1. split em `payload.signature`, decode base64url de ambos;
  2. assinatura Ed25519 válida sobre os bytes crus do payload;
  3. parse JSON; `v == 1`;
  4. `fingerprint == expected_fingerprint`;
  5. `now < exp`;
  6. `now >= iat - 300` (tolerância de clock skew de 5 min; relógio anterior à
     emissão = rejeitado — hardening contra rollback trivial de relógio);
  7. `status ∈ {"active", "trial"}`;
  8. `license_expires_at` ausente ou `> now`.
  Qualquer falha → `Err` com motivo (mapeado para `valid: false`, nunca panic).

### Alterado: `src/services/license_service.rs`

- Novo setting `KEY_LICENSE_TOKEN`; settings antigos `KEY_LICENSE_STATUS`,
  `KEY_LICENSE_EXPIRES_AT`, `KEY_LICENSE_LAST_VALIDATED` são removidos do código
  e apagados do banco na inicialização (migração).
- `read_local_state`: lê `license_key` + token; `valid` = `verify_token` ok;
  `status`/`expires_at`/`last_validated_at` do `LicenseState` (exibição no
  frontend) são derivados das claims verificadas (`last_validated_at = iat`).
  Token ausente/inválido → `valid: false`, demais campos `None`.
- `apply_api_response`: exige `token` na resposta; verifica **antes de
  persistir** (resposta sem token válido = erro "Resposta inválida do servidor").
  Persiste apenas `license_key` + token.
- `is_locally_valid` é removida; toda decisão local passa por `verify_token`.
- `ensure_license_valid`: mesma cadência — revalida online quando
  `now - claims.iat >= REVALIDATE_EVERY_SECS` (6h); em falha de rede, segue
  válido enquanto o token verificar (as 48h assinadas); token expirado/ausente
  e sem rede → inválido, mensagem pedindo conexão/ativação.
- `clear_license`: também limpa o token.

### Migração de instalações existentes

Sem token salvo → `valid: false`. O fluxo existente de `ensure_license_valid`
usa a `license_key` antiga (que continua salva) para validar online e obter o
primeiro token. Com internet é transparente; offline mostra a tela de
ativação/licença até conectar.

## Tratamento de erros

| Cenário | Comportamento |
|---|---|
| Token adulterado/assinatura inválida | `valid: false`, mensagem de ativação |
| Token de outra máquina (fingerprint) | `valid: false` |
| Token expirado (>48h offline) | exige validação online |
| Relógio antes de `iat - 5min` | `valid: false` |
| Servidor responde sem `token` | erro "Resposta inválida do servidor"; estado anterior preservado se ainda verificar |
| `LICENSE_TOKEN_PRIVATE_KEY` ausente no servidor | 500 (fail closed), logado |

## Testes

Rust (`license_token.rs` + `license_service.rs`), com par de chaves de teste
gerado no próprio teste (`ed25519-dalek` assina e verifica):

1. token válido → claims corretas, `valid: true`;
2. payload adulterado (1 byte) → falha de assinatura;
3. fingerprint divergente → falha;
4. `exp` no passado → falha;
5. `now < iat - 5min` → falha;
6. `status` editado direto no SQLite (cenário do ataque) → ignorado, app
   continua inválido sem token;
7. `clear_license` remove token;
8. resposta de API sem token → erro, nada persistido.

Portal: teste de roundtrip — `signLicenseToken` produz token cujo payload
decodificado bate com a licença e cuja assinatura verifica com a pública do
par de teste.

## Fora de escopo (decidido pelo usuário)

- Fingerprint de hardware (mantém UUIDv5 de hostname+username por ora).
- `timingSafeEqual`/rate-limiting no portal.
- Criptografia das senhas Xtream e CSP do Tauri.

## Riscos residuais aceitos

- Rollback de relógio do sistema *após* receber um token ainda pode esticar a
  janela de 48h enquanto `now` ficar entre `iat` e `exp`; o teto absoluto é
  `licenseExpiresAt` assinado. Mitigação completa (watermark monotônico)
  considerada e adiada por custo/benefício.
- Engenharia reversa do binário para trocar a chave pública embutida sempre é
  possível em software client-side; o objetivo desta mudança é eliminar a
  adulteração trivial de dados, não DRM absoluto.
