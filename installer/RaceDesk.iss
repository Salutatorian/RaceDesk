#define AppVersion "0.1.1"
[Setup]
AppId=RaceDesk
AppName=RaceDesk
AppVersion={#AppVersion}
AppPublisher=RaceDesk contributors
DefaultDirName={localappdata}\Programs\RaceDesk
DisableDirPage=yes
DisableProgramGroupPage=yes
UsePreviousAppDir=no
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\..
OutputBaseFilename=RaceDesk-Setup-{#AppVersion}-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\assets\racedesk.ico
UninstallDisplayIcon={app}\RaceDesk.exe
CloseApplications=yes
RestartApplications=no
SetupMutex=RaceDeskInstaller
AppMutex=Local\RaceDesk-DesktopWidget-v1
VersionInfoVersion={#AppVersion}
VersionInfoDescription=RaceDesk Windows Installer
VersionInfoProductName=RaceDesk
VersionInfoProductVersion={#AppVersion}
[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"
[Files]
Source: "..\app\RaceDesk.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\racedesk.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\racedesk-logo.svg"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\racedesk-logo.png"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\app\collector\*"; DestDir: "{app}\collector"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\Uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion
[Icons]
Name: "{userprograms}\RaceDesk"; Filename: "{app}\RaceDesk.exe"; WorkingDir: "{app}"; IconFilename: "{app}\assets\racedesk.ico"
Name: "{userdesktop}\RaceDesk"; Filename: "{app}\RaceDesk.exe"; Tasks: desktopicon; IconFilename: "{app}\assets\racedesk.ico"
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\RaceDesk"; Flags: deletekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "RaceDesk"; ValueType: none; Flags: uninsdeletevalue
[Run]
Filename: "{app}\RaceDesk.exe"; Description: "Open RaceDesk"; Flags: nowait postinstall skipifsilent
[Code]
procedure CloseRaceDesk();
var
  Wnd: HWND;
  Attempt: Integer;
begin
  Wnd := FindWindowByWindowName('RaceDesk');
  if Wnd <> 0 then PostMessage(Wnd, $0010, 0, 0);
  for Attempt := 1 to 50 do
  begin
    if not CheckForMutexes('Local\RaceDesk-DesktopWidget-v1') then Break;
    Sleep(100);
  end;
end;

function InitializeSetup(): Boolean;
begin
  CloseRaceDesk();
  if CheckForMutexes('Local\RaceDesk-DesktopWidget-v1') then
  begin
    MsgBox('Please quit RaceDesk using its tray icon or the widget X, then run this installer again. Your settings will be kept.', mbInformation, MB_OK);
    Result := False;
  end
  else Result := True;
end;

function InitializeUninstall(): Boolean;
begin
  CloseRaceDesk();
  Result := not CheckForMutexes('Local\RaceDesk-DesktopWidget-v1');
end;
