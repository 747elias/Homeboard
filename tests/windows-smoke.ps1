$ErrorActionPreference='Stop'
$port=18080
$install=Join-Path $env:ProgramFiles 'Homeboard'
$setup=(Resolve-Path 'dist/Homeboard-Setup-0.2.0-beta.exe').Path
$p=Start-Process $setup -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',"/PORT=$port",'/LOG=installer-smoke.log' -Wait -PassThru
if ($p.ExitCode -ne 0) { Get-Content installer-smoke.log -Tail 80; throw "Setup failed: $($p.ExitCode)" }
try {
    $svc=Get-CimInstance Win32_Service -Filter "Name='Homeboard'"
    if ($svc.StartMode -ne 'Auto' -or $svc.State -ne 'Running' -or $svc.StartName -notmatch 'LocalService') { throw 'Wrong service configuration' }
    $base="http://localhost:$port"
    $response=Invoke-WebRequest "$base/" -UseBasicParsing
    if ($response.StatusCode -ne 200 -or $response.Content -notmatch 'homeboard') { throw 'Dashboard not served' }
    $headers=@{Origin=$base}
    $login=Invoke-RestMethod "$base/api/auth/login" -Method Post -Headers $headers -ContentType 'application/json' -Body '{"password":"Windows-Smoke-Only-12345"}' -SessionVariable session
    $headers['X-CSRF-Token']=$login.csrf
    $config=Invoke-RestMethod "$base/api/editor/config" -WebSession $session
    $config.widgets[0].title='Windows-Dienst geprüft'
    $updated=Invoke-RestMethod "$base/api/editor/config" -Method Put -Headers $headers -ContentType 'application/json' -Body ($config|ConvertTo-Json -Depth 20) -WebSession $session
    if ($updated.revision -ne 1) { throw 'Save failed' }
    $tasks=Invoke-RestMethod "$base/api/tasks"
    $tasks.tasks=@([pscustomobject]@{id='windows-task';title='Aufgaben unter Windows speichern';status='todo';priority=2;project='personal'})
    $savedTasks=Invoke-RestMethod "$base/api/tasks" -Method Put -Headers $headers -ContentType 'application/json' -Body ($tasks|ConvertTo-Json -Depth 30) -WebSession $session
    if ($savedTasks.revision -ne 1 -or $savedTasks.tasks[0].title -ne 'Aufgaben unter Windows speichern') { throw 'Tasks save failed' }
    if ((Invoke-WebRequest "$base/tasks" -UseBasicParsing).StatusCode -ne 200) { throw 'Tasks page failed' }
    Restart-Service Homeboard
    (Get-Service Homeboard).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
    $after=Invoke-RestMethod "$base/api/dashboard"
    if ($after.widgets[0].title -ne 'Windows-Dienst geprüft') { throw 'Persistence failed' }
    $afterTasks=Invoke-RestMethod "$base/api/tasks"
    if ($afterTasks.tasks[0].id -ne 'windows-task') { throw 'Tasks persistence failed' }
    Stop-Service Homeboard
    & (Join-Path $install 'homeboard.exe') --reset-access --reset-password
    if ($LASTEXITCODE -ne 0) { throw 'Recovery failed' }
    Start-Service Homeboard
    $status=Invoke-RestMethod "$base/api/auth/status"
    if ($status.configured) { throw 'Password reset failed' }
    Write-Host 'Windows installer, LocalService, HTTP, auth, save, restart and recovery passed.'
} finally {
    $u=Start-Process (Join-Path $install 'unins000.exe') -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -Wait -PassThru
    if ($u.ExitCode -ne 0) { throw 'Uninstall failed' }
    if (Get-Service Homeboard -ErrorAction SilentlyContinue) { throw 'Service remains after uninstall' }
    if (Get-NetFirewallRule -Name Homeboard-HTTP -ErrorAction SilentlyContinue) { throw 'Firewall rule remains' }
    if (!(Test-Path (Join-Path $env:ProgramData 'Homeboard/dashboard.json'))) { throw 'Silent uninstall should preserve data' }
}
