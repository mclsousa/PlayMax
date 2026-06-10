# Play Max — Fase 6: Sync Background + FTS5 + TMDB

**Data:** 2026-06-06  
**Status:** Implementado

## Escopo

| Item | Status |
|------|--------|
| Sync em background (30 min) | **Implementado** |
| Toggle settings + última sync | **Implementado** |
| Busca FTS5 (`catalog_fts`) | **Implementado** |
| TMDB enrichment (chave opcional) | **Implementado** |
| PiP nativo Windows | **Deferido** |

## A. Sync em background

### Persistência

Tabela `app_settings` (migration 007):

| Chave | Tipo | Descrição |
|-------|------|-----------|
| `auto_sync_enabled` | bool | Toggle "Atualizar catálogo automaticamente" |
| `last_background_sync` | i64 | Timestamp da última sync automática |
| `tmdb_api_key` | string | Chave TMDB (local, nunca commitada) |

### Backend

```
lib.rs setup
  └─ start_background_sync (tokio interval 30 min)
        ├─ auto_sync_enabled?
        ├─ SyncGuard livre?
        └─ sync_profile (primeira lista = ativa)
              └─ emite sync-progress / sync-error
```

- `SyncGuard` impede sync manual e automática simultâneas.
- Sync manual retorna erro se já em andamento; background apenas ignora.

### Frontend

`SettingsPage`: toggle, última sincronização, seção TMDB, listas M3U.

## B. Busca FTS5

### Migration 007

```sql
CREATE VIRTUAL TABLE catalog_fts USING fts5(
  item_id UNINDEXED,
  profile_id UNINDEXED,
  item_type UNINDEXED,
  poster UNINDEXED,
  name,
  category,
  tokenize='unicode61 remove_diacritics 2'
);
```

### Fluxo

```
persist_catalog (catalog_sync.rs)
  └─ catalog_fts::reindex_profile (full replace por perfil)

search_catalog (search.rs)
  ├─ FTS MATCH + bm25 (se índice populado)
  └─ fallback LIKE (legado)
```

## C. TMDB enrichment

- Commands `get_movie` / `get_series_detail` enriquecem on-demand quando campos faltam.
- API v3: `/search/movie` e `/search/tv` por título + ano extraído do nome.
- Sem chave → retorno inalterado (graceful skip).

## D. PiP — deferido

Mesma conclusão das Fases 4–5: mpv via HWND não expõe PiP. Alternativa viável seria janela Tauri secundária always-on-top com segundo viewport mpv — estimativa > 3h, fora desta entrega.

## Commands novos

| Command | Descrição |
|---------|-----------|
| `get_app_settings` | Lê preferências locais |
| `update_app_settings` | Atualiza toggle sync e/ou chave TMDB |

## Critérios de aceite

- [x] Toggle persiste em SQLite `app_settings`
- [x] Background sync a cada 30 min com app aberto
- [x] Skip se sync manual em andamento
- [x] FTS5 populado em `persist_catalog`
- [x] `search_catalog` usa FTS com fallback LIKE
- [x] TMDB opcional em detalhe filme/série
- [x] PiP documentado como deferido
- [x] `cargo test` + `npm run build` passam
