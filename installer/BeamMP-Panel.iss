; Build with scripts/tasks.ps1 installer (requires Inno Setup 7).
#if Ver < EncodeVer(7, 0, 0)
  #error Inno Setup 7 or newer is required.
#endif

#define AppName "BeamMP Panel"
#define AppExeName "BeamMP-Panel.exe"
#define AppBinary SourcePath + "..\dist\portable\" + AppExeName
#if !FileExists(AppBinary)
  #error Build the portable application before compiling the installer.
#endif
; The executable's resource version comes from Cargo.toml via winresource.
#define AppVersion GetStringFileInfo(AppBinary, "ProductVersion")

[Setup]
; Keep this ID unchanged so upgrades replace the existing installation.
AppId={{F8348797-F3C5-4882-B834-401109D668AA}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Aspenini
AppPublisherURL=https://github.com/Aspenini/BeamMP-Panel
AppSupportURL=https://github.com/Aspenini/BeamMP-Panel/issues
AppUpdatesURL=https://github.com/Aspenini/BeamMP-Panel/releases
DefaultDirName={localappdata}\Programs\BeamMP Panel
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
SetupIconFile=..\icons\BeamMP-Panel.ico
UninstallDisplayIcon={app}\{#AppExeName}
OutputDir=..\dist\installer
OutputBaseFilename=BeamMP-Panel-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
CloseApplicationsFilter={#AppExeName}
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#AppBinary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

; Only installed files are removed. Server folders and user settings are preserved.
