param([Parameter(Mandatory=$true)][string]$Checker, [int]$Port=80)
$ErrorActionPreference='Stop'
$existing=Get-Service -Name Homeboard -ErrorAction SilentlyContinue
$wasRunning=$existing -and $existing.Status -eq 'Running'
try {
    if ($existing) { Stop-Service Homeboard -Force; (Get-Service Homeboard).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30)) }
    & $Checker --check-port --port $Port
    if ($LASTEXITCODE -ne 0) { throw 'Port not available' }
    exit 0
} catch {
    if ($wasRunning) { Start-Service Homeboard -ErrorAction SilentlyContinue }
    exit 1
}
