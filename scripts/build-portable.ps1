$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

npm run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

powershell -ExecutionPolicy Bypass -File scripts/with-cargo.ps1 build --no-bundle
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$releaseDir = Join-Path $root "release"
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

$exeCandidates = @(
  (Join-Path $root "src-tauri\target\release\play-max.exe"),
  (Join-Path $env:LOCALAPPDATA "Temp\cursor-sandbox-cache\*\cargo-target\release\play-max.exe")
)

$sourceExe = $null
foreach ($candidate in $exeCandidates) {
  $resolved = Get-Item -Path $candidate -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($resolved) {
    $sourceExe = $resolved.FullName
    break
  }
}

if (-not $sourceExe) {
  Write-Error "play-max.exe não encontrado após o build."
}

$destExe = Join-Path $releaseDir "play-max.exe"
Copy-Item $sourceExe $destExe -Force
Write-Host "Executável portátil gerado em: $destExe"
