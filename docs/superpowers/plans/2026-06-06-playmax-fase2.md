# Play Max Fase 2 — Catálogo VOD + Home Real

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adicionar suporte M3U+Xtream com catálogo VOD (filmes/séries), páginas de detalhe, Home com carrosséis reais e player VOD com seek.

**Architecture:** Trait `CatalogSource` no Rust com implementações `M3uSource` e `XtreamSource`; `SyncService` grava em SQLite (`movies`, `series`, `episodes`); React consome novos Tauri commands.

**Tech Stack:** Tauri 2, Rust, rusqlite, reqwest, React 19, TypeScript, Tailwind 4, libmpv

**Spec:** `docs/superpowers/specs/2026-06-06-playmax-fase2-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `src-tauri/src/db/migrations.rs` | Migration 002 (profiles + VOD tables) |
| `src-tauri/src/db/models.rs` | Movie, Series, Episode, CatalogItem structs |
| `src-tauri/src/db/movies.rs` | CRUD + list queries |
| `src-tauri/src/db/series.rs` | CRUD + episodes |
| `src-tauri/src/parsers/m3u/classifier.rs` | Group → live/movie/series |
| `src-tauri/src/parsers/m3u/series_parser.rs` | S01E05 / 1x05 parsing |
| `src-tauri/src/parsers/xtream/client.rs` | HTTP client player_api.php |
| `src-tauri/src/parsers/xtream/types.rs` | Deserialize JSON Xtream |
| `src-tauri/src/services/catalog_source.rs` | Trait + dispatch |
| `src-tauri/src/services/sync_service.rs` | Refactor: M3U + Xtream paths |
| `src-tauri/src/services/profile_service.rs` | add_xtream_profile |
| `src-tauri/src/commands/mod.rs` | Novos commands |
| `src/lib/types.ts` | Movie, Series, Episode, CatalogItem |
| `src/lib/api.ts` | Wrappers invoke |
| `src/hooks/useMovies.ts`, `useSeries.ts`, `useCatalog.ts` | Data hooks |
| `src/pages/MoviesPage.tsx` | Grade filmes |
| `src/pages/SeriesPage.tsx` | Grade séries |
| `src/pages/MovieDetailPage.tsx` | Detalhe filme |
| `src/pages/SeriesDetailPage.tsx` | Detalhe série |
| `src/pages/OnboardingPage.tsx` | Toggle M3U/Xtream |
| `src/pages/HomePage.tsx` | Carrosséis reais |
| `src/hooks/usePlayer.ts` | VOD seek props |
| `src/components/player/PlayerOverlay.tsx` | Seek bar |
| `src/App.tsx` | Novas rotas |

---

### Task 1: Migration 002 + models

**Files:**
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/models.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/movies.rs`
- Create: `src-tauri/src/db/series.rs`

- [ ] **Step 1: Adicionar migration 002 em `migrations.rs`**

```rust
const MIGRATION_002: &str = r#"
-- Recreate profiles to allow xtream (SQLite lacks easy ALTER CHECK)
CREATE TABLE IF NOT EXISTS profiles_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK(type IN ('m3u', 'xtream')),
  url TEXT,
  file_path TEXT,
  username TEXT,
  password TEXT,
  last_sync INTEGER
);
INSERT INTO profiles_new (id, name, type, url, file_path, username, password, last_sync)
  SELECT id, name, type, url, file_path, NULL, NULL, last_sync FROM profiles;
DROP TABLE profiles;
ALTER TABLE profiles_new RENAME TO profiles;

CREATE TABLE IF NOT EXISTS movies (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  poster TEXT,
  backdrop TEXT,
  plot TEXT,
  genres TEXT,
  rating TEXT,
  stream_url TEXT NOT NULL,
  category TEXT,
  added_at INTEGER NOT NULL,
  sort_order INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS series (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  poster TEXT,
  backdrop TEXT,
  plot TEXT,
  genres TEXT,
  rating TEXT,
  category TEXT,
  added_at INTEGER NOT NULL,
  sort_order INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS episodes (
  id TEXT PRIMARY KEY,
  series_id TEXT NOT NULL REFERENCES series(id) ON DELETE CASCADE,
  season INTEGER NOT NULL,
  episode INTEGER NOT NULL,
  title TEXT NOT NULL,
  plot TEXT,
  stream_url TEXT NOT NULL,
  duration INTEGER,
  added_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_movies_profile_added ON movies(profile_id, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_series_profile_added ON series(profile_id, added_at DESC);
CREATE INDEX IF NOT EXISTS idx_episodes_series ON episodes(series_id, season, episode);
"#;

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(MIGRATION_001)?;
    conn.execute_batch(MIGRATION_002)?;
    Ok(())
}
```

