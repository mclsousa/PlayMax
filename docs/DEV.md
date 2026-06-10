# Play Max – desenvolvimento local

## Comandos

| Comando | Uso |
|---------|-----|
| `npm run tauri:dev` | Desenvolvimento normal (pre-build + hot reload Rust) |
| `npm run tauri:dev:fast` | Pre-build + Tauri sem watcher Rust (`--no-watch`) – só frontend muda |
| `npm run app:local` | Frontend buildado + app **sem Vite** (usa `dist/`, evita erro localhost) |
| `npm run setup:lib:manual` | Baixa `libmpv-wrapper.dll` e `libmpv-2.dll` (obrigatório uma vez) |

## Tempos esperados

| Situação | Tempo |
|----------|-------|
| **Primeira compilação** (projeto novo ou `cargo clean`) | ~2–5 min |
| **Reinício com cache** (`npm run tauri:dev`) | ~10–30 s |
| **Só mudanças no frontend** (`tauri:dev:fast` ou app já aberto) | instantâneo (HMR Vite) |

A etapa mais lenta é o **link** do Rust (ex.: `446/448` no Cargo). O script `tauri:dev` faz um `cargo build` **antes** de abrir o watcher, para você ver o progresso completo sem surpresas.

## Regras importantes

1. **Não pressione Ctrl+C** enquanto o Cargo estiver compilando ou linkando — isso deixa binários pela metade e o próximo start pode falhar ou demorar mais.
2. **Não edite arquivos `.rs` em paralelo** enquanto a primeira compilação roda (ex.: agentes de IA salvando vários arquivos de uma vez disparam rebuilds em loop).
3. **`npm run dev` sozinho** abre só o Vite no browser — reprodução mpv **não funciona** fora do Tauri.
4. **Não abra `play-max.exe` direto** — no modo dev a UI vem de `localhost:1420`. Se aparecer "localhost se recusou a conectar", use `npm run tauri:dev` ou `npm run app:local`.
5. Se faltar DLL mpv, o dev para com erro claro; rode `npm run setup:lib:manual`.

## Warm start (opcional)

Se quiser aquecer o cache Rust sem abrir o app:

```powershell
cd src-tauri
$env:CARGO_INCREMENTAL = "1"
cargo build --no-default-features
```

Depois `npm run tauri:dev` reutiliza o artefato e sobe mais rápido.

## Porta 1420 ocupada

`tauri:dev` encerra processos na porta 1420, depois `scripts/ensure-vite.ps1` sobe o Vite (se necessário) e espera `http://localhost:1420`. O `beforeDevCommand` do Tauri chama o mesmo script com `-WaitOnly` (só aguarda a URL). Se o app não subir, verifique se outro terminal ainda está com `tauri:dev` aberto.
