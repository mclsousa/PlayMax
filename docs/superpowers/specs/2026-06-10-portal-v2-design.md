# Design: Portal de licenças v2 — entrega automática, admin de suporte, landing essencial

**Data:** 2026-06-10
**Status:** Implementado em 2026-06-10. E2E produção 16/17 PASS — o item restante (by-session caminho de erro 404) depende de configurar as env vars do Stripe no Vercel (STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, NEXT_PUBLIC_STRIPE_PRICE_ID), sem as quais o fluxo de venda automática não opera.

## Contexto

O portal (license-portal, Next.js 15 + Supabase + Stripe) já tem: webhook Stripe
completo (compra cria licença, renovação estende, cancelamento revoga/expira),
checkout com `success_url=/success?session_id={CHECKOUT_SESSION_ID}`, webhook
gravando `stripe_checkout_session_id` na licença, e tokens de licença assinados
(spec 2026-06-09). Faltam três coisas, decididas com o usuário:

1. **Entrega automática da chave** após pagamento Stripe (hoje a página de
   sucesso manda esperar o WhatsApp — entrega manual).
2. **Admin com kit de suporte**: desbloquear, resetar ativação de PC, editar
   vencimento/nº de dispositivos, notas. (Busca/filtros, opções na criação e
   redesign visual ficaram FORA do escopo, por decisão do usuário.)
3. **Landing essencial**: preço, o que está incluso, botão assinar — sem
   redesign elaborado.

O usuário vende pelos dois canais: checkout Stripe automático e manual via
WhatsApp/Pix (cria chave no painel).

## 1. Entrega automática da chave

### Novo endpoint: `GET /api/license/by-session?session_id=cs_...`

Arquivo: `src/app/api/license/by-session/route.ts`, lógica em
`src/lib/license.ts` (`getLicenseBySession`).

Fluxo do handler:
1. `session_id` ausente ou sem prefixo `cs_` → 400 `{ok:false, code:"INVALID_REQUEST"}`.
2. `stripe.checkout.sessions.retrieve(session_id)` — sessão inexistente →
   404 `{ok:false, code:"SESSION_NOT_FOUND"}`.
3. `session.payment_status !== "paid"` → 402 `{ok:false, code:"NOT_PAID"}`.
4. Busca licença com `stripe_checkout_session_id = session_id`:
   - encontrada → 200 `{ok:true, licenseKey, status, expiresAt}`.
   - não encontrada (webhook ainda processando) → 200 `{ok:true, pending:true}`.

Segurança: o `session_id` do Stripe é um token de alta entropia que só o
comprador recebe; a verificação `paid` no Stripe impede enumeração útil. O
endpoint nunca lista nem busca licenças por outro critério.

### Página `/success` refeita (client component)

- Lê `session_id` da URL. Sem `session_id` → mensagem genérica de pagamento
  recebido (comportamento atual).
- Polling no endpoint a cada 2s, máximo 30 tentativas (~60s):
  - chave chegou → exibe `PLAY-XXXX-XXXX-XXXX` em destaque, botão **Copiar**
    (navigator.clipboard + feedback "copiado"), e os passos: 1) baixe e
    instale o Play Max, 2) abra e cole a chave, 3) adicione sua lista M3U/Xtream.
  - timeout → "Pagamento confirmado! Sua chave está sendo gerada — atualize a
    página em instantes ou fale com o suporte." (mantém o link/menção WhatsApp).
  - erro 4xx → mensagem de sessão inválida com contato de suporte.

## 2. Admin — kit de suporte

### API (`src/app/api/admin/licenses/route.ts` + `src/lib/license.ts`)

PATCH ganha novas actions (mantendo `revoke`):

