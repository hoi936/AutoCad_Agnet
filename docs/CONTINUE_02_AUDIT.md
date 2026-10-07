> Cập nhật native 07/10/2026: Tauri debug build + startup cửa sổ thật PASS.
> Windows source tests: 40 Rust/MCP/vault + 9 frontend PASS. Doctor exit 0.
> Evidence: `docs/evidence/continue02/windows-*.txt`, `windows-native-startup.json`.
> Các blocker toolchain ở audit ban đầu bên dưới là lịch sử đã được gỡ;
> native IPC/WebDriver, installer, OAuth và AutoCAD live vẫn chưa nghiệm thu.

> Cập nhật sau khi người dùng cho phép cài: doctor desktop đã PASS trên Windows.
> Node 24.12.0/.NET SDK 10.0.300 có sẵn; báo thiếu trước đây một phần do PATHEXT=.CPL
> của phiên WSL. Đã sửa chẩn đoán native; Rust MSVC, pnpm, C++/SDK và dependency
> Windows đã được chuẩn bị. Các số liệu T0 bên dưới giữ nguyên làm lịch sử,
> không dùng để kết luận toolchain hiện tại vẫn thiếu. Build native được kiểm chứng riêng.

# Audit continuation 02 — T0 — 07/10/2026

Cwd: `/mnt/d/phan dinh hoi/autocad_agnet`, đường Windows tương ứng
`D:\PHAN DINH HOI\AutoCad_agnet`. Git status trả 128: không có Git repository hợp lệ;
không xác định branch/commit hoặc khẳng định working tree sạch. Không init repo mới.
Phạm vi 02 đã mở lại OAuth và live AutoCAD; không dùng giới hạn continuation 01 để bỏ chúng.
Lát cắt hiện tại hoàn thành audit và scripts T0, chưa hoàn thành T1–T6.

| Capability | Đường code thật | Đường UI thật | Test đang có | Trạng thái | Việc còn thiếu |
|---|---|---|---|---|---|
| Desktop | Tauri main.rs → Studio::open_mcp | pnpm dev, native invoke | WebDriver script | BLOCKED_EXTERNAL | Windows development prerequisites |
| Chat/provider API key | AgentService + cad-model-provider | ChatPanel/ProviderSettings thuộc project | HTTP fixtures, 9 React tests | FIXTURE_PASS bản nền | Global accounts, catalog, capability/inference/tool probe |
| OAuth ChatGPT | Chưa có adapter/vault token | Chưa có | Chưa có | NOT_IMPLEMENTED | Eligibility chính thức, PKCE/OIDC, registration, DPAPI, fixtures |
| MCP | rmcp 3.5.0 stdio process | Đi qua backend, không arbitrary invoke | Process tests + acceptance | FIXTURE_PASS cached artifacts | Live gateway, lifecycle UI |
| CAD mock | Sidecar snapshot + permit + readback | Preview/approve/recovery/history | Create/resize/revert/import/recovery | FIXTURE_PASS cached artifacts | Không thay bằng chứng DWG |
| Skill | Một package mounting-plate, planner hardcoded | Chưa có registry UI | Geometry tests | NOT_IMPLEMENTED runtime | Loader/router/provenance và ba skill |
| AutoCAD | Chưa có C# plugin/pipe gateway | Nút chưa tích hợp | Chưa có live/simulator | NOT_IMPLEMENTED | Profile/build, protocol, pairing, read-only rồi mutation |
| Persistence | SQLx 0.9, migrations 0001–0003 | Project/chat/history | Migration tests | FIXTURE_PASS cached artifacts | Account/skills/live target schema, retention/backup |
| T0 scripts | doctor/setup/start/build Windows | PowerShell | 10 setup/parser checks + launch guards | FIXTURE_PASS | Installation chỉ sau consent, native build |

## Baseline lần này

Node Linux 26.7.0, pnpm 10.18.3. Rust/cargo không còn trên PATH và toolchain /tmp
của phiên trước đã mất. Không đổi lockfile hoặc khẳng định đã build Rust mới.

| Lệnh (cwd root) | Exit | Kết quả |
|---|---:|---|
| git status --short | 128 | Không có Git metadata hợp lệ |
| pnpm typecheck | 0 | TypeScript |
| pnpm lint | 0 | ESLint |
| pnpm test | 0 | 9 tests / 4 files |
| pnpm build | 0 | Vite 7.3.6, 85 modules; frontend only |
| pnpm test:core | 1 qua pnpm | cargo: not found |
| pnpm test:mcp | 1 qua pnpm | cargo: not found |
| 5 binary test cũ trong target/debug/deps | 0 mỗi binary | 40 ca chạy lại đạt; không phải rebuild source |

Danh tính/hash binary và log từng suite: [cached-baseline.json](evidence/continue02/cached-baseline.json).
Không thay số test baseline bằng số từ báo cáo lượt cũ: các artifact trên đã thực sự chạy lại.

## Windows và AutoCAD quan sát được

PowerShell Windows thực thi được qua WSL. WebView2 154.0.4258.62 đã có.
Không tìm thấy Node/pnpm/Rust/.NET SDK trên Windows PATH; không tìm thấy C++ workload,
Windows SDK headers hay Windows Tauri CLI dependency. Không có sidecar .exe hoặc plugin bundle.

Registry báo AutoCAD 2026.1.2 Update, version 25.1.179.0; acad.exe file/product
R25.1.179.0.0; AcMgd/AcDbMgd/AcCoreMgd assembly version 25.1.0.0.
Có thêm AutoCAD Mechanical 2024 nhưng không chọn làm profile mục tiêu.
Process acad thay đổi từ 0 sang 1 trong các lần đọc; không suy ra pairing/connected.
Chưa đọc runtimeconfig thành công và chưa nạp plugin. Không đụng DWG hoặc cấu hình AutoCAD.

Nguồn chính thức đã đối chiếu: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/),
[WinGet install](https://learn.microsoft.com/en-us/windows/package-manager/winget/install),
[Autodesk Managed .NET compatibility](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6C680F2-DE2E-418A-A182-E4884073338A.htm).
Autodesk ghi .NET 10 cho AutoCAD 2026 Update 1.2 trở đi; đây là cơ sở chọn profile ứng viên,
không phải chứng nhận DLL đã nạp hoặc runtime trên máy đã được chạy thử.

## Scripts mới và kết quả

- doctor.ps1: nhóm Desktop / dependencies-artifacts / bridge, đọc version lockfile,
  registry, PE/assembly metadata; không chạy AutoCAD API hay sửa registry.
- setup-windows.ps1: mặc định plan, cài từng component bằng -Install sau explicit consent;
  đòi version cụ thể, kiểm tra WinGet trước; pnpm lấy packageManager; không latest fallback.
- start-desktop.ps1 / build-windows.ps1: doctor gate, -CheckOnly không chạy ứng dụng/build;
  pnpm chạy từ root đúng path có dấu cách. Không tạo vòng startup hoặc spawn sidecar thứ hai.
- test-windows-scripts.ps1: parser + no-install/invalid-version/WhatIf guards. Không cài gì.

Log đã làm sạch nằm trong [evidence/continue02](evidence/continue02).
Setup default exit 0, tests exit 0 (10 checks); doctor/launch/build guards exit 1 vì thiếu
prerequisite là kết quả chặn đúng, không phải native pass. Cài đặt thật chưa chạy.
