; Inno Setup script for PeaPod Windows (.tasks/02-windows.md §7.1).
; Build the exe first: cargo build -p pea-windows --release
; Then run: iscc /DSourceExe=..\..\target\release\pea_windows.exe PeaPod.iss
; Or from repo root: iscc /DSourceExe=target\release\pea_windows.exe pea-windows\installer\PeaPod.iss

#ifndef SourceExe
#define SourceExe "..\..\target\release\pea_windows.exe"
#endif

[Setup]
AppName=PeaPod
AppVersion=0.1.0
AppVerName=PeaPod 0.1.0
DefaultDirName={userpf}\PeaPod
DefaultGroupName=PeaPod
OutputDir=output
OutputBaseFilename=PeaPod-Setup-0.1.0
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
; Per-user install so no admin required; proxy is per-user (HKCU) anyway.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "pea_windows.exe"; Flags: ignoreversion
; Optional: add a README or license if desired later.

[Icons]
Name: "{group}\PeaPod"; Filename: "{app}\pea_windows.exe"; Comment: "PeaPod protocol (system tray)"
Name: "{group}\Uninstall PeaPod"; Filename: "{uninstallexe}"

[Run]
; Do NOT run the app after install (§7.1.2: installer does not set proxy by default; user enables in app).

[UninstallRun]
; §7.1.3: Restore system proxy to previous state if PeaPod was enabled.
; Runs before files are removed, so the exe is still present.
Filename: "{app}\pea_windows.exe"; Parameters: "--restore-proxy"; RunOnceId: "RestoreProxy"; StatusMsg: "Restoring system proxy..."

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True;
end;
