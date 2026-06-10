param(
    [switch]$NoWatch,
    [switch]$SkipPrebuild
)

$ErrorActionPreference = "Stop"
$root = Join-Path $PSScriptRoot ".."
Set-Location $root

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) {
    $env:PATH = "$cargoBin;" + $env:PATH
}
$env:CARGO_INCREMENTAL = "1"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust/Cargo not found." -ForegroundColor Red
    Write-Host "Install from https://www.rust-lang.org/tools/install" -ForegroundColor Yellow
    exit 1
}

& (Join-Path $PSScriptRoot "kill-dev-port.ps1")
& (Join-Path $PSScriptRoot "check-libmpv.ps1")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& (Join-Path $PSScriptRoot "ensure-vite.ps1")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Remove stray agent artifacts that trigger unnecessary Cargo rebuilds.
$examplesDir = Join-Path $root "src-tauri\examples"
$ftsTest = Join-Path $examplesDir "fts_test.rs"
if (Test-Path $ftsTest) {
    Write-Host "Removing stale src-tauri/examples/fts_test.rs..." -ForegroundColor Yellow
    Remove-Item $ftsTest -Force
    if ((Get-ChildItem $examplesDir -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
        Remove-Item $examplesDir -Force -ErrorAction SilentlyContinue
    }
}

if (-not $SkipPrebuild) {
    Write-Host "==> Pre-building Rust (debug)..." -ForegroundColor Cyan
    Write-Host "    First compile can take 2-5 min (linking is slow). Do not Ctrl+C during this step." -ForegroundColor DarkGray
    Push-Location (Join-Path $root "src-tauri")
    cargo build --no-default-features
    $buildExit = $LASTEXITCODE
    Pop-Location
    if ($buildExit -ne 0) { exit $buildExit }
    Write-Host "==> Rust pre-build complete." -ForegroundColor Green
}

$tauriArgs = @("dev")
if ($NoWatch) {
    $tauriArgs += "--no-watch"
    Write-Host "==> --no-watch: Rust file changes require restarting dev." -ForegroundColor Yellow
}

& npx -- tauri @tauriArgs
exit $LASTEXITCODE
