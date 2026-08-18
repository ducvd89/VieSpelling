; Kịch bản đóng gói cho Inno Setup.
;
; Cài vào **thư mục người dùng** chứ không phải Program Files, và đó là điều kiện
; để tính năng tải CUDA chạy được: ứng dụng ghi ba DLL runtime xuống cạnh chính
; file exe của nó, mà thư mục Program Files thì tiến trình không có quyền quản trị
; không ghi nổi. Cài theo người dùng cũng khỏi phải hỏi quyền lúc cài.
;
; Bản cài **không kèm DLL CUDA** — chúng nặng 493 MB, và phần lớn người dùng
; không cần: không có card NVIDIA thì mô hình không chạy được, còn các tầng luật
; thì vẫn làm việc đầy đủ. Ứng dụng tự mời tải khi nào người dùng chọn mô hình.

#define Ten "VieSpelling"
#define Ban "0.1.1"
#define Exe "vie-spellcheck.exe"

[Setup]
AppId={{8E2A6F14-3C7D-4B9E-9A21-VIESPELL0001}
AppName=Sửa chính tả EPUB tiếng Việt
AppVersion={#Ban}
AppPublisher=ducvd
AppPublisherURL=https://github.com/ducvd89/VieSpelling
DefaultDirName={localappdata}\Programs\{#Ten}
DefaultGroupName=Sửa chính tả EPUB
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=.
OutputBaseFilename=VieSpelling-{#Ban}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "vi"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Tạo lối tắt ngoài màn hình"; GroupDescription: "Lối tắt:"

[Files]
Source: "..\target\release\{#Exe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\vsc.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme

[Icons]
Name: "{group}\Sửa chính tả EPUB"; Filename: "{app}\{#Exe}"
Name: "{group}\Gỡ cài đặt"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Sửa chính tả EPUB"; Filename: "{app}\{#Exe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#Exe}"; Description: "Mở ứng dụng"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Ba DLL do ứng dụng tự tải về sau khi cài, nên bản gỡ không biết chúng — kê ra
; đây để gỡ xong không còn nửa GB nằm lại trong thư mục người dùng.
Type: files; Name: "{app}\cublas64_*.dll"
Type: files; Name: "{app}\cublasLt64_*.dll"
Type: files; Name: "{app}\cudart64_*.dll"
