#ifndef AppVersion
#define AppVersion "0.2.0"
#endif
#ifndef BuildDir
#define BuildDir "target\x86_64-pc-windows-msvc\release"
#endif

#define AppName "Flux Launcher CN"
#define AppPublisher "lvxingqi"
#define AppExeName "flux-launcher.exe"
#define AppDescription "A lightweight native Windows 11 launcher and file search tool"
#define AppUrl "https://github.com/lvxingqi/flux-launcher-cn"

[Setup]
AppId={{03BBAE09-3528-4CD6-A941-5553ABD33A6C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}/issues
AppUpdatesURL={#AppUrl}/releases/latest
AppCopyright=Copyright (C) 2026 m1nuzz; Copyright (C) 2026 lvxingqi
DefaultDirName={localappdata}\Programs\Flux Launcher CN
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline
CloseApplications=yes
RestartApplications=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64
OutputBaseFilename=FluxLauncher-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=flux-launcher.ico
UninstallDisplayIcon={app}\flux-launcher.ico
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppDescription}
VersionInfoProductName={#AppName}
VersionInfoCompany={#AppPublisher}
LicenseFile=..\..\LICENSE
OutputDir=..\..\artifacts\installer

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startup"; Description: "Start Flux Launcher CN automatically with Windows"; GroupDescription: "Windows startup:"

[Files]
Source: "flux-launcher.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BuildDir}\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Flux Launcher CN"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\flux-launcher.ico"; IconIndex: 0

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Flux Launcher CN"; ValueData: "{code:StartupCommand}"; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch Flux Launcher CN now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\{#AppExeName}"; Parameters: "--shutdown"; Flags: waituntilterminated skipifdoesntexist; RunOnceId: "FluxLauncherShutdown"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
Type: filesandordirs; Name: "{group}"
Type: filesandordirs; Name: "{userappdata}\FluxLauncherCN"
Type: filesandordirs; Name: "{userappdata}\FluxLauncher"

[Code]
function StartupCommand(Param: String): String;
begin
  Result := '"' + ExpandConstant('{app}\{#AppExeName}') + '" --startup';
end;
