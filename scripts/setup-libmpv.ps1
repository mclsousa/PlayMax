# Manual libmpv setup for Play Max (Windows x86_64)
$ErrorActionPreference = "Stop"
$libDir = Join-Path $PSScriptRoot "..\src-tauri\lib"
$tempDir = Join-Path $libDir "temp"
New-Item -ItemType Directory -Force -Path $libDir, $tempDir | Out-Null

function Copy-FirstMatch($root, $pattern, $dest) {
    $match = Get-ChildItem -Path $root -Recurse -Filter $pattern -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $match) { throw "$pattern not found under $root" }
    Copy-Item $match.FullName $dest -Force
    return $match.FullName
}

if (-not (Test-Path (Join-Path $libDir "libmpv-wrapper.dll"))) {
    Write-Host "Downloading libmpv-wrapper..." -ForegroundColor Cyan
    $wrapperUrl = "https://github.com/nini22P/libmpv-wrapper/releases/download/v0.1.0/libmpv-wrapper-windows-x86_64.zip"
    $wrapperZip = Join-Path $tempDir "wrapper.zip"
    Invoke-WebRequest -Uri $wrapperUrl -OutFile $wrapperZip -UseBasicParsing
    $wrapperExtract = Join-Path $tempDir "wrapper_extract"
    if (Test-Path $wrapperExtract) { Remove-Item $wrapperExtract -Recurse -Force }
    Expand-Archive -Path $wrapperZip -DestinationPath $wrapperExtract -Force
    $copied = Copy-FirstMatch $wrapperExtract "libmpv-wrapper.dll" (Join-Path $libDir "libmpv-wrapper.dll")
    Write-Host "  -> $copied" -ForegroundColor Green
} else {
    Write-Host "libmpv-wrapper.dll already present." -ForegroundColor Green
}

if (-not (Test-Path (Join-Path $libDir "libmpv-2.dll"))) {
    Write-Host "Downloading mpv-dev..." -ForegroundColor Cyan
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/zhongfly/mpv-winbuild/releases/latest"
    # Match official `npx tauri-plugin-libmpv-api setup-lib`: LGPL build, non-v3 (no AVX2 requirement).
    $asset = $release.assets | Where-Object {
        $_.name -match "^mpv-dev-lgpl-x86_64-" -and $_.name -notmatch "v3"
    } | Select-Object -First 1
    if (-not $asset) { throw "Could not find mpv-dev-lgpl-x86_64 (non-v3) asset in latest release." }

    $mpvZip = Join-Path $tempDir "mpv-dev.7z"
    if (-not (Test-Path $mpvZip) -or (Get-Item $mpvZip).Length -lt 1MB) {
        Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $mpvZip -UseBasicParsing
    }

    $7zr = Join-Path $tempDir "7zr.exe"
    if (-not (Test-Path $7zr)) {
        Write-Host "Downloading 7zr..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri "https://www.7-zip.org/a/7zr.exe" -OutFile $7zr -UseBasicParsing
    }

    $extractDir = Join-Path $tempDir "mpv_extract"
    if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
    & $7zr x $mpvZip "-o$extractDir" -y | Out-Null

    $copied = Copy-FirstMatch $extractDir "libmpv-2.dll" (Join-Path $libDir "libmpv-2.dll")
    Write-Host "  -> $copied" -ForegroundColor Green
} else {
    Write-Host "libmpv-2.dll already present." -ForegroundColor Green
}

Write-Host "Done. DLLs in $libDir" -ForegroundColor Green
Get-ChildItem $libDir -Filter "*.dll" | Format-Table Name, Length -AutoSize