- [ ] **Step 2: Adicionar structs em `models.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Movie {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub plot: Option<String>,
    pub genres: Option<String>,
    pub rating: Option<String>,
    pub stream_url: String,
    pub category: Option<String>,
    pub added_at: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub plot: Option<String>,
    pub genres: Option<String>,
    pub rating: Option<String>,
    pub category: Option<String>,
    pub added_at: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: String,
    pub series_id: String,
    pub season: i32,
    pub episode: i32,
    pub title: String,
    pub plot: Option<String>,
    pub stream_url: String,
    pub duration: Option<i32>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoviesPage {
    pub items: Vec<Movie>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPage {
    pub items: Vec<Series>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetail {
    pub series: Series,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub id: String,
    pub item_type: String, // "movie" | "series"
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub added_at: i64,
}
```

Atualizar `Profile` com `username: Option<String>`, `password: Option<String>` (password nunca serializado para frontend — usar `#[serde(skip_serializing)]` no campo password ou omitir no list_profiles).

- [ ] **Step 3: Implementar `db/movies.rs` e `db/series.rs`**

Funções mínimas:
- `delete_by_profile(conn, profile_id)`
- `insert_batch(conn, items)`
- `list_movies(conn, profile_id, category, search, offset, limit) -> MoviesPage`
- `get_movie(conn, id) -> Option<Movie>`
- `list_series(...)`, `get_series`, `list_episodes_for_series`
- `list_recent(conn, profile_id, limit) -> Vec<CatalogItem>` — UNION movies+series ORDER BY added_at
- `list_releases(conn, profile_id, limit, since_ts) -> Vec<CatalogItem>`

- [ ] **Step 4: Teste Rust do repositório**

```bash
cd src-tauri && cargo test db::movies -- --nocapture
```

Expected: PASS (testes com DB in-memory)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/
git commit -m "feat(db): add VOD tables and repositories for phase 2"
```

---

### Task 2: M3U classifier

**Files:**
- Create: `src-tauri/src/parsers/m3u/classifier.rs`
- Create: `src-tauri/src/parsers/m3u/series_parser.rs`
- Modify: `src-tauri/src/parsers/m3u.rs` (re-export ou split)
- Modify: `src-tauri/src/parsers/mod.rs`

- [ ] **Step 1: Teste falhando do classifier**

```rust
// src-tauri/src/parsers/m3u/classifier.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_movie_groups() {
        assert_eq!(classify_group("Filmes 4K"), ContentKind::Movie);
        assert_eq!(classify_group("VOD Movies"), ContentKind::Movie);
    }

    #[test]
    fn classifies_series_groups() {
        assert_eq!(classify_group("Séries Netflix"), ContentKind::Series);
        assert_eq!(classify_group("TV Shows"), ContentKind::Series);
    }

    #[test]
    fn defaults_to_live() {
        assert_eq!(classify_group("Notícias"), ContentKind::Live);
    }
}
```

- [ ] **Step 2: Implementar `classify_group`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Live,
    Movie,
    Series,
}

pub fn classify_group(group: &str) -> ContentKind {
    let g = group.to_lowercase();
    let movie_re = regex::Lazy::new(|| regex::Regex::new(
        r"(?i)(filmes?|movies?|vod|cine|cinema|4k\s*movies?)"
    ).unwrap());
    let series_re = regex::Lazy::new(|| regex::Regex::new(
        r"(?i)(s[eé]ries?|tv\s*shows?|novelas?|animes?)"
    ).unwrap());
    if movie_re.is_match(&g) { return ContentKind::Movie; }
    if series_re.is_match(&g) { return ContentKind::Series; }
    ContentKind::Live
}
```

Adicionar `regex = "1"` em `Cargo.toml`.

- [ ] **Step 3: Implementar `series_parser.rs`**

```rust
pub struct ParsedEpisode {
    pub series_name: String,
    pub season: i32,
    pub episode: i32,
    pub episode_title: String,
}

pub fn parse_series_title(name: &str) -> Option<ParsedEpisode> {
    // "Breaking Bad S01E05" or "Breaking Bad 1x05 - Pilot"
    let re = regex::Regex::new(
        r"^(.+?)\s+(?:S(\d{1,2})E(\d{1,2})|(\d{1,2})x(\d{1,2}))\s*(?:[-–]\s*(.+))?$"
    ).ok()?;
    let caps = re.captures(name)?;
    // ... extract groups
}
```

