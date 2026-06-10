# Play Max — Fase 3: Detalhes, Favoritos, Busca

**Data:** 2026-06-06  
**Status:** Aprovado pelo usuário

## Decisões de escopo

| Decisão | Escolha |
|---------|---------|
| Metadados | Somente da lista / Xtream API — sem TMDB |
| Episódios Xtream | Sync completo no full sync; `get_series_info` on-demand se episódios ausentes no DB |
| Favoritos | Por perfil (`profile_id`), tipos `movie`, `series`, `channel` |
| Busca | `search_catalog` unificada (LIKE em channels + movies + series) |
| Player séries | Navega para `/series` com contexto; autoplay próximo episódio no mpv |
| EPG | Stretch — fora desta entrega inicial |

## Fora de escopo (Fase 4+)

- TMDB / enriquecimento externo
- EPG completo / guia de programação
- PiP, Chromecast
- Múltiplos perfis de usuário (UI)

## Arquitetura

```
React UI
  SeriesDetailPage (temporadas, episódios, favorito)
  MovieDetailPage (polish metadados, favorito)
  MyListPage (/list)
  SearchPage (/search)
        │
        ▼ Tauri commands
get_series_detail ──► series_sync (Xtream on-demand)
favorites CRUD
search_catalog (UNION LIKE)
        │
        ▼
SQLite: favorites (+ episodes sync incremental)
```

## Banco de dados (migration 005)

```sql
favorites(
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK(item_type IN ('movie', 'series', 'channel')),
  item_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(profile_id, item_type, item_id)
)
```

Índice: `(profile_id, created_at DESC)`.

## Backend Rust

### Sync on-demand de episódios (`series_sync.rs`)

Quando `get_series_detail` encontra 0 episódios:

1. Carrega série + perfil
2. Se `profile.type != 'xtream'` → retorna vazio (M3U já tem episódios no sync)
3. Usa `series.sort_order` como `series_id` Xtream (convenção Fase 2)
4. `get_series_info` → apaga episódios antigos → insere batch → atualiza metadados da série

### Commands novos

| Command | Descrição |
|---------|-----------|
| `add_favorite` | Adiciona favorito |
| `remove_favorite` | Remove favorito |
| `list_favorites` | Lista enriquecida (nome, poster) |
| `is_favorite` | Boolean |
| `search_catalog` | Busca unificada por perfil |

`get_series_detail` passa a ser `async` e dispara sync on-demand quando necessário.

## Frontend

### Detalhe de série (`/series/:id`)

- Hero: backdrop, título, gêneros, rating
- Pills de temporada (tab ativa)
- Lista de episódios da temporada com botão play
- Play → `/series` + `VodPlayInfo` + fila autoplay
- Estado de loading enquanto sync on-demand

### Detalhe de filme (`/movies/:id`)

- Metadados existentes + toggle favorito

### Minha Lista (`/list`)

- Grid de favoritos (movies, series, channels)
- Clique → detalhe ou play (live → `/live`)

### Busca (`/search?q=`)

- TopBar na Home navega para resultados ao submit
- Agrupamento: Canais | Filmes | Séries

### Player — autoplay

`VodPlayInfo.autoplayQueue`: ao `end-file`/`eof`, reproduz próximo episódio automaticamente.

## Ordem de implementação

1. Docs Fase 3
2. Migration 005 + favorites repo + commands
3. series_sync on-demand + get_series_detail async
4. SeriesDetailPage (tabs + sync + play)
5. Autoplay no usePlayer
6. MyListPage + FavoriteToggle
7. search_catalog + SearchPage + Home search
8. Verificação `cargo test` + `npm run build`

## Critérios de aceite

- [ ] `/series/:id` mostra temporadas e episódios; sync Xtream on-demand se vazio
- [ ] Play episódio abre player em `/series`; autoplay próximo episódio
- [ ] Favoritos persistem por perfil; `/list` exibe grid
- [ ] Heart toggle em detalhes de filme/série
- [ ] Busca unificada retorna channels + movies + series
- [ ] `npm run build` e `cargo test` passam
