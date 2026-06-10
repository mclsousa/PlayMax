# Play Max Fase 5 — EPG + Polish

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** EPG básico Xtream on-demand com cache SQLite; UI "Agora/Depois" no ao vivo; polish favorito em grid VOD.

**Architecture:** `epg_service` + migration 006 + `get_channel_epg` command; frontend `useChannelEpg` + `ChannelEpgDisplay`.

**Tech Stack:** Tauri 2, Rust, React 19, SQLite, Xtream `get_short_epg`

**Spec:** `docs/superpowers/specs/2026-06-06-playmax-fase5-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `src-tauri/src/parsers/xtream/types.rs` | `XtreamShortEpgResponse`, decode base64 |
| `src-tauri/src/parsers/xtream/client.rs` | `get_short_epg`, `short_epg_url` |
| `src-tauri/src/parsers/xtream/fixtures/short_epg.json` | Fixture testes |
| `src-tauri/src/db/migrations.rs` | Migration 006 `epg_programs` |
| `src-tauri/src/db/epg.rs` | Cache CRUD + TTL |
| `src-tauri/src/db/channels.rs` | `get_channel` |
| `src-tauri/src/services/epg_service.rs` | Orquestração fetch + cache |
| `src-tauri/src/commands/mod.rs` | `get_channel_epg` |
| `src-tauri/permissions/app-commands.toml` | ACL |
| `src/hooks/useChannelEpg.ts` | Fetch EPG no frontend |
| `src/components/live/ChannelEpgDisplay.tsx` | UI pt-BR Agora/Depois |
| `src/components/live/ChannelCard.tsx` | EPG compacto no ativo |
| `src/pages/LivePage.tsx` | Wire hook + overlay |
| `src/components/vod/PosterCard.tsx` | FavoriteToggle opcional |
| `src/components/vod/VodGrid.tsx` | Repassa favorito |

---

### Task 1: Xtream EPG parser + client

**Files:**
- Modify: `types.rs`, `client.rs`, `mod.rs`
- Create: `fixtures/short_epg.json`

- [x] **Step 1:** Tipos `XtreamShortEpgResponse` / `XtreamEpgListing`
- [x] **Step 2:** `decode_xtream_epg_text` (base64)
- [x] **Step 3:** `get_short_epg(stream_id, limit)`
- [x] **Step 4:** Testes fixture + URL builder

---

### Task 2: Migration + cache DB

**Files:**
- Modify: `migrations.rs`, `models.rs`, `db/mod.rs`
- Create: `db/epg.rs`

- [x] **Step 1:** Migration 006 `epg_programs`
- [x] **Step 2:** `cache_fresh`, `list_cached_programs`, `replace_channel_programs`
- [x] **Step 3:** Modelos `EpgProgram`, `ChannelEpg`
- [x] **Step 4:** Testes TTL e replace

---

### Task 3: EPG service + command

**Files:**
- Create: `services/epg_service.rs`
- Modify: `services/mod.rs`, `commands/mod.rs`, `lib.rs`, ACL

- [x] **Step 1:** `get_channel_epg` — M3U unavailable, Xtream fetch
- [x] **Step 2:** `parse_trailing_stream_id` público em `catalog_sync`
- [x] **Step 3:** Command async + ACL
- [x] **Step 4:** Testes service (fixture map, M3U unavailable)

---

### Task 4: Frontend EPG UI

**Files:**
- Create: `useChannelEpg.ts`, `ChannelEpgDisplay.tsx`
- Modify: `api.ts`, `types.ts`, `LivePage.tsx`, `ChannelCard.tsx`, `ChannelList.tsx`, `PlayerOverlay.tsx`

- [x] **Step 1:** API + types
- [x] **Step 2:** Hook on channel change
- [x] **Step 3:** Overlay + lista compacta
- [x] **Step 4:** Strings pt-BR

---

### Task 5: Polish PosterCard favorito

**Files:**
- Modify: `PosterCard.tsx`, `VodGrid.tsx`

- [x] **Step 1:** Props `favoriteItemType` / `favoriteItemId`
- [x] **Step 2:** `FavoriteToggle` hover no poster

---

### Task 6: Verificação

- [x] **Step 1:** `cargo test` — parsers, epg db, epg_service (87 passed)
- [x] **Step 2:** `npm run build`
- [ ] **Step 3:** Manual: perfil Xtream → Ao Vivo → play canal → ver EPG

---

## Como testar EPG manualmente

1. `npm run tauri:dev`
2. Adicionar perfil **Xtream** e sincronizar
3. Abrir **Ao Vivo** → selecionar categoria → reproduzir canal
4. Verificar abaixo do nome no player: **Agora:** + **Depois:**
5. Canal ativo na lista lateral deve mostrar linha compacta EPG
6. Perfil **M3U**: deve exibir **EPG indisponível** (sem crash)

## Deferidos (Fase 6)

- Sync background 30min + settings toggle
- Busca FTS5
- PiP / TMDB