- [ ] **Step 4: Rodar testes**

```bash
cd src-tauri && cargo test classifier -- --nocapture
cd src-tauri && cargo test series_parser -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/parsers/m3u/ src-tauri/Cargo.toml
git commit -m "feat(m3u): classify groups and parse series episode titles"
```

---

### Task 3: Xtream client

**Files:**
- Create: `src-tauri/src/parsers/xtream/mod.rs`
- Create: `src-tauri/src/parsers/xtream/client.rs`
- Create: `src-tauri/src/parsers/xtream/types.rs`
- Modify: `src-tauri/src/parsers/mod.rs`

- [ ] **Step 1: Tipos JSON Xtream em `types.rs`**

```rust
#[derive(Debug, Deserialize)]
pub struct XtreamVodStream {
    pub stream_id: i64,
    pub name: String,
    pub stream_icon: Option<String>,
    pub rating: Option<String>,
    pub category_id: Option<String>,
    pub added: Option<String>,
    pub container_extension: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct XtreamSeries {
    pub series_id: i64,
    pub name: String,
    pub cover: Option<String>,
    pub plot: Option<String>,
    pub genre: Option<String>,
    pub rating: Option<String>,
    pub category_id: Option<String>,
    pub last_modified: Option<String>,
}
```

- [ ] **Step 2: Cliente HTTP**

```rust
pub struct XtreamClient {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl XtreamClient {
    pub fn new(base_url: String, username: String, password: String) -> Self { ... }

    fn api_url(&self, action: &str) -> String {
        format!(
            "{}/player_api.php?username={}&password={}&action={}",
            self.base_url.trim_end_matches('/'),
            urlencoding::encode(&self.username),
            urlencoding::encode(&self.password),
            action
        )
    }

    pub async fn get_vod_streams(&self) -> AppResult<Vec<XtreamVodStream>> { ... }
    pub async fn get_series(&self) -> AppResult<Vec<XtreamSeries>> { ... }
    pub async fn get_series_info(&self, series_id: i64) -> AppResult<XtreamSeriesInfo> { ... }
    pub async fn get_live_streams(&self) -> AppResult<Vec<XtreamLiveStream>> { ... }

    pub fn build_vod_url(&self, stream_id: i64, extension: &str) -> String {
        format!(
            "{}/movie/{}/{}/{}.{}",
            self.base_url.trim_end_matches('/'),
            self.username, self.password, stream_id, extension
        )
    }

    pub fn build_series_url(&self, stream_id: i64, extension: &str) -> String { ... }
}
```

Adicionar `urlencoding = "2"` em `Cargo.toml`.

- [ ] **Step 3: Teste com fixture JSON**

Criar `src-tauri/src/parsers/xtream/fixtures/vod_streams.json` (snippet anonimizado) e teste de deserialize.

```bash
cd src-tauri && cargo test xtream -- --nocapture
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parsers/xtream/ src-tauri/Cargo.toml
git commit -m "feat(xtream): add API client for VOD and series"
```

---

### Task 4: Sync service refactor

**Files:**
- Create: `src-tauri/src/services/catalog_source.rs`
- Modify: `src-tauri/src/services/sync_service.rs`
- Modify: `src-tauri/src/services/profile_service.rs`
- Modify: `src-tauri/src/parsers/m3u.rs` (retornar `ParsedCatalog` em vez de só channels)

- [ ] **Step 1: Trait CatalogSyncResult**

```rust
pub struct CatalogSyncResult {
    pub channels: Vec<Channel>,
    pub movies: Vec<Movie>,
    pub series: Vec<Series>,
    pub episodes: Vec<Episode>,
}
```

- [ ] **Step 2: `sync_m3u_profile`**

Após parse M3U, para cada entrada:
- `classify_group(group)` → Live: channels, Movie: movies, Series: parse episode → series map

Agrupar séries M3U em `HashMap<String, Series>` + episódios.

- [ ] **Step 3: `sync_xtream_profile`**

1. `get_live_streams` → channels
2. `get_vod_streams` → movies (com `build_vod_url`)
3. `get_series` → para cada série `get_series_info` → episodes
4. Emitir progresso: 10% live, 40% vod, 70% series

- [ ] **Step 4: `sync_profile` dispatch**

```rust
match profile.profile_type.as_str() {
    "m3u" => sync_m3u_profile(...).await,
    "xtream" => sync_xtream_profile(...).await,
    _ => Err(...),
}
```