| action | Efeito |
|---|---|
| `revoke` | (existente) status → `revoked` |
| `unrevoke` | status → `active`. O vencimento continua mandando: se `expires_at` já passou, a licença segue rejeitada por `LICENSE_EXPIRED` na validação. |
| `reset_devices` | apaga TODAS as linhas de `license_activations` da licença. Cliente ativa no PC novo; o PC antigo recebe `DEVICE_NOT_ACTIVATED` na próxima validação (rejeição autoritativa — o app já apaga o token local, nenhuma mudança no app é necessária). |
| `update` | atualiza campos opcionais do body: `expiresAt` (ISO string ou `null` para "sem vencimento"), `maxDevices` (inteiro ≥ 1), `notes` (string ou `null`). Só altera os campos presentes no body. |

Novas funções em `src/lib/license.ts`: `unrevokeLicense(licenseId)`,
`resetActivations(licenseId)`, `updateLicense(licenseId, {expiresAt?, maxDevices?, notes?})`.
Validações: `maxDevices` inteiro 1–10; `expiresAt` data ISO válida ou null;
action desconhecida → 400.

### UI (`src/app/admin/page.tsx`)

Mantém o estilo atual (inline styles, sem redesign). Mudanças:

- Nova coluna **Nota** na tabela (texto truncado).
- Coluna Ações por linha:
  - `Bloquear` (status ≠ revoked) / `Desbloquear` (status = revoked);
  - `Resetar PC` (habilitado quando ativações > 0), com `confirm()` nativo;
  - `Editar` — alterna uma linha expandida abaixo com 3 campos:
    vencimento (`<input type="date">` + checkbox "sem vencimento"),
    nº de PCs (`<input type="number" min=1 max=10>`),
    nota (`<input type="text">`), botões Salvar/Cancelar.
- O GET já retorna `notes`, `max_devices` e `license_activations(count)`; a
  contagem de PCs passa a exibir `count / max_devices` (hoje é fixo `/ 1`).

## 3. Landing essencial (`src/app/page.tsx`)

Mantém a estrutura de página única, sem framework de UI:

- Título "Play Max" + tagline curta.
- **Preço** vindo de `NEXT_PUBLIC_PRICE_DISPLAY` (ex.: `R$ 25/mês`); se a env
  não existir, omite o preço (não quebra). Documentar no `.env.example`.
- 3–4 bullets do que está incluso (1 PC por licença, filmes/séries/TV ao vivo,
  EPG, suporte via WhatsApp).
- Botão "Assinar Play Max" (fluxo de checkout existente, sem mudanças).
- Linha de pós-compra: "Após o pagamento, sua chave aparece na tela na hora."
- Link "Painel admin" discreto no rodapé (como hoje).

## Fora de escopo (decidido)

- Busca/filtros/ordenacão e botão copiar no admin; opções na criação manual
  (vencimento/PCs/nota no momento de criar — dá para editar logo após criar).
- Redesign visual do admin e da landing (dark theme, Lucide etc.).
- Envio de chave por e-mail (Resend) — candidato a etapa futura.
- Login admin diferente do campo `x-admin-key` atual.
- `timingSafeEqual`/rate-limiting (item separado do relatório de segurança).

## Tratamento de erros

- `by-session`: respostas tipadas por código (tabela acima); erros do Stripe
  logados no servidor, resposta genérica 500.
- Admin PATCH: action desconhecida ou validação falha → 400 com mensagem;
  licença inexistente → 404.
- Success page: qualquer falha de rede durante o polling conta como tentativa;
  timeout mostra o caminho de suporte (nunca tela branca).

## Testes

- Portal: `npx tsc --noEmit` + `npm test` (suíte existente) verdes;
  `npm run build` limpo.
- Teste de unidade para `getLicenseBySession` não é viável sem mockar Stripe —
  cobertura via E2E.
- E2E autorizado contra produção (mesmo padrão do teste do token):
  criar licença de teste → `update` (vencimento/maxDevices/nota) → verificar via
  GET → `reset_devices` → ativar de novo → `revoke` → `unrevoke` → revogar ao
  final e conferir cada resposta.
- Fluxo Stripe completo: checklist manual com cartão de teste do Stripe
  (modo test) OU validação em produção na primeira venda real, a critério do
  usuário — o endpoint `by-session` é testável com uma sessão paga real.
