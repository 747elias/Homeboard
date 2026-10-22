param([Parameter(Mandatory=$true)][string]$InstallDir,[ValidateRange(1,65535)][int]$Port=80,[switch]$OpenFirewall)
$ErrorActionPreference='Stop'
$data=Join-Path $env:ProgramData 'Homeboard'
New-Item -ItemType Directory -Force -Path $data | Out-Null
# Only Administrators, SYSTEM and the service identity can read/write passwords and data.
& icacls.exe $data /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-19:(OI)(CI)M' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Data ACL failed' }
$configPath=Join-Path $data 'server.json'
$bind='::'
if (Test-Path $configPath) { $old=Get-Content $configPath -Raw | ConvertFrom-Json; $bind=$old.bind }
@{bind=$bind;port=$Port} | ConvertTo-Json | Set-Content $configPath -Encoding Ascii
$binary='"'+(Join-Path $InstallDir 'homeboard.exe')+'" --service'
$existing=Get-CimInstance Win32_Service -Filter "Name='Homeboard'"
if ($existing) {
    $result=Invoke-CimMethod -InputObject $existing -MethodName Change -Arguments @{PathName=$binary;StartMode='Automatic';StartName='NT AUTHORITY\LocalService'}
} else {
    $result=Invoke-CimMethod -ClassName Win32_Service -MethodName Create -Arguments @{Name='Homeboard';DisplayName='Homeboard Backend';PathName=$binary;ServiceType=[byte]16;ErrorControl=[byte]1;StartMode='Automatic';DesktopInteract=$false;StartName='NT AUTHORITY\LocalService'}
}
if ($result.ReturnValue -ne 0) { throw "Service registration failed: $($result.ReturnValue)" }
& sc.exe description Homeboard 'Persönliches lokales Dashboard. Öffnet keinen Browser.' | Out-Null
& sc.exe failure Homeboard reset= 86400 actions= restart/10000/restart/30000/restart/60000 | Out-Null
Get-NetFirewallRule -Name Homeboard-HTTP -ErrorAction SilentlyContinue | Remove-NetFirewallRule
if ($OpenFirewall) { New-NetFirewallRule -Name Homeboard-HTTP -DisplayName 'Homeboard (privates Netzwerk)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort $Port -Program (Join-Path $InstallDir 'homeboard.exe') -Profile Private | Out-Null }
$suffix=if($Port -eq 80){''}else{':'+$Port}
"[InternetShortcut]`r`nURL=http://localhost$suffix/`r`n" | Set-Content (Join-Path $InstallDir 'homeboard.url') -Encoding Ascii
"[InternetShortcut]`r`nURL=http://localhost$suffix/edit`r`n" | Set-Content (Join-Path $InstallDir 'editor.url') -Encoding Ascii
Start-Service Homeboard
(Get-Service Homeboard).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
