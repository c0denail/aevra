#define AppVersion "0.1.0"
#define ProjectRoot AddBackslash(SourcePath) + ".."
#ifndef PayloadDir
  #define PayloadDir ProjectRoot + "\artifacts\Aevra-memory"
#endif

[Setup]
AppId={{D8909A50-29ED-4591-BAA2-3E43C9AC3547}
AppName=Aevra
AppVersion={#AppVersion}
AppPublisher=Aevra
DefaultDirName={localappdata}\Programs\Aevra
DefaultGroupName=Aevra
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19045
OutputDir={#ProjectRoot}\artifacts\installer
OutputBaseFilename=Aevra-{#AppVersion}-windows-x64-setup
SetupIconFile={#ProjectRoot}\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\Aevra.exe
WizardStyle=modern
Compression=lzma2/fast
SolidCompression=yes
DiskSpanning=yes
DiskSliceSize=1566000000
SlicesPerDisk=1
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: ".gitkeep,*.gguf"
Source: "{#PayloadDir}\resources\models\*.gguf"; DestDir: "{app}\resources\models"; Flags: ignoreversion nocompression
Source: "{#ProjectRoot}\.cache\WebView2RuntimeInstallerX64.exe"; Flags: dontcopy

[Tasks]
Name: "desktopicon"; Description: "Masaüstü kısayolu oluştur"; Flags: unchecked

[Icons]
Name: "{userprograms}\Aevra"; Filename: "{app}\Aevra.exe"
Name: "{userdesktop}\Aevra"; Filename: "{app}\Aevra.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Aevra.exe"; Description: "Aevra uygulamasını aç"; Flags: nowait postinstall skipifsilent

[Code]
function HasWebView2: Boolean;
var Version: String;
begin
  Result := (RegQueryStringValue(HKLM32, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'))
    or (RegQueryStringValue(HKCU32, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'));
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var ExitCode: Integer;
begin
  Result := '';
  if not HasWebView2 then begin
    ExtractTemporaryFile('WebView2RuntimeInstallerX64.exe');
    if not Exec(ExpandConstant('{tmp}\WebView2RuntimeInstallerX64.exe'), '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      Result := 'Microsoft WebView2 başlatılamadı.'
    else if (ExitCode <> 0) and (ExitCode <> 3010) then
      Result := 'Microsoft WebView2 kurulamadı. Hata: ' + IntToStr(ExitCode);
    NeedsRestart := ExitCode = 3010;
  end;
end;
