# Fail fast when libmpv DLLs are missing (required for Tauri dev/build).
$ErrorActionPreference = "Stop"
$root = Join-Path $PSScriptRoot ".."
$libDir = Join-Path $root "src-tauri\lib"
$dlls = @("libmpv-wrapper.dll", "libmpv-2.dll")
$missing = @()

foreach ($dll in $dlls) {
    if (-not (Test-Path (Join-Path $libDir $dll))) {
        $missing += $dll
    }
}

if ($missing.Count -gt 0) {
    Write-Host "Missing libmpv DLL(s): $($missing -join ', ')" -ForegroundColor Red
    Write-Host "Expected in: $libDir" -ForegroundColor Yellow
    Write-Host "Run: npm run setup:lib:manual" -ForegroundColor Yellow
    exit 1
}
