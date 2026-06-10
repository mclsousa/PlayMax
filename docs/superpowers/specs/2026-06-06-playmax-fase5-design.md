# Play Max — Fase 5: EPG + Polish

**Data:** 2026-06-06  
**Status:** Aprovado

## Decisões de escopo

| Decisão | Escolha |
|---------|---------|
| EPG fonte | Xtream `get_short_epg` via `stream_id` (extraído da URL ao vivo) |
| Cache | SQLite `epg_programs`, TTL 2h, fetch on-demand ao reproduzir canal |
| M3U | Mensagem graciosa **"EPG indisponível"** (sem `stream_id` Xtream) |
| UI EPG | Player overlay + linha compacta no `ChannelCard` ativo |
| Sync background | **Documentado / deferido** — intervalo 30min + toggle settings |
| Busca FTS5 | **Deferido Fase 6** — LIKE unificado permanece |
| PiP | **Fase 6** — mpv embutido ainda inviável |
| TMDB | **Fase 6** |

## Fora de escopo (Fase 6+)

- PiP nativo Windows com mpv embutido (janela always-on-top separada)
- TMDB / enriquecimento externo
- EPG XMLTV completo (`xmltv.php`, `get_simple_data_table` bulk)
- Busca FTS5
- Sync periódico automático em background (spec abaixo, implementação futura)

## Arquitetura

```
React UI
  LivePage + ChannelCard + PlayerOverlay
  useChannelEpg(profileId, channelId)
        │
        ▼ Tauri command
get_channel_epg(profile_id, channel_id, limit?)
        │
        ▼
epg_service.rs
  ├─ cache hit? → epg_programs (SQLite, TTL 2h)
  └─ miss → XtreamClient.get_short_epg(stream_id)
            → decode base64 title/description
            → replace_channel_programs
        │
        ▼
Xtream API: player_api.php?action=get_short_epg&stream_id=X&limit=N
```

## Banco de dados (migration 006)

```sql
epg_programs(
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  channel_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  start_ts INTEGER NOT NULL,
  end_ts INTEGER NOT NULL,
  epg_id TEXT,
  fetched_at INTEGER NOT NULL
)
```

Índices: `(profile_id, channel_id, start_ts)`, `(profile_id, channel_id, fetched_at DESC)`.

## Backend Rust

### Xtream client

- `get_short_epg(stream_id, limit)` → `XtreamShortEpgResponse { epg_listings }`
- Títulos/descrições decodificados de Base64 (`decode_xtream_epg_text`)

### `epg_service.rs`

1. Carrega canal + perfil
2. Se `profile.type != 'xtream'` → `{ available: false, reason: "EPG indisponível" }`
3. Extrai `stream_id` da URL (`/live/.../ID.ts`)
4. Cache fresh (< 2h)? → retorna SQLite
5. Senão → API Xtream → persiste cache → retorna programas

### Command

| Command | Descrição |
|---------|-----------|
| `get_channel_epg` | EPG on-demand com cache; `limit` default 4, max 20 |

## Frontend

### Hook `useChannelEpg`

Dispara fetch quando `channelId` muda (canal em reprodução).

### UI

| Local | Conteúdo |
|-------|----------|
| `PlayerOverlay` | **Agora:** título + horário; **Depois:** próximo programa |
| `ChannelCard` (ativo) | Linha compacta "Agora / Depois" |
| M3U | "EPG indisponível" |

### Polish — favorito em grid VOD

- `PosterCard` aceita `favoriteItemType` + `favoriteItemId`
- `VodGrid` repassa tipo (`movie` | `series`)

## Sync em background (deferido)

**Design futuro (não implementado nesta fase):**

- Intervalo: 30 minutos enquanto app aberto
- Toggle em Settings: "Atualizar catálogo automaticamente"
- Reutiliza `sync_profile` existente; emite evento `sync-progress`
- Pausar quando player ativo ou sync manual em andamento

## PiP — nota técnica (Fase 6)

Mesma conclusão da Fase 4: mpv via HWND não expõe PiP. Alternativa viável seria janela Tauri secundária always-on-top com segundo viewport mpv — estimativa > 2h, fora desta entrega.

## Busca FTS5 — nota (Fase 6)

Migration futura com tabela FTS5 espelhando movies/series/channels; `search_catalog` passaria a usar `MATCH`. Mantido LIKE na Fase 5 por escopo.

## Critérios de aceite

- [ ] Xtream: EPG exibe programa atual + próximo no player ao vivo
- [ ] Cache SQLite evita refetch dentro de 2h
- [ ] M3U: "EPG indisponível" sem erro
- [ ] `get_channel_epg` registrado no ACL
- [ ] Testes Rust com fixture `short_epg.json`
- [ ] Heart opcional em `PosterCard` / `VodGrid`
- [ ] `npm run build` e `cargo test` passam
