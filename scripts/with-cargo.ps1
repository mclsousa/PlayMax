param(
    [Parameter(Position = 0)]
    [string]$Command = "dev",
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Rest
)

$ErrorActionPreference = "Stop"
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) {
    $env:PATH = "$cargoBin;" + $env:PATH
}
$env:CARGO_INCREMENTAL = "1"

$root = Join-Path $PSScriptRoot ".."
Set-Location $root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust/Cargo not found." -ForegroundColor Red
    Write-Host "Install from https://www.rust-lang.org/tools/install" -ForegroundColor Yellow
    exit 1
}

$tauriArgs = @($Command) + $Rest
& npx -- tauri @tauriArgs