Delete movies/series/episodes do profile antes de insert (cascade episodes).

- [ ] **Step 5: `add_xtream_profile` em profile_service**

```rust
pub fn add_xtream_profile(
    db: &SharedDb,
    name: String,
    url: String,
    username: String,
    password: String,
) -> AppResult<Profile>
```

- [ ] **Step 6: Teste integração sync M3U com fixture**

Usar `tests/fixtures/` com entradas Filmes e Séries; verificar contagem em DB in-memory.

```bash
cd src-tauri && cargo test sync -- --nocapture
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/services/ src-tauri/src/parsers/
git commit -m "feat(sync): support M3U VOD classification and Xtream catalog sync"
```

---

### Task 5: Tauri commands + TypeScript types

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Commands Rust**

```rust
#[tauri::command]
pub fn add_xtream_profile(
    state: State<'_, SharedDb>,
    name: String,
    url: String,
    username: String,
    password: String,
) -> AppResult<Profile> { ... }

#[tauri::command]
pub fn list_movies(
    state: State<'_, SharedDb>,
    profile_id: String,
    category: Option<String>,
    search: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> AppResult<MoviesPage> { ... }

#[tauri::command]
pub fn get_movie(state: State<'_, SharedDb>, id: String) -> AppResult<Movie> { ... }

#[tauri::command]
pub fn list_series(...) -> AppResult<SeriesPage> { ... }

#[tauri::command]
pub fn get_series_detail(state: State<'_, SharedDb>, id: String) -> AppResult<SeriesDetail> { ... }

#[tauri::command]
pub fn list_recent(state: State<'_, SharedDb>, profile_id: String, limit: Option<i64>) -> AppResult<Vec<CatalogItem>> { ... }

#[tauri::command]
pub fn list_releases(state: State<'_, SharedDb>, profile_id: String, limit: Option<i64>) -> AppResult<Vec<CatalogItem>> { ... }
```

Registrar todos em `lib.rs` `generate_handler!`.

- [ ] **Step 2: TypeScript types + api.ts wrappers**

Espelhar structs Rust com `camelCase`.

- [ ] **Step 3: Build**

```bash
npm run build
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/ src-tauri/src/lib.rs src/lib/
git commit -m "feat(api): expose VOD catalog commands to frontend"
```

---

### Task 6: Onboarding dual

**Files:**
- Modify: `src/pages/OnboardingPage.tsx`

- [ ] **Step 1: Toggle M3U | Xtream**

```tsx
type SourceType = "m3u" | "xtream";
const [sourceType, setSourceType] = useState<SourceType>("m3u");
```

Botões segmentados no topo do form.

- [ ] **Step 2: Campos Xtream**

Quando `sourceType === "xtream"`: mostrar URL + username + password; esconder file picker.

- [ ] **Step 3: Submit**

```tsx
const profile = sourceType === "m3u"
  ? await api.addM3uProfile(name, url || undefined, filePath ?? undefined)
  : await api.addXtreamProfile(name, url, username, password);
await api.syncProfileBlocking(profile.id);
navigate("/home", { replace: true });
```

- [ ] **Step 4: Testar manualmente**

```bash
npm run tauri:dev
```

- [ ] **Step 5: Commit**

```bash
git add src/pages/OnboardingPage.tsx
git commit -m "feat(onboarding): add Xtream Codes source option"
```

---

### Task 7: Páginas Filmes e Séries

