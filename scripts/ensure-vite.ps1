param(
    [switch]$WaitOnly
)

$port = 1420
$url = "http://localhost:$port/"
$root = Join-Path $PSScriptRoot ".."

function Test-PortListening {
    $conn = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    return $null -ne $conn
}

function Test-ViteHttp {
    try {
        $response = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 2
        return $response.StatusCode -ge 200
    } catch {
        return $false
    }
}

if (-not $WaitOnly -and -not (Test-PortListening)) {
    Write-Host "==> Starting Vite on port $port..." -ForegroundColor Cyan
    Start-Process -FilePath "npm.cmd" -ArgumentList "run", "dev" -WorkingDirectory $root -WindowStyle Hidden
}

$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $deadline) {
    if (Test-ViteHttp) {
        Write-Host "==> Vite ready at $url" -ForegroundColor Green
        exit 0
    }
    Start-Sleep -Milliseconds 500
}

Write-Host "ERROR: Vite did not start on $url within 90s." -ForegroundColor Red
Write-Host "Run manually: npm run dev" -ForegroundColor Yellow
exit 1
