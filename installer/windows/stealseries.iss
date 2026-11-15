; Inno Setup script for the Windows installer (ADR 0006, docs/architecture/release.md).
;
;   iscc /DAppVersion=0.1.0 installer\windows\stealseries.iss
;   → target\installer\StealSeries-0.1.0-windows-x64-setup.exe
;
; Per-user install without admin rights, so in-app updates never show a UAC prompt. In-app
; updates run this installer with /VERYSILENT; it then starts the new version itself.

#ifndef AppVersion
  #error Pass the version: iscc /DAppVersion=X.Y.Z
#endif

[Setup]
; Never change the AppId: it ties updates and the uninstaller to earlier installs.
AppId={{BF2D6249-B55C-4547-9AB4-B20E42F9B1BA}
AppName=StealSeries
AppVersion={#AppVersion}
AppVerName=StealSeries {#AppVersion}
AppPublisher=StealSeries (independent, not affiliated with SteelSeries)
AppPublisherURL=https://github.com/leonseemann/stealseries
AppSupportURL=https://github.com/leonseemann/stealseries/issues
AppUpdatesURL=https://github.com/leonseemann/stealseries/releases
VersionInfoVersion={#AppVersion}
PrivilegesRequired=lowest
DefaultDirName={userpf}\StealSeries
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
LicenseFile=..\..\LICENSE-MIT
OutputDir=..\..\target\installer
OutputBaseFilename=StealSeries-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\stealseries.exe
SetupIconFile=..\..\crates\ss-app\bundle\icon\StealSeries.ico
UninstallDisplayName=StealSeries
; The app quits itself before an update; close it if it still runs (manual installs).
CloseApplications=force
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "..\..\target\release\stealseries.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; The AppUserModelID makes toasts show "StealSeries" (platform::init_notifications).
Name: "{userprograms}\StealSeries"; Filename: "{app}\stealseries.exe"; AppUserModelID: "StealSeries.App"
Name: "{userdesktop}\StealSeries"; Filename: "{app}\stealseries.exe"; Tasks: desktopicon

[Registry]
; "Start at login" is written by the app; remove it together with the app.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "StealSeries"; ValueType: none; Flags: uninsdeletevalue dontcreatekey
; Toast sender registration written by the app.
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\StealSeries.App"; ValueType: none; Flags: uninsdeletekey dontcreatekey

[Run]
; Interactive install: offer to start the app on the last page.
Filename: "{app}\stealseries.exe"; Description: "Start StealSeries"; Flags: nowait postinstall skipifsilent
; In-app update (silent): start the new version right away.
Filename: "{app}\stealseries.exe"; Flags: nowait; Check: WizardSilent
