# Play Max Fase 6 — Sync Background + FTS5 + TMDB

**Goal:** Sync automática em background, busca FTS5, enriquecimento TMDB opcional.

**Spec:** `docs/superpowers/specs/2026-06-06-playmax-fase6-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `src-tauri/src/db/migrations.rs` | Migration 007 settings + FTS |
| `src-tauri/src/db/settings.rs` | CRUD app_settings |
| `src-tauri/src/db/catalog_fts.rs` | Reindex + delete FTS |
| `src-tauri/src/db/search.rs` | FTS MATCH + LIKE fallback |
| `src-tauri/src/services/sync_guard.rs` | Mutex sync em andamento |
| `src-tauri/src/services/background_sync.rs` | Intervalo 30 min |
| `src-tauri/src/services/tmdb_service.rs` | Enriquecimento TMDB v3 |
| `src-tauri/src/services/catalog_sync.rs` | Hook reindex FTS |
| `src-tauri/src/commands/mod.rs` | Settings + TMDB em get_movie/detail |
| `src/pages/SettingsPage.tsx` | Toggle + TMDB + última sync |

---

### Task 1: Migration 007 + settings

- [x] `app_settings` table
- [x] `catalog_fts` virtual table
- [x] `db/settings.rs`

### Task 2: FTS5 search

- [x] `catalog_fts::reindex_profile` em `persist_catalog`
- [x] `search_catalog` FTS + fallback LIKE
- [x] Testes Rust

### Task 3: Background sync

- [x] `SyncGuard` compartilhado
- [x] `start_background_sync` no setup
- [x] Commands `get_app_settings` / `update_app_settings`

### Task 4: TMDB

- [x] `tmdb_service` search movie/tv
- [x] Enrich em `get_movie` / `get_series_detail`
- [x] UI chave TMDB em Settings

### Task 5: Frontend Settings

- [x] Toggle auto-sync
- [x] Última sincronização
- [x] Campo TMDB API key

### Task 6: Verificação

- [x] `cargo test`
- [x] `npm run build`

---

## Como testar

### Sync automática

1. Abra **Configurações** → ative **Atualizar catálogo automaticamente**.
2. Observe **Última sincronização** após sync manual ou automática.
3. Com app aberto, aguarde 30 min (ou reduza intervalo em dev) — sync dispara sem clique.
4. Inicie sync manual durante automática → manual mostra erro; background ignora.

### Busca FTS5

1. Sincronize uma lista com canais/filmes/séries.
2. Abra **Busca** e pesquise termos parciais (ex.: "glob").
3. Resultados devem incluir canais, filmes e séries.
4. `cargo test search_` valida FTS e fallback.

### TMDB

1. Obtenha chave em [themoviedb.org](https://www.themoviedb.org/settings/api).
2. Cole em **Configurações → Chave da API TMDB** → Salvar.
3. Abra detalhe de filme/série sem pôster da fonte IPTV → metadados preenchidos.
4. Sem chave → comportamento inalterado.

### PiP

Documentado como deferido — sem implementação nesta fase.
