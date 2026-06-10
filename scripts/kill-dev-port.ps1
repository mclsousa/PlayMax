# Libera a porta do Vite (1420) antes de iniciar o Tauri dev.
$port = 1420
$killed = @()

$listeners = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
foreach ($conn in $listeners) {
    $owningPid = $conn.OwningProcess
    if ($owningPid -gt 0 -and $killed -notcontains $owningPid) {
        Write-Host "Encerrando processo $owningPid na porta $port..." -ForegroundColor Yellow
        Stop-Process -Id $owningPid -Force -ErrorAction SilentlyContinue
        $killed += $owningPid
    }
}

if ($killed.Count -eq 0) {
    # Fallback via netstat (alguns ambientes Windows)
    $netstat = netstat -ano | Select-String ":$port\s+.*LISTENING"
    foreach ($line in $netstat) {
        $procId = ($line -split '\s+')[-1]
        if ($procId -match '^\d+$' -and [int]$procId -gt 0 -and $killed -notcontains [int]$procId) {
            Write-Host "Encerrando PID $procId (netstat)..." -ForegroundColor Yellow
            Stop-Process -Id ([int]$procId) -Force -ErrorAction SilentlyContinue
            $killed += [int]$procId
        }
    }
}

if ($killed.Count -gt 0) {
    Start-Sleep -Milliseconds 200
}
