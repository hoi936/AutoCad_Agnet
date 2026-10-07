# Chạy CAD Agent Studio trên Windows

Mục tiêu phát hành là desktop Windows với Rust MSVC. Kết quả frontend hoặc core
trên WSL không chứng minh ứng dụng Windows hay installer hoạt động.

## Kiểm tra môi trường

Mở PowerShell Windows thông thường trong thư mục dự án:

```powershell
cd 'D:\PHAN DINH HOI\AutoCad_agnet'
.\scripts\doctor.ps1
```

Doctor chỉ đọc PATH, phiên bản, registry và vị trí toolchain. Nó kiểm tra Node,
pnpm đúng `packageManager`, Rust host MSVC/target, Visual Studio C++ x64 linker,
Windows SDK và WebView2 Runtime. Không tự cài phần mềm, sửa execution policy,
thay registry hoặc yêu cầu quyền administrator. Doctor có thể báo thiếu nếu
toolchain cài ở vị trí không chuẩn; đối chiếu dòng báo và môi trường PowerShell.
Nếu chính sách của máy chặn script, sử dụng quy trình chạy script được quản trị viên
cho phép; không tắt bảo mật hay thay execution policy để bỏ qua kiểm tra.

Prerequisite chính thức: [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows).
Node phải đáp ứng yêu cầu của Vite trong lockfile (20.19+ hoặc 22.12+); nên dùng
bản LTS được hỗ trợ. Cài dependency bằng pnpm 10.18.3 trên Windows; không dùng
lại binary trong `node_modules` đã cài bởi WSL.

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` chạy Tauri và backend Rust cùng Vite. `pnpm dev:web` chỉ mở giao diện
trình duyệt; các command Rust và nút ghi bị khóa, không phải native E2E.

## Demo không cần API key hoặc AutoCAD

1. Tạo dự án, mở hội thoại; giữ MockProvider và CAD mock.
2. Gửi: `Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm.`
3. Kiểm tra preview; chỉ nút **Duyệt & áp dụng** mới ghi vào mô phỏng.
4. Gửi: `Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép.` và duyệt.
5. Dùng lịch sử để đề xuất hoàn tác, kiểm tra rồi duyệt phương án hoàn tác.

MockProvider chạy các fixture đã mô tả, không phải LLM tổng quát. Plan chỉ tạo
phương án; Ask chỉ đọc. AutoCAD chưa có bridge, không tạo DWG thật.

Các câu fixture lỗi dành cho kiểm tra trong chế độ mock: `[fixture:invalid_json]`,
`[fixture:forbidden_tool]`, `[fixture:timeout]`, `[fixture:401]`, `[fixture:429]`,
`[fixture:loop]`, `[fixture:cancel]`. Đây là lỗi mô phỏng, không phải kết quả gọi
OpenAI thật. Nút Dừng chờ backend xác nhận trạng thái; không tuyên bố hoàn tác.

OpenAI được cấu hình trong phần **Cài đặt provider và quyền dữ liệu** của dự án.
Nhập model ID có quyền sử dụng, đồng ý gửi ngữ cảnh theo dự án/provider rồi lưu
cấu hình. Nhập API key tại form mật khẩu của ứng dụng. Form xóa giá trị sau khi
gửi; không có command đọc lại khóa. Windows Credential Manager là nơi lưu bền
vững. **Chỉ phiên chạy này** là lựa chọn riêng, mất credential khi đóng ứng dụng.
Vault lỗi phải báo lỗi; không thay bằng file plaintext. Chọn live không âm thầm
chuyển sang mock. Các test mặc định dùng fixture, không gọi API tính phí.

## Kiểm tra và đóng gói

```powershell
pnpm typecheck
pnpm lint
pnpm test
pnpm test:core
pnpm test:mcp
pnpm build
pnpm package:windows
```

`pnpm build` chỉ tạo frontend assets. Installer NSIS và app executable là hai
artifact khác nhau. Script đóng gói cần Windows toolchain; đọc đường dẫn artifact
thật từ log build. Bản development chưa ký không phải bản phát hành production.
Không bật auto-update hoặc tắt Defender/SmartScreen để nghiệm thu.

Sau khi build thật, chạy native smoke theo [TESTING.md](TESTING.md), rồi kiểm tra
ứng dụng cài đặt trong đường dẫn có khoảng trắng: mở demo không key, tạo/resize/
hoàn tác qua duyệt, đóng/mở giữ hội thoại và đối chiếu CAD state. Kiểm tra vault
Windows và live OpenAI là các nhóm riêng; live chỉ chạy khi người dùng chủ động bật.

## Bằng chứng môi trường hiện tại (05/10/2026)

- Host thực thi: WSL/Linux. Native Linux check có log thiếu `gdk-3.0.pc`.
- Có đường dẫn `powershell.exe` nhưng WSL interop thông thường lỗi vsock; phép
  dò Windows PATH riêng không tìm thấy `cargo`, `node`, `pnpm` và trả exit code 1.
- Chưa có bằng chứng Rust MSVC/C++ SDK/WebView2 đầy đủ hay Windows build thành công.
- Windows native smoke: **NOT RUN — WINDOWS REQUIRED**.
- Windows credential integration: **NOT RUN — WINDOWS REQUIRED**.
- Installer: **NOT BUILT ON WINDOWS**. Live OpenAI: **LIVE NOT RUN**.
- AutoCAD thật: **NOT RUN — AUTOCAD REQUIRED**, ngoài phạm vi lượt này.

Các dòng trên không khẳng định mọi prerequisite đã bị gỡ hoặc không tồn tại trên
máy; cần chạy doctor trong đúng Windows shell để chẩn đoán đầy đủ.

## Sidecar trong build

Hook prepare-desktop build binary host, tạo tên có target triple trong
apps/desktop/src-tauri/binaries và externalBin đóng gói theo cấu hình Tauri.
Chỉ hỗ trợ build cùng host/target trong quy trình này; không tuyên bố cross-compile.
Backend dùng child process riêng, không cấp shell permission cho frontend.
Đối chiếu [quy tắc externalBin chính thức](https://v2.tauri.app/develop/sidecar/).
Windows credential smoke opt-in:

```powershell
cargo test -p cad-model-provider windows_credential_roundtrip_and_cleanup -- --ignored
```

Test ghi credential fixture trong namespace riêng rồi xóa, không dùng API key thật.
Chưa chạy test này trong phiên WSL.

## Continuation 02 — T0 (07/10/2026)

```powershell
.\scripts\doctor.ps1 -Profile desktop
.\scripts\doctor.ps1 -Profile bridge
.\scripts\setup-windows.ps1
.\scripts\test-windows-scripts.ps1
.\scripts\start-desktop.ps1 -CheckOnly
.\scripts\build-windows.ps1 -CheckOnly
```

Setup mặc định chỉ in kế hoạch. Không gọi -Install khi chưa cho phép cài phần mềm.
Khi được phép, chọn từng component và version chính xác đã xem bằng winget show;
script không tự lấy latest hoặc thay AutoCAD. WinGet xác minh manifest/hash, không dùng
ignore-security-hash và không tắt security. pnpm được khóa theo packageManager.
Installer có thể cần UAC/restart riêng; không chạy toàn bộ CAD Agent/AutoCAD quyền admin.
Mở lại terminal sau cài, doctor phải đạt trước start/build. Bỏ -CheckOnly để thực hiện.

WebView2 đã có trên máy khảo sát; không cần cài lại chỉ vì các prerequisite khác thiếu.
AutoCAD 2026.1.2 đã phát hiện; live plugin và .NET build SDK vẫn chưa có.
Log và giới hạn của phát hiện nằm trong CONTINUE_02_AUDIT.md / AUTOCAD_BRIDGE.md.

## Cài đặt đã được cho phép — 07/10/2026

Người dùng đã đồng ý cài các prerequisite Windows trong hội thoại; không cần hỏi lại
cho chính phạm vi này. Đã xác minh Node 24.12.0 và .NET SDK 10.0.300 có sẵn,
cài Rustup 1.29.1 (Rust MSVC 1.99.0), pnpm 10.18.3 và Build Tools 17.14.41.
Linker MSVC 14.44.35207, SDK headers 10.0.26100.0 và WebView2 154.0.4258.62 đã được doctor đọc.

Phát hiện phiên WSL truyền PATHEXT=.CPL: tên ngắn như node/pnpm không hoạt động đúng.
windows-native.ps1 gọi executable rõ ràng, giữ mã thoát, xử lý UTF-8/quotes và đặt
PATHEXT từ giá trị machine chỉ cho child process. Không sửa registry hoặc security policy.
Doctor cũ có false-negative; xem kết quả mới trước khi quyết định cài lại.

Dependency Linux đã giữ ở node_modules.linux (root và apps/desktop), dependency Windows
được cài mới theo lockfile. Không dùng lẫn hai tập dependency. Muốn quay về Linux phải
chuyển lại tập dependency tương ứng hoặc cài lại đúng platform; không sửa source/DB.

Lệnh kiểm chứng Windows từ repository:

```powershell
.\scripts\doctor.ps1
.\scripts\verify-windows-build.ps1
.\scripts\verify-windows-core.ps1
```

Script build chạy frontend checks và Tauri debug no-bundle; script core kiểm tra MCP
và credential fixture tự cleanup, không dùng API key thật. Bằng chứng build/test được
lưu trong docs/evidence/continue02; doctor PASS chưa chứng minh native build PASS.
