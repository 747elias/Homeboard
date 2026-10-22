$ErrorActionPreference='Stop'
try {
    if (Get-Service Homeboard -ErrorAction SilentlyContinue) {
        Stop-Service Homeboard -Force
        (Get-Service Homeboard).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30))
        & sc.exe delete Homeboard | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Service deletion failed' }
    }
    Get-NetFirewallRule -Name Homeboard-HTTP -ErrorAction SilentlyContinue | Remove-NetFirewallRule
    Remove-Item (Join-Path $PSScriptRoot 'homeboard.url'),(Join-Path $PSScriptRoot 'editor.url') -ErrorAction SilentlyContinue
    exit 0
} catch { exit 1 }
