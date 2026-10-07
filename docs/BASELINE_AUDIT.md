# Baseline S0 — kiểm tra lại ngày 05/10/2026

Cwd `/mnt/d/PHAN DINH HOI/AutoCad_agnet` (Windows `D:\PHAN DINH HOI\AutoCad_agnet`).
Đã mở master/continue prompt, README, status, handoff, manifests, lockfile,
Tauri config/permissions, migration và test. `git status --short --branch` exit 128:
thư mục `.git` không phải Git repository hợp lệ; không xác định được branch/commit/dirty state.
Không init lại hoặc xóa file để thay đổi kết quả này.

Node 26.7.0, pnpm 10.18.3, rustc 1.99.0, Linux WSL x86_64.
Toolchain tạm từ phiên trước vẫn tồn tại; dùng CARGO_HOME=/tmp/cad-cargo,
RUSTUP_HOME=/tmp/cad-rustup, PATH thêm /tmp/cad-cargo/bin và /tmp/cad-tools.
PowerShell Get-Command cargo,node,pnpm không tìm thấy lệnh (exit 1); ordinary WSL interop
bị lỗi vsock. Chưa xác minh đầy đủ Windows SDK/Build Tools/WebView2.

| Capability trước sửa | Module/đường thật | Test rerun | Trạng thái | Bước nhỏ nhất tiếp theo |
|---|---|---|---|---|
| Desktop UI → Rust | App.tsx → ipc.ts → src-tauri/main.rs → Studio | Frontend only | not verified native | Doctor Windows + native runner |
| Project/migration | app-core/lib.rs, 0001 SQL | Persistence/FK test | implemented | Add conversations migration |
| Hình học mock | Studio ghi trực tiếp documents.geometry_json | 7 core tests | partial | Tách gateway có state qua MCP |
| Preview/approval | Studio.propose/approve/revert | Hash/TTL/session/revision/idempotency | implemented in process | Giữ checks khi qua process boundary |
| Contract | contracts/lib.rs → generated.ts/schema | 2 drift tests | implemented | Agent/wire contracts |
| Provider/chat/secrets | Không có | Không có | missing | ModelProvider + vault + fixtures |
| MCP/permit | Không có | Không có | missing | rmcp real stdio + MAC permit |
| UI | Form tham số, no mock invoke | 3 React tests | partial | Chat/settings giữ đường apply |
| Installer | Config + platform guard | Native check fail prerequisites | not verified | Package sidecar Windows |

## Lệnh baseline thực chạy (cwd root)

| Lệnh | Exit | Kết quả |
|---|---:|---|
| cargo test --locked | 0 | 7 core + 2 contract pass |
| pnpm test | 0 | 3 tests / 2 files pass |
| pnpm typecheck | 0 | TypeScript pass |
| pnpm lint | 0 | ESLint pass |
| pnpm build | 0 | Vite 82 modules; không phải exe |
| cargo check --locked -p cad-agent-studio | 101 | gdk-3.0.pc thiếu (gdk-sys); GUI chưa compile |

Baseline không có API key/live call, không có AutoCAD/DWG, không có native E2E.
Đây là log đánh giá ban đầu; trạng thái sau triển khai xem IMPLEMENTATION_STATUS.md.
