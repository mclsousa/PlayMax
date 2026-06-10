# Play Max

Aplicativo IPTV desktop para Windows — Tauri 2 + React + Rust + libmpv. Fase 1 (Ao Vivo) + Fase 2 (catálogo VOD).

## Pré-requisitos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri prerequisites (Windows)](https://tauri.app/start/prerequisites/)
- WebView2 (já incluso no Windows 10/11)

## Setup

```bash
npm install
npm run setup:lib:manual   # baixa libmpv-wrapper.dll e libmpv-2.dll
```

Se `setup:lib` automático falhar, use o script manual acima (requer 7zr na primeira execução).

## Desenvolvimento

```bash
npm run tauri:dev
```

Detalhes de tempos de startup, warm start e troubleshooting: **[docs/DEV.md](docs/DEV.md)**.

| Comando | Descrição |
|---------|-----------|
| `npm run tauri:dev` | Pre-build Rust + dev com hot reload |
| `npm run tauri:dev:fast` | Pre-build + dev sem watcher Rust (só UI) |

> **Primeira compilação:** ~2–5 min (link lento). **Reinícios:** ~10–30 s. Não use Ctrl+C durante o compile.

> Se `cargo not found` aparecer, o Rust está instalado mas fora do PATH deste terminal. Use `npm run tauri:dev` (já corrige o PATH) ou reinicie o terminal após instalar o Rust.

> O preview `npm run dev` abre só o frontend no browser — reprodução mpv **não** funciona fora do Tauri.

## Build release

```bash
npm run tauri build
```

## Fixture de performance (10k canais)

```bash
node scripts/generate-fixture.mjs 10000
```

Importe `tests/fixtures/channels-10000.m3u` via onboarding para validar scroll virtualizado.

## Funcionalidades (Fase 1)

- Onboarding com lista M3U (URL ou arquivo)
- Cache SQLite de canais
- Tela Ao Vivo com lista virtualizada, filtro por grupo e busca
- Player nativo libmpv (play/pause, stop, volume, mute, fullscreen, retry)
- Configurações: atualizar/remover listas

## Funcionalidades (Fase 2)

- **Onboarding Xtream Codes** — URL do servidor, usuário e senha; sincronização via `player_api.php`
- **Classificação unificada de conteúdo** — módulo `content_kind` separa canais, filmes e séries (ver abaixo)
- **Filmes e Séries** — grades com posters, páginas de detalhe (sinopse, elenco, episódios)
- **Home com carrosséis** — “Adicionados recentemente” e “Lançamentos” alimentados pelo catálogo SQLite
- **Player VOD** — barra de seek, posição/duração e controles de reprodução para filmes e episódios

## Classificação de conteúdo

O backend usa `src-tauri/src/services/content_kind.rs` como fonte única de regras:

| Tipo | Regras principais |
|------|-------------------|
| **Canal** | Marcas IPTV (MEGAPIX, Universal TV, Premiere, Telecine, Sportv…), sufixos FHD/HD/4K sem ano, grupos Canais/Live/24h |
| **Filme** | Grupos Filmes/VOD/Lançamentos, ou grupo de gênero (Comédia, Drama…) + título com `(AAAA)` ou duração |
| **Série** | Grupo Séries, ou título com `S01E05` / `1x05`; episódios agregados por nome da série |

**Xtream:** `get_live_streams` → canais; `get_vod_streams` → filmes; `get_series` → séries. Categorias da API (`get_*_categories`) viram nomes de grupo. Streams ao vivo nunca entram na tabela de filmes.

**M3U:** cada entrada é classificada antes do insert; sync apaga e reconstrói o catálogo do perfil.

**Home:** carrosséis usam `list_recent_movies`, `list_recent_series` e `list_releases` (últimos 30 dias pelo `added_at` do servidor). Canais e duplicatas são filtrados na query.

> **Após atualizar:** abra **Configurações** e **sincronize novamente** cada perfil para reclassificar o catálogo com as regras novas.

## Estrutura

- `src/` — UI React (páginas Home, Filmes, Séries, detalhes, player)
- `src-tauri/src/` — backend Rust (parsers M3U/Xtream, SQLite, sync, commands)
- `src-tauri/lib/` — DLLs mpv (não versionadas; geradas pelo setup)
- `tests/fixtures/` — M3U de teste (canais, catálogo VOD)
