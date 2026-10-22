#define AppName "Homeboard"
#define AppVersion "0.1.0-beta"
#ifndef AppExe
  #define AppExe "..\target\x86_64-pc-windows-msvc\release\homeboard.exe"
#endif
[Setup]
AppId={{B23F1B13-563F-44E3-9E29-C151047510B4}
AppName={#AppName}
AppVersion={#AppVersion}
VersionInfoVersion=0.1.0.0
AppPublisher=Homeboard Contributors
AppPublisherURL=https://github.com/747elias/Homeboard
DefaultDirName={autopf}\Homeboard
DefaultGroupName=Homeboard
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=Homeboard-Setup-0.1.0-beta
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
UninstallDisplayIcon={app}\homeboard.exe
CloseApplications=no
RestartApplications=no
SetupLogging=yes
[Languages]
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
[Tasks]
Name: "firewall"; Description: "Port für Geräte im privaten Netzwerk freigeben"; Flags: unchecked
[Files]
Source: "{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#AppExe}"; DestName: "homeboard-portcheck.exe"; Flags: dontcopy
Source: "Install.ps1"; Flags: dontcopy
Source: "Prepare.ps1"; Flags: dontcopy
Source: "Uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
[Icons]
Name: "{group}\Homeboard – Dashboard"; Filename: "{app}\homeboard.url"
Name: "{group}\Homeboard – Editor"; Filename: "{app}\editor.url"
Name: "{group}\Homeboard deinstallieren"; Filename: "{uninstallexe}"
[Code]
var PortPage: TInputQueryWizardPage;
    SelectedPort: Integer;
    KeepData: Boolean;
function PSPath(): String;
begin Result := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'); end;
procedure InitializeWizard();
begin
  PortPage := CreateInputQueryPage(wpSelectDir, 'Lokaler Webserver', 'Nur das Backend startet mit Windows.', 'Port 80 ist Standard. Falls ein anderer Dienst ihn belegt, ist beispielsweise 8080 möglich. Das Dashboard wird niemals automatisch im Browser geöffnet.');
  PortPage.Add('HTTP-Port:', False);
  PortPage.Values[0] := ExpandConstant('{param:PORT|80}');
end;
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if CurPageID = PortPage.ID then begin
    SelectedPort := StrToIntDef(PortPage.Values[0], 0);
    if (SelectedPort < 1) or (SelectedPort > 65535) then begin
      MsgBox('Bitte einen Port zwischen 1 und 65535 eingeben.', mbError, MB_OK); Result := False;
    end;
  end;
end;
function PrepareToInstall(var NeedsRestart: Boolean): String;
var Code: Integer;
begin
  Result := '';
  SelectedPort := StrToIntDef(PortPage.Values[0], 0);
  if (SelectedPort < 1) or (SelectedPort > 65535) then begin Result := 'Ungültiger Port. /PORT=80 oder beispielsweise /PORT=8080 angeben.'; Exit; end;
  ExtractTemporaryFile('Prepare.ps1'); ExtractTemporaryFile('homeboard-portcheck.exe');
  if (not Exec(PSPath(), '-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + ExpandConstant('{tmp}\Prepare.ps1') + '" -Checker "' + ExpandConstant('{tmp}\homeboard-portcheck.exe') + '" -Port ' + IntToStr(SelectedPort), '', SW_HIDE, ewWaitUntilTerminated, Code)) or (Code <> 0) then
    Result := 'Port ' + IntToStr(SelectedPort) + ' ist belegt oder nicht verfügbar. Bitte IIS/andere Webserver prüfen oder zurückgehen und z. B. Port 8080 wählen. Ein vorhandener Homeboard-Dienst wurde wieder gestartet.';
end;
procedure CurStepChanged(CurStep: TSetupStep);
var Code: Integer; Firewall: String;
begin
  if CurStep = ssPostInstall then begin
    ExtractTemporaryFile('Install.ps1'); Firewall := '';
    if WizardIsTaskSelected('firewall') then Firewall := ' -OpenFirewall';
    if (not Exec(PSPath(), '-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + ExpandConstant('{tmp}\Install.ps1') + '" -InstallDir "' + ExpandConstant('{app}') + '" -Port ' + IntToStr(SelectedPort) + Firewall, '', SW_HIDE, ewWaitUntilTerminated, Code)) or (Code <> 0) then
      RaiseException('Der Homeboard-Dienst konnte nicht eingerichtet werden. Windows-Ereignisanzeige und ProgramData\Homeboard\logs prüfen.');
  end;
end;
function InitializeUninstall(): Boolean;
begin
  KeepData := True;
  if not UninstallSilent then KeepData := MsgBox('Daten und Einstellungen in ProgramData\Homeboard behalten?', mbConfirmation, MB_YESNO) = IDYES;
  Result := True;
end;
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Code: Integer;
begin
  if CurUninstallStep = usUninstall then begin
    if (not Exec(PSPath(), '-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\Uninstall.ps1') + '"', '', SW_HIDE, ewWaitUntilTerminated, Code)) or (Code <> 0) then
      RaiseException('Der Homeboard-Dienst konnte nicht entfernt werden. Dienststatus und Berechtigungen prüfen und die Deinstallation erneut starten.');
  end;
  if (CurUninstallStep = usPostUninstall) and (not KeepData) then DelTree(ExpandConstant('{commonappdata}\Homeboard'), True, True, True);
end;
