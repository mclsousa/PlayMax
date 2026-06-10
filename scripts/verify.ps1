# Play Max verification script (Phase 1)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

Write-Host "==> Frontend build" -ForegroundColor Cyan
npm run build

Write-Host "==> Generate 10k fixture" -ForegroundColor Cyan
node scripts/generate-fixture.mjs 10000

Write-Host "==> Check mpv DLLs" -ForegroundColor Cyan
& (Join-Path $PSScriptRoot "check-libmpv.ps1")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "  OK libmpv-wrapper.dll, libmpv-2.dll" -ForegroundColor Green

$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "==> Rust tests (M3U parser)" -ForegroundColor Cyan
    Push-Location src-tauri
    cargo test m3u -- --nocapture
    Pop-Location

    Write-Host "==> Tauri release build" -ForegroundColor Cyan
    npm run tauri build
} else {
    Write-Host "Rust not installed. Skipped cargo test and tauri build." -ForegroundColor Yellow
    Write-Host "Install from https://www.rust-lang.org/tools/install" -ForegroundColor Yellow
}

Write-Host "Verification complete." -ForegroundColor Green
