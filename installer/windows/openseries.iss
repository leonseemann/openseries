; Inno Setup script for the Windows installer (ADR 0006, docs/architecture/release.md).
;
;   iscc /DAppVersion=0.1.0 installer\windows\openseries.iss
;   → target\installer\OpenSeries-0.1.0-windows-x64-setup.exe
;
; Per-user install without admin rights, so in-app updates never show a UAC prompt. In-app
; updates run this installer with /VERYSILENT; it then starts the new version itself.

#ifndef AppVersion
  #error Pass the version: iscc /DAppVersion=X.Y.Z
#endif

[Setup]
; Never change the AppId: it ties updates and the uninstaller to earlier installs.
AppId={{72CCD05A-DBBE-4F2F-9375-DDD225417826}
AppName=OpenSeries
AppVersion={#AppVersion}
AppVerName=OpenSeries {#AppVersion}
AppPublisher=OpenSeries (independent, not affiliated with SteelSeries)
AppPublisherURL=https://github.com/leonseemann/openseries
AppSupportURL=https://github.com/leonseemann/openseries/issues
AppUpdatesURL=https://github.com/leonseemann/openseries/releases
VersionInfoVersion={#AppVersion}
PrivilegesRequired=lowest
DefaultDirName={userpf}\OpenSeries
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
LicenseFile=..\..\LICENSE-MIT
OutputDir=..\..\target\installer
OutputBaseFilename=OpenSeries-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\openseries.exe
SetupIconFile=..\..\crates\ss-app\bundle\icon\OpenSeries.ico
UninstallDisplayName=OpenSeries
; The app quits itself before an update; close it if it still runs (manual installs).
CloseApplications=force
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "..\..\target\release\openseries.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; The AppUserModelID makes toasts show "OpenSeries" (platform::init_notifications).
Name: "{userprograms}\OpenSeries"; Filename: "{app}\openseries.exe"; AppUserModelID: "OpenSeries.App"
Name: "{userdesktop}\OpenSeries"; Filename: "{app}\openseries.exe"; Tasks: desktopicon

[Registry]
; "Start at login" is written by the app; remove it together with the app.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "OpenSeries"; ValueType: none; Flags: uninsdeletevalue dontcreatekey
; Toast sender registration written by the app.
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\OpenSeries.App"; ValueType: none; Flags: uninsdeletekey dontcreatekey

[Run]
; Interactive install: offer to start the app on the last page.
Filename: "{app}\openseries.exe"; Description: "Start OpenSeries"; Flags: nowait postinstall skipifsilent
; In-app update (silent): start the new version right away.
Filename: "{app}\openseries.exe"; Flags: nowait; Check: WizardSilent
