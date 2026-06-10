# Play Max Fase 3 — Detalhes, Favoritos, Busca

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Completar detalhe de séries (temporadas, sync on-demand, autoplay), favoritos com Minha Lista, e busca unificada.

**Architecture:** `series_sync` busca episódios Xtream quando DB vazio; tabela `favorites`; `search_catalog` com UNION LIKE; React estende `VodPlayInfo` com fila autoplay.

**Tech Stack:** Tauri 2, Rust, rusqlite, React 19, TypeScript, Tailwind 4, libmpv

**Spec:** `docs/superpowers/specs/2026-06-06-playmax-fase3-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `src-tauri/src/db/migrations.rs` | Migration 005 favorites |
| `src-tauri/src/db/favorites.rs` | CRUD + list enriched |
| `src-tauri/src/db/search.rs` | `search_catalog` UNION |
| `src-tauri/src/services/series_sync.rs` | Xtream on-demand episodes |
| `src-tauri/src/db/series.rs` | delete/replace episodes, update series |
| `src-tauri/src/commands/mod.rs` | Novos commands + async detail |
| `src-tauri/permissions/app-commands.toml` | ACL novos commands |
| `src/lib/types.ts` | Favorite, SearchResult, AutoplayEpisode |
| `src/lib/api.ts` | Wrappers invoke |
| `src/hooks/useFavorites.ts` | Favoritos + toggle |
| `src/hooks/useSearch.ts` | Busca debounced |
| `src/pages/SeriesDetailPage.tsx` | Tabs temporada, sync, play |
| `src/pages/MovieDetailPage.tsx` | FavoriteToggle |
| `src/pages/MyListPage.tsx` | Grid favoritos |
| `src/pages/SearchPage.tsx` | Resultados agrupados |
| `src/pages/HomePage.tsx` | TopBar → /search |
| `src/hooks/usePlayer.ts` | Autoplay queue |
| `src/components/favorites/FavoriteToggle.tsx` | Heart button |
| `src/App.tsx` | Rotas /list, /search |

---

### Task 1: Migration 005 + favorites repo

**Files:**
- Modify: `src-tauri/src/db/migrations.rs`
- Create: `src-tauri/src/db/favorites.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Adicionar migration 005**

```sql
CREATE TABLE IF NOT EXISTS favorites (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK(item_type IN ('movie', 'series', 'channel')),
  item_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(profile_id, item_type, item_id)
);
CREATE INDEX IF NOT EXISTS idx_favorites_profile ON favorites(profile_id, created_at DESC);
```

- [ ] **Step 2: Structs `Favorite`, `FavoriteItem` em models.rs**

- [ ] **Step 3: Implementar `favorites.rs`**

Funções: `add`, `remove`, `is_favorite`, `list_favorites` (JOIN movies/series/channels para nome/poster).

- [ ] **Step 4: Testes Rust**

```bash
cd src-tauri && cargo test db::favorites -- --nocapture
```

Expected: PASS

---

### Task 2: Series sync on-demand

**Files:**
- Create: `src-tauri/src/services/series_sync.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/db/series.rs`

- [ ] **Step 1: Helpers em series.rs**

`delete_episodes_for_series`, `update_series_metadata`, `replace_episodes_for_series`.

- [ ] **Step 2: `sync_episodes_on_demand(db, series_id)`**

Xtream only; usa `sort_order` como API id; chama `map_xtream_series` parcial.

- [ ] **Step 3: Tornar `get_series_detail` async**

Dispara sync se episódios vazios; retorna detail atualizado.

- [ ] **Step 4: Teste com DB in-memory + mock client fixture**

```bash
cd src-tauri && cargo test series_sync -- --nocapture
```

---

### Task 3: Favorites commands + ACL

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/permissions/app-commands.toml`

- [ ] **Step 1: Commands**

`add_favorite`, `remove_favorite`, `list_favorites`, `is_favorite`

- [ ] **Step 2: Registrar + ACL**

- [ ] **Step 3: TypeScript api.ts + types.ts**

---

### Task 4: Series detail UI + autoplay

**Files:**
- Modify: `src/pages/SeriesDetailPage.tsx`
- Modify: `src/lib/types.ts` (`AutoplayEpisode`, `VodPlayInfo.autoplayQueue`)
- Modify: `src/hooks/usePlayer.ts`

- [ ] **Step 1: Season pills + episódios filtrados**

- [ ] **Step 2: Loading sync ("Buscando episódios...")**

- [ ] **Step 3: Play com autoplayQueue dos episódios restantes**

- [ ] **Step 4: usePlayer end-file eof → play next da fila**

---

### Task 5: Minha Lista + FavoriteToggle

**Files:**
- Create: `src/pages/MyListPage.tsx`
- Create: `src/components/favorites/FavoriteToggle.tsx`
- Create: `src/hooks/useFavorites.ts`
- Modify: `src/pages/MovieDetailPage.tsx`, `SeriesDetailPage.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: FavoriteToggle component**

- [ ] **Step 2: MyListPage grid com PosterCard**

- [ ] **Step 3: Wire /list route (substituir PlaceholderPage)**

---

### Task 6: Unified search

**Files:**
- Create: `src-tauri/src/db/search.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Create: `src/pages/SearchPage.tsx`
- Create: `src/hooks/useSearch.ts`
- Modify: `src/pages/HomePage.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: `search_catalog(conn, profile_id, query, limit)`**

UNION LIKE em channels.name, movies.name, series.name.

- [ ] **Step 2: SearchPage com seções agrupadas**

- [ ] **Step 3: Home TopBar submit → `/search?q=`**

- [ ] **Step 4: Teste Rust search**

```bash
cd src-tauri && cargo test db::search -- --nocapture
```

---

### Task 7: Verificação final

- [ ] **Step 1: Build + testes**

```bash
npm run build
cd src-tauri && cargo test
```

Expected: PASS

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Series detail tabs + play | Task 4 |
| Xtream sync on-demand | Task 2 |
| Autoplay next | Task 4 |
| Migration favorites | Task 1 |
| Favorites commands | Task 3 |
| Minha Lista /list | Task 5 |
| Heart toggle | Task 5 |
| search_catalog | Task 6 |
| Home search | Task 6 |

## Execution handoff

Plan saved to `docs/superpowers/plans/2026-06-06-playmax-fase3.md`.
