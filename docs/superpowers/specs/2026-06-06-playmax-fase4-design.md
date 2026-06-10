# Play Max — Fase 4: Player Avançado + Polish

**Data:** 2026-06-06  
**Status:** Aprovado

## Decisões de escopo

| Decisão | Escolha |
|---------|---------|
| Legendas / áudio | mpv `track-list` via `tauri-plugin-libmpv-api`; UI em popover no `PlayerOverlay` |
| Próximo / Anterior | Ao vivo: lista da categoria atual; VOD: `navItems` + `navIndex` em `VodPlayInfo` |
| Favorito ao vivo | `FavoriteToggle` em `ChannelCard` (heart compacto) |
| Detalhe de filme | Categoria, ano extraído do título, rating formatado |
| PiP | **Fora de escopo** — ver nota técnica abaixo |
| EPG | **Fase 5** — endpoint Xtream não implementado no backend |

## Fora de escopo (Fase 5+)

- EPG / guia de programação (`get_short_epg`, `get_simple_data_table`)
- PiP nativo no Windows com mpv embutido
- Chromecast / AirPlay
- TMDB

## Arquitetura

```
React UI
  PlayerOverlay (+ TrackPopover, skip prev/next)
  ChannelCard (+ FavoriteToggle)
  MovieDetailPage (meta polish)
        │
        ▼ tauri-plugin-libmpv-api (sem novos commands Rust)
getProperty('track-list', 'node')
setProperty('sid' | 'aid', id)
command('loadfile', …)  — já existente
        │
        ▼
usePlayer (+ vodInfo, playNavNext/Prev)
useMpvTracks
```

## Player — faixas de mídia

### Leitura de faixas

Após `file-loaded` (ou delay curto pós-`loadfile`):

1. `getProperty('track-list', 'node')` → parse por `type: 'audio' | 'sub'`
2. `getProperty('aid', 'int64')` / `getProperty('sid', …)` para faixa ativa

### Seleção

| Ação | mpv |
|------|-----|
| Trocar legenda | `setProperty('sid', trackId)` |
| Desativar legenda | `setProperty('sid', 'no')` |
| Trocar áudio | `setProperty('aid', trackId)` |

UI: botões ícone-only com tooltip pt-BR; menu popover lista faixas.

## Player — navegação

### Ao vivo (`LivePage`)

- Índice do canal na lista virtualizada da categoria/busca atual
- `buildChannelNavigation(channels, playingId)` → `goPrev` / `goNext` chama `loadStream`

### VOD (`MoviesPage`, `SeriesPage`)

`VodPlayInfo` estendido:

```typescript
navItems?: VodNavigationItem[];
navIndex?: number;
```

- Filmes: `navItems` = filmes visíveis na lista da categoria
- Séries: `navItems` = episódios ordenados (T/E); `autoplayQueue` = slice após índice atual
- `usePlayer.playNavNext/Prev` atualiza `navIndex` e reconstrói `autoplayQueue`

## Polish — favoritos ao vivo

- `FavoriteToggle` com `size="sm"` ao lado de cada `ChannelCard`
- Tipo `channel`; mesma API Fase 3 (`add_favorite` / `remove_favorite`)
- Sem alteração de ACL ou Rust

## Polish — detalhe de filme

- Categoria: `location.state.category` ou `movie.category`
- Ano: regex em `movie.name` — `(YYYY)` ou token `19xx|20xx`
- Rating: `formatRating()` — prefixo ★ e uma casa decimal se numérico

## PiP — nota técnica (não viável nesta fase)

O player usa mpv embutido via HWND (`tauri-plugin-libmpv-api` + `data-mpv-viewport`). Picture-in-Picture no Windows exigiria:

- Janela flutuante separada do Tauri com segundo contexto de renderização, ou
- API de PiP do sistema operacional (não exposta pelo plugin atual)

**Conclusão:** documentar como Fase 5; usuário usa tela cheia (F) como alternativa.

## EPG — nota técnica (Fase 5)

O sync Xtream persiste `epg_channel_id` (`tvg_id`) em canais, mas **não há** client para:

- `get_short_epg`
- `get_simple_data_table`
- `xmltv.php`

Implementar EPG básico requer novo serviço Rust + cache SQLite + UI no painel ao vivo.

## Critérios de aceite

- [ ] Popover de legendas lista faixas `sub` e permite desativar
- [ ] Popover de áudio lista faixas quando há mais de uma
- [ ] Botões Anterior/Próximo funcionam em `/live`, `/movies`, `/series`
- [ ] Heart em itens da `ChannelList` persiste favorito por perfil
- [ ] Detalhe de filme exibe categoria, ano e rating
- [ ] `npm run build` e `cargo test` passam
- [ ] Sem novos commands Tauri (mpv via plugin existente)
