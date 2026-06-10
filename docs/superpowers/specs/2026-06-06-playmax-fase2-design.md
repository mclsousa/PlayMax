# Play Max — Fase 2: Catálogo VOD + Home Real

**Data:** 2026-06-06  
**Status:** Aprovado pelo usuário

## Decisões de escopo

| Decisão | Escolha |
|---------|---------|
| Fontes de lista | M3U + Xtream Codes no onboarding |
| Metadados | Somente da lista (Xtream API / tags M3U) — sem TMDB |
| Classificação M3U | Detecção automática por padrões de `group-title` |
| Escopo da entrega | Catálogo completo + Home com carrosséis reais (recentes, lançamentos) |

## Fora de escopo (Fase 3+)

- TMDB / enriquecimento externo
- Favoritos (`favorites`)
- Histórico / continuar assistindo (`history`)
- Busca unificada FTS5
- EPG / guia de programação
- PiP, múltiplos perfis de usuário

## Arquitetura

```
React UI
  Onboarding (M3U | Xtream)
  Home (hero + carrosséis reais)
  Filmes / Séries / Detalhes
  Player VOD (seek)
        │
        ▼ Tauri commands
SyncService ──► CatalogSource trait
                  ├── M3uSource (classifier live/movie/series)
                  └── XtreamSource (player_api.php)
        │
        ▼
SQLite: profiles, channels, movies, series, episodes
```

## Banco de dados (migration 002)

### `profiles` (alteração)

- `type`: `'m3u' | 'xtream'`
- `username`, `password` (nullable; só Xtream)

### Novas tabelas

```sql
movies(
  id, profile_id, name, poster, backdrop, plot, genres,
  rating, stream_url, category, added_at, sort_order
)

series(
  id, profile_id, name, poster, backdrop, plot, genres,
  rating, category, added_at, sort_order
)

episodes(
  id, series_id, season, episode, title, plot,
  stream_url, duration, added_at
)
```

Índices: `(profile_id, added_at DESC)` em `movies` e `series`.

## Backend Rust

### Xtream client (`parsers/xtream/`)

Endpoints via `player_api.php`:

- `get_live_streams` → `channels` (já coberto indiretamente; Xtream perfis usam isto para ao vivo)
- `get_vod_streams` + `get_vod_categories` → `movies`
- `get_series` + `get_series_categories` → `series`
- `get_series_info` → `episodes` por temporada

Credenciais nunca expostas ao frontend.

### M3U classifier (`parsers/m3u/classifier.rs`)

Classificação por `group-title` (case-insensitive):

| Tipo | Padrões exemplo |
|------|-----------------|
| `movie` | filmes, movies, vod, cinema, 4k movies |
| `series` | séries, series, tv shows, novelas |
| `live` | default (demais grupos) |

**Filmes M3U:** 1 linha EXTINF = 1 filme.

**Séries M3U:** parse título `Nome S01E05` / `Nome 1x05` → agrupa por nome da série; episódios na tabela `episodes`.

### Sync

- `sync_profile` despacha por `profile.type`
- Eventos `sync-progress` com fases distintas
- Batches de 500 registros (padrão Fase 1)
- Ao re-sync: apaga e reinsere conteúdo do perfil (cascade em episodes)

### Commands novos

| Command | Descrição |
|---------|-----------|
| `add_xtream_profile` | Cria perfil Xtream |
| `list_movies` | Paginação + filtro categoria + busca |
| `list_series` | Paginação + filtro categoria + busca |
| `get_movie` | Detalhe de filme |
| `get_series_detail` | Série + episódios por temporada |
| `list_recent` | Últimos N itens (movies + series) |
| `list_releases` | Itens com `added_at` nos últimos 30 dias |

## Frontend

### Onboarding

Toggle **M3U** | **Xtream**:

- M3U: URL ou arquivo (fluxo atual)
- Xtream: URL servidor + usuário + senha

Pós-sync → `/home`.

### Páginas

| Rota | Componente | Comportamento |
|------|------------|---------------|
| `/movies` | `MoviesPage` | Grade virtualizada 2:3, filtro categoria |
| `/series` | `SeriesPage` | Grade posters |
| `/movies/:id` | `MovieDetailPage` | Backdrop, sinopse, Assistir |
| `/series/:id` | `SeriesDetailPage` | Temporadas/episódios (Xtream) ou lista agrupada (M3U) |

### Home

| Seção | Fonte |
|-------|-------|
| Hero | Item mais recente com `backdrop` |
| Adicionados recentemente | `list_recent` |
| Lançamentos | `list_releases` |
| Ao vivo agora | `LiveChannelsRow` (existente) |
| Continuar assistindo | Mock (Fase 3) |

### Player VOD

- Observar `time-pos` e `duration` no mpv
- Barra de seek no overlay
- Tipo de mídia: `live` vs `vod` (seek só em VOD)

## Ordem de implementação

1. Migration + models Rust/TS
2. Xtream client + sync
3. M3U classifier + sync estendido
4. Onboarding dual
5. Commands de listagem + hooks React
6. Páginas Filmes/Séries/Detalhes
7. Home com dados reais
8. Player VOD seek

## Critérios de aceite

- [ ] Onboarding aceita M3U e Xtream
- [ ] Sync Xtream popula movies, series, episodes, channels
- [ ] Sync M3U classifica grupos automaticamente
- [ ] Filmes e Séries exibem grade com posters da lista
- [ ] Detalhe de filme/série reproduz no player
- [ ] Home mostra hero + recentes + lançamentos com dados reais
- [ ] Player VOD permite seek
- [ ] `npm run build` e testes Rust do classifier passam
