# Play Max — setup de licenciamento
# Execute na raiz do projeto: .\scripts\setup-licensing.ps1

$ErrorActionPreference = "Stop"

Write-Host "=== Play Max: Setup de licenciamento ===" -ForegroundColor Cyan

$root = Split-Path -Parent $PSScriptRoot
$portalDir = Join-Path $root "license-portal"

Write-Host "`n[1/4] Build do portal de licenças..." -ForegroundColor Yellow
Push-Location $portalDir
npm install
npm run build
Pop-Location

Write-Host "`n[2/4] Verificando Vercel CLI..." -ForegroundColor Yellow
$vercelUser = vercel whoami 2>$null
if ($LASTEXITCODE -ne 0) {
  Write-Host "Vercel não logado. Rode: vercel login" -ForegroundColor Red
} else {
  Write-Host "Vercel logado como: $vercelUser" -ForegroundColor Green
}

Write-Host "`n[3/4] Configurar .env do app desktop" -ForegroundColor Yellow
$appEnv = Join-Path $root ".env"
$portalUrl = Read-Host "URL do portal Vercel (ex: https://playmax-lic.vercel.app)"

@"
VITE_LICENSE_API_URL=$portalUrl
"@ | Set-Content -Path $appEnv -Encoding UTF8
Write-Host "Criado $appEnv" -ForegroundColor Green

Write-Host "`n[4/4] Build do instalador Play Max..." -ForegroundColor Yellow
Push-Location $root
npm run build
npm run tauri:build
Pop-Location

Write-Host "`n=== Concluído ===" -ForegroundColor Cyan
Write-Host "Instalador em: src-tauri\target\release\bundle\"
Write-Host "`nAinda falta (só você pode fazer):" -ForegroundColor Yellow
Write-Host "  1. Supabase: criar projeto + rodar license-portal\supabase\migrations\001_licenses.sql"
Write-Host "  2. Vercel: cd license-portal && vercel --prod (com env vars do .env.example)"
Write-Host "  3. Stripe: webhook apontando para /api/webhooks/stripe"
Write-Host "  4. Code signing do .exe (opcional mas recomendado)"
