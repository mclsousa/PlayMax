# Play Max — Licenciamento (Supabase + Vercel + Stripe)

Painel SaaS para gerar chaves `PLAY-XXXX-XXXX-XXXX`, cobrar via Stripe (cartão/Pix) e validar o app desktop.

## 1. Supabase

1. Crie um projeto em [supabase.com](https://supabase.com)
2. SQL Editor → execute `supabase/migrations/001_licenses.sql`
3. Copie **Project URL** e **service_role key** (Settings → API)

## 2. Stripe

1. Crie produto + preço recorrente (mensal) no Dashboard
2. Ative **Pix** em Settings → Payment methods (Brasil)
3. Copie `STRIPE_SECRET_KEY` e crie um **Price ID** (`price_...`)
4. Webhook apontando para `https://SEU-DOMINIO/api/webhooks/stripe`:
   - `checkout.session.completed`
   - `invoice.paid`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`

## 3. Deploy Vercel

```bash
cd license-portal
cp .env.example .env.local
# preencha as variáveis
npm install
npm run build
```

No Vercel: importe a pasta `license-portal`, configure as env vars do `.env.example`.

## 4. App desktop

```bash
cp .env.example .env
# VITE_LICENSE_API_URL=https://seu-portal.vercel.app
npm run tauri build
```

Distribua o `.exe` da pasta `src-tauri/target/release/bundle/`.

## Fluxo operacional (100 clientes)

1. Cliente paga no link do portal **ou** você gera chave manual em `/admin`
2. Você envia por WhatsApp: **instalador + chave PLAY-XXXX + credenciais M3U/Xtream**
3. Cliente instala → ativa licença → cola lista → sincroniza

## Admin

- URL: `https://seu-portal.vercel.app/admin`
- Header: `x-admin-key: ADMIN_API_KEY`
- **Nova licença** — gera chave manual (trial, cortesia, etc.)
- **Bloquear** — revoga acesso imediato na próxima validação

## API (app desktop)

| Endpoint | Uso |
|----------|-----|
| `POST /api/license/activate` | Primeira ativação (1 PC) |
| `POST /api/license/validate` | Revalidação periódica |

Body: `{ licenseKey, deviceFingerprint, deviceName? }`

## Segurança

- Nunca commite `SUPABASE_SERVICE_ROLE_KEY` ou `STRIPE_SECRET_KEY`
- Use `ADMIN_API_KEY` longo e aleatório
- Assine o `.exe` com certificado Windows antes de distribuir em massa

## Próximos passos sugeridos

- E-mail automático com chave após pagamento Stripe
- Tauri Updater para correções sem reenviar `.exe`
- Code signing (SmartScreen)