**Files:**
- Create: `src/pages/MoviesPage.tsx`
- Create: `src/pages/SeriesPage.tsx`
- Create: `src/pages/MovieDetailPage.tsx`
- Create: `src/pages/SeriesDetailPage.tsx`
- Create: `src/hooks/useMovies.ts`, `useSeries.ts`
- Create: `src/components/vod/VodGrid.tsx`, `PosterCard.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: `VodGrid` virtualizado**

Reutilizar padrão de `@tanstack/react-virtual` como `ChannelList`; variant `poster` (aspect 2/3).

- [ ] **Step 2: `MoviesPage`**

TopBar + grid + filtro categorias (distinct `category` from movies).

- [ ] **Step 3: `MovieDetailPage`**

Backdrop hero, plot, botão "Assistir" → navega para `/live` ou player inline com `streamUrl`.

Rota: `/movies/:id`

- [ ] **Step 4: `SeriesPage` + `SeriesDetailPage`**

Grade de séries; detalhe agrupa episódios por temporada:

```tsx
const bySeason = episodes.reduce<Record<number, Episode[]>>(...)
```

Rota: `/series/:id`

- [ ] **Step 5: Atualizar `App.tsx`**

```tsx
<Route path="/movies" element={<MoviesPage />} />
<Route path="/movies/:id" element={<MovieDetailPage />} />
<Route path="/series" element={<SeriesPage />} />
<Route path="/series/:id" element={<SeriesDetailPage />} />
```

Remover `PlaceholderPage` para movies/series.

- [ ] **Step 6: Build + commit**

```bash
npm run build
git add src/pages/ src/components/vod/ src/hooks/ src/App.tsx
git commit -m "feat(ui): add Movies and Series pages with detail views"
```

---

### Task 8: Home com carrosséis reais

**Files:**
- Modify: `src/pages/HomePage.tsx`
- Modify: `src/components/home/HeroBanner.tsx`
- Modify: `src/components/home/ContentCarousel.tsx`
- Create: `src/hooks/useCatalog.ts`

- [ ] **Step 1: Hook `useCatalog`**

```ts
export function useCatalog(profileId: string | null) {
  const [recent, setRecent] = useState<CatalogItem[]>([]);
  const [releases, setReleases] = useState<CatalogItem[]>([]);
  // fetch list_recent + list_releases
}
```

- [ ] **Step 2: Hero dinâmico**

`HeroBanner` recebe `item: CatalogItem | null` — usa `backdrop` ou `poster`; título + botão "Assistir" linkando para `/movies/:id` ou `/series/:id`.

- [ ] **Step 3: Carrosséis reais**

Substituir `RELEASES` mock por `releases`; adicionar carrossel "Adicionados recentemente" com `recent`.

Manter "Continuar assistindo" mock com comentário `{/* Phase 3 */}`.

- [ ] **Step 4: Commit**

```bash
git add src/pages/HomePage.tsx src/components/home/ src/hooks/useCatalog.ts
git commit -m "feat(home): wire recent and releases carousels to catalog"
```

---

### Task 9: Player VOD seek

**Files:**
- Modify: `src/hooks/usePlayer.ts`
- Modify: `src/components/player/PlayerOverlay.tsx`
- Modify: `src/lib/types.ts` (PlayableItem union)

- [ ] **Step 1: Tipo unificado de reprodução**

```ts
export type PlayableItem =
  | { kind: "live"; channel: Channel }
  | { kind: "vod"; id: string; name: string; streamUrl: string };
```

- [ ] **Step 2: Observar time-pos e duration**

```ts
const OBSERVED_PROPERTIES = [
  ["pause", "flag"],
  ["volume", "int64"],
  ["time-pos", "double"],
  ["duration", "double"],
  // ...
] as const;
```

- [ ] **Step 3: Seek bar no overlay (só VOD)**

```tsx
{playable?.kind === "vod" && (
  <input
    type="range"
    min={0}
    max={duration}
    value={position}
    onChange={(e) => seek(Number(e.target.value))}
  />
)}
```

```ts
async function seek(seconds: number) {
  await setProperty("time-pos", seconds);
}
```

- [ ] **Step 4: Wire detalhe → player**

`MovieDetailPage` e episódio em `SeriesDetailPage` chamam `playVod(item)`.

- [ ] **Step 5: Testar VOD**

```bash
npm run tauri:dev
```

Reproduzir filme; arrastar seek bar.

- [ ] **Step 6: Commit**

```bash
git add src/hooks/usePlayer.ts src/components/player/
git commit -m "feat(player): add VOD seek bar for movies and episodes"
```

---

### Task 10: README + verificação final

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Atualizar README**

Adicionar Fase 2 features, onboarding Xtream, rotas novas.

- [ ] **Step 2: Verificação completa**

```bash
npm run build
cd src-tauri && cargo test
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document phase 2 catalog features"
```

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Migration profiles xtream | Task 1 |
| movies/series/episodes tables | Task 1 |
| M3U classifier | Task 2 |
| Xtream client | Task 3 |
| Sync dual source | Task 4 |
| add_xtream_profile | Task 4, 5 |
| list_movies/series/detail | Task 5 |
| list_recent/releases | Task 5 |
| Onboarding dual | Task 6 |
| Movies/Series pages | Task 7 |
| Home carrosséis reais | Task 8 |
| Player VOD seek | Task 9 |
| Critérios de aceite | Task 10 |

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-06-playmax-fase2.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks
2. **Inline Execution** — implement task-by-task in this session with checkpoints

Which approach?
