# Play Max Fase 4 — Player Avançado + Polish

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Controles avançados do player (legendas, áudio, skip), favorito em canais ao vivo, polish de detalhe de filme.

**Architecture:** `useMpvTracks` + `TrackPopover` no overlay; navegação via `VodPlayInfo.navItems` e `buildChannelNavigation`; sem novos commands Rust.

**Tech Stack:** Tauri 2, React 19, TypeScript, Tailwind 4, `tauri-plugin-libmpv-api`

**Spec:** `docs/superpowers/specs/2026-06-06-playmax-fase4-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `src/lib/mpvTracks.ts` | Parse `track-list` node |
| `src/hooks/useMpvTracks.ts` | Estado faixas + select sid/aid |
| `src/components/player/TrackPopover.tsx` | Menu popover pt-BR |
| `src/components/player/PlayerIcons.tsx` | Ícones legendas/áudio/skip |
| `src/components/player/PlayerOverlay.tsx` | Wire tracks + navegação |
| `src/hooks/usePlayer.ts` | `vodInfo`, `playNavNext/Prev` |
| `src/lib/playerNavigation.ts` | Helpers live/VOD nav |
| `src/lib/titleMeta.ts` | Ano + rating format |
| `src/lib/types.ts` | `VodNavigationItem`, nav fields |
| `src/components/live/ChannelCard.tsx` | FavoriteToggle sm |
| `src/pages/LivePage.tsx` | Channel prev/next |
| `src/pages/MoviesPage.tsx` | Movie nav + overlay props |
| `src/pages/SeriesPage.tsx` | Episode nav |
| `src/pages/SeriesDetailPage.tsx` | navItems no play |
| `src/pages/MovieDetailPage.tsx` | Meta polish |

---

### Task 1: mpv tracks hook + popover

**Files:**
- Create: `src/lib/mpvTracks.ts`, `src/hooks/useMpvTracks.ts`, `src/components/player/TrackPopover.tsx`
- Modify: `src/components/player/PlayerIcons.tsx`, `src/components/player/PlayerOverlay.tsx`

- [ ] **Step 1:** `parseMpvTrackList` filtra `audio` e `sub`
- [ ] **Step 2:** `useMpvTracks` refresh on `mediaKey` + delay 2s
- [ ] **Step 3:** `TrackPopover` com opção "Desativadas" para legendas
- [ ] **Step 4:** Ícones + tooltips pt-BR no overlay

---

### Task 2: Navegação prev/next

**Files:**
- Create: `src/lib/playerNavigation.ts`
- Modify: `src/lib/types.ts`, `src/hooks/usePlayer.ts`
- Modify: `src/pages/LivePage.tsx`, `MoviesPage.tsx`, `SeriesPage.tsx`, `SeriesDetailPage.tsx`

- [ ] **Step 1:** Estender `VodPlayInfo` com `navItems` / `navIndex`
- [ ] **Step 2:** `playNavNext/Prev` + `canNavNext/Prev` em `usePlayer`
- [ ] **Step 3:** Live: `buildChannelNavigation`
- [ ] **Step 4:** Movies/Series: popular `navItems` ao dar play
- [ ] **Step 5:** Skip buttons no `PlayerOverlay`

---

### Task 3: Favorito ao vivo + movie polish

**Files:**
- Modify: `src/components/favorites/FavoriteToggle.tsx`, `ChannelCard.tsx`
- Modify: `src/pages/MovieDetailPage.tsx`
- Create: `src/lib/titleMeta.ts`

- [ ] **Step 1:** `FavoriteToggle` size sm em `ChannelCard`
- [ ] **Step 2:** `extractYearFromTitle` + `formatRating`
- [ ] **Step 3:** Meta line: categoria • ano • gêneros • rating

---

### Task 4: Documentação deferidos

- [ ] **PiP:** Documentado como inviável com mpv HWND — Fase 5
- [ ] **EPG:** Sem endpoint no codebase — Fase 5 (`get_short_epg`)

---

### Task 5: Verificação

```bash
npm run build
cd src-tauri && cargo test
```

Expected: PASS

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Legendas UI | Task 1 |
| Áudio UI | Task 1 |
| Prev/next live | Task 2 |
| Prev/next VOD | Task 2 |
| Favorito ao vivo | Task 3 |
| Movie detail polish | Task 3 |
| PiP deferred | Task 4 |
| EPG deferred | Task 4 |

## Execution handoff

Plan saved to `docs/superpowers/plans/2026-06-06-playmax-fase4.md`.
