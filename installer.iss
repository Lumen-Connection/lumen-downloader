; Instalador do Lumen Stream (Inno Setup 6).
; Compilado no CI de release com: ISCC /DAppVersion=<versão> installer.iss
; A versão vem de fora (tag do git) para nunca divergir do Cargo.toml.

#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif

[Setup]
; AppId identifica o app para o Windows entre versões — NUNCA mude, ou
; atualizações passam a instalar em duplicidade em vez de substituir.
AppId={{6B75A2E3-3277-4E90-8579-6311907612F4}
AppName=Lumen Stream
AppVersion={#AppVersion}
AppPublisher=Lumen Connection
AppPublisherURL=https://github.com/Lumen-Connection/lumen-stream
AppSupportURL=https://github.com/Lumen-Connection/lumen-stream/issues
; Instalação por usuário (sem pedir admin/UAC): {autopf} resolve para
; %LocalAppData%\Programs, o mesmo modelo do VS Code/Discord.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Lumen Stream
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=LumenStream-Setup-{#AppVersion}
SetupIconFile=assets\LumenStreamIcon.ico
UninstallDisplayIcon={app}\lumen-stream.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "target\release\lumen-stream.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Lumen Stream"; Filename: "{app}\lumen-stream.exe"
Name: "{autodesktop}\Lumen Stream"; Filename: "{app}\lumen-stream.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\lumen-stream.exe"; Description: "{cm:LaunchProgram,Lumen Stream}"; Flags: nowait postinstall skipifsilent
