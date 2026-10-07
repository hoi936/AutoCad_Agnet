# R1B live verification — 07/10/2026

Đã xác minh qua AppRuntime native, dùng đúng profile ChatGPT đã đăng nhập và model
user chọn `gpt-5.6-luna`: catalog 4 models, function call `convert_length` với
1.25 m → 1250 mm, output gửi lại model và turn completed, standalone chat nhà 8×12 m.
Runtime đóng/mở lại giữ 4 messages cùng selected model. CAD disconnected, không Demo.
Evidence: `docs/evidence/continue03-r1b/windows-live-runtime.json` và `.txt` (Exit 0),
`windows-ui-inspect.json`, `windows-ui-chat-completed.json`, `windows-ui-restart-history.json`.
Bản cuối: `target/native-smoke/Chat R1B Final 92CF9ABF39E6/cad-agent-studio.exe`,
build ID `r1b-1791345882`, 31,190,528 bytes, SHA256
`92CF9ABF39E6F10A21EABFE4057496E062D8FEA3C7E3E3715D739C4CA2796704`, PID 11776.
App startup responding, 0 mock sidecar children.
Backend live acceptance và native UI probe/chat được kiểm riêng; cả hai PASS.

Lỗi policy user báo trước đây là lịch sử: lần đăng nhập tiếp đã xác minh được trong UI,
và credential đó đã gọi catalog/inference thành công. Không còn dùng lỗi cũ làm blocker.

Sửa adapter: cho phép Content-Type vắng mặt nhưng vẫn parse SSE có giới hạn và chờ
response.completed; dựng output từ output_item.done nếu terminal output rỗng, kiểm tra
identity/arguments/duplicate và không thực thi khi stream bị ngắt. Không log raw SSE.
Các lỗi parser có mã theo giai đoạn. UI tự lấy catalog sau login, không gọi inference
ngầm; lỗi catalog không xóa kết quả xác minh; tiêu đề ghi rõ CAD chưa kết nối.

53 Rust/vault tests (52 trong suite + 1 WinCred riêng), 19 frontend tests đã pass.
Typecheck/lint/Vite/native build + startup pass.
Scan read-only 5 bảng ứng dụng không thấy mẫu API key/JWT; đây là pattern check giới hạn,
không phải chứng minh tuyệt đối không có mọi loại secret. Vault/fixture tests là evidence riêng.

Runner tái kiểm chứng: `scripts/verify-r1b-live.ps1 -Live` (đóng desktop trước; dùng hạn mức
ChatGPT, chỉ chấp nhận đúng một profile ChatGPT, không fallback API key). Runner tạo hội
thoại kiểm thử, giữ model đang chọn hoặc model đầu tiên từ catalog nếu chưa chọn model.
Baseline: `scripts/verify-r1b.ps1`. Không chạy hai runner đồng thời do Windows khóa exe.

Native UI probe và chat thật đã PASS; có evidence riêng.
Refresh/revoke HTTP E2E và AutoCAD live chưa nghiệm thu. R1B đã đạt lát cắt
chat độc lập; bước tiếp theo là R2 AutoCAD read-only với gate riêng.

---

## OAuth live — lỗi ban đầu, đã được vượt qua — 07/10/2026

User đã thử đăng nhập và báo trang xác thực hiển thị:
- `error_code: 3p_delegated_access_policy_denied`
- `request_id: f40fbeed-6bdb-4095-babe-73c3fab8b378`
- “Tổ chức của bạn chưa cho phép ứng dụng này truy cập”.

Trạng thái: **BLOCKED_EXTERNAL (theo báo cáo user)** tại bước cấp quyền.
Chưa có callback/token exchange thành công được kiểm chứng; catalog/inference/probe
live vẫn NOT RUN. Đây không phải HTTP trace do test runner thu thập. Chưa xác định
workspace được chọn hoặc cấu hình ứng dụng khác mà user đăng nhập thành công.
Thông báo cho thấy policy từ chối yêu cầu hiện tại; không chứng minh mọi phần tích hợp
đã đúng. Cần kiểm tra quyền của workspace/client với quản trị viên hoặc OpenAI support,
kèm request ID. Không đổi client thành Codex, nhập token ứng dụng khác hoặc tự chuyển billing.

Thông tin trước lần thử này được giữ bên dưới làm lịch sử.

# R1B — 07/10/2026 — chưa nghiệm thu live

| Capability | Code status | Test kind | Result | Environment | Evidence | Remaining blocker |
|---|---|---|---|---|---|---|
| Account/chat độc lập Demo/CAD | Implemented | Core + frontend | FIXTURE_PASS | Windows | r1b/windows-rust.txt; windows-test.txt | Live login/inference |
| OAuth PKCE/JWT/loopback/cancel | Implemented | Signed JWT + real loopback fixtures | FIXTURE_PASS / NATIVE_PASS loopback | Windows | r1b/windows-rust.txt | Browser exchange live NOT RUN |
| OAuth DPAPI bundle / API WinCred | Implemented | Native vault fixtures | NATIVE_PASS | Windows user | r1b/windows-rust.txt; windows-wincred.txt | Token thật/refresh live NOT RUN |
| Catalog/profile/model isolation | Implemented | Route/schema/DB fixtures | FIXTURE_PASS | Windows | r1b/windows-rust.txt; windows-restart.txt | Catalog thật NOT RUN |
| Standalone tool loop/probe | Implemented | 0/1/multiple/invalid/cancel/budget | FIXTURE_PASS | Windows | r1b/windows-rust.txt | Live text + tool calling NOT RUN |
| Conversation/model restart | Implemented | SQLite close/reopen | NATIVE_PASS fixture | Windows | r1b/windows-restart.txt | GUI restart với account thật |
| Native build/startup | Implemented | Tauri debug + process/window | NATIVE_PASS startup | Windows | r1b/windows-native-build.txt; windows-startup.json | Full UI/IPC E2E NOT RUN |
| OAuth/live chat/probe | Implemented, chưa live test | Chờ user login qua app | NOT RUN | — | HANDOFF.md | User tương tác đăng nhập/consent |
| AutoCAD/NSIS | NOT_IMPLEMENTED | Ngoài trọng tâm R1B | NOT RUN | — | Lịch sử bên dưới | Hoàn tất R1B trước R2 |

`r1b/` trong bảng là `docs/evidence/continue03-r1b/`. 50 tests Rust/vault và 16 frontend
pass lần này; không cộng rerun/40+9 cũ. R1B **chưa PASS toàn bộ** cho đến khi live
identity/permission/catalog/text/tool-call/chat được kiểm chứng. UIAutomation không thấy
navigation control, nên chỉ startup native PASS, không lấy đó thay native UI/IPC E2E.

# Continuation 03 — R1 lát cắt đầu — 07/10/2026

| Capability | Code status | Test kind | Result | Environment | Evidence | Remaining blocker |
|---|---|---|---|---|---|---|
| Chat-first shell, navigation trước project | Implemented | React/typecheck/lint | FIXTURE_PASS | Windows | continue03/windows-test.txt | Chat live/Ask không document chưa có |
| Demo opt-in, lazy MCP, backend gate | Implemented | Native build; startup kiểm riêng | NATIVE_PASS build | Windows | continue03/windows-native-build.txt | UI/IPC E2E chưa chạy |
| Stale proposal invalidation | Implemented slice | Core + React | FIXTURE_PASS | Windows | continue03/windows-core.txt | Full provenance/source_message + account/document switch chưa đủ |
| Migration 0004 + recovery preservation | Implemented | Upgrade 0001/0003 + unknown receipt | FIXTURE_PASS | Windows SQLite DELETE/FULL | continue03/windows-core.txt | Chưa có backup tự động |
| Build ID / launcher bản mới | Implemented | Native build + startup report | NATIVE_PASS startup; 0 mock child | Windows | continue03/windows-startup.json | Chưa installer |
| OAuth/OIDC, global accounts, catalog/probe | NOT_IMPLEMENTED | Docs compatibility reviewed | NOT RUN live | — | docs/AUTH_COMPATIBILITY.md | Cần code; không phải chờ user key |
| AutoCAD read-only/write, skill runtime, floorplan | NOT_IMPLEMENTED | Chưa có | NOT RUN | — | docs/CONTINUE_03_DELTA.md | R2–R4 |
| Native UI E2E / NSIS | Chưa nghiệm thu | Chưa chạy | NOT RUN | — | docs/TESTING.md | R5 |

Evidence prefix: `docs/evidence/`. R1 **chưa hoàn thành**; kết quả 02 bên dưới là lịch sử.
Lượt 03 chạy 25 core/contracts/MCP integration tests và 12 frontend tests; không cộng
40+9 của đợt 02, không gọi đây là live inference/AutoCAD PASS.

# Continuation 02 — T0 native — 07/10/2026

| Capability | Implementation status | Test type | Test result | Environment | Evidence | Remaining blocker |
|---|---|---|---|---|---|---|
| Windows prerequisites + doctor | NATIVE_PASS | Native executable/version/SDK checks | Exit 0 | Windows MSVC | windows-doctor-final.txt | Không còn blocker build desktop |
| Desktop debug build/startup | NATIVE_PASS (startup) | Tauri source build + window/process | Exit 0; cửa sổ responding, sidecar chạy | Windows, đường dẫn Unicode | windows-native-build.txt; windows-native-startup.json | Native IPC/UI E2E chưa chạy |
| Frontend baseline | FIXTURE_PASS | Typecheck/lint/Vitest/Vite | Exit 0; 9 tests | Windows Node 24.12.0 | windows-typecheck/lint/test/build.txt | Không thay native UI E2E |
| Rust source baseline | FIXTURE_PASS | Core/contracts/provider + MCP processes | Exit 0; 35 + 4 tests | Windows Rust 1.99.0 MSVC | windows-core.txt; windows-mcp.txt | Live inference/CAD chưa chạy |
| Windows Credential Manager | NATIVE_PASS | Fixture credential roundtrip + cleanup | Exit 0; 1 test | Windows user vault | windows-vault.txt | Chưa dùng API key thật |
| Windows scripts | FIXTURE_PASS | Parser/setup guards/native argument quoting | Exit 0 | Windows PowerShell | windows-scripts-final.txt | Không phải installer smoke |
| Normal sidecar restored | NATIVE_PASS | Build without fault-injection | Exit 0 | Windows | windows-sidecar-normal.txt | — |
| AutoCAD detection | NATIVE_PASS (detection) | Registry + PE/assembly metadata | 2026.1.2 / R25.1.179.0.0 | Windows | doctor-bridge.txt | Chưa plugin/handshake |
| T1 navigation/global accounts | NOT_IMPLEMENTED | Chưa có | NOT RUN | — | CONTINUE_02_AUDIT.md | Triển khai lát cắt tiếp |
| T2 OAuth/token vault/provider probes | NOT_IMPLEMENTED | Chưa có | NOT RUN | — | CONTINUE_02_AUDIT.md | API-key adapter cũ có fixture; chưa OAuth |
| T3 skill runtime + ba skill | NOT_IMPLEMENTED | Chưa có | NOT RUN | — | docs/SKILLS.md | Loader/router/provenance/primitives |
| T4–T5 C# live bridge | NOT_IMPLEMENTED | Chưa có | NOT RUN | Windows .NET SDK 10.0.300 sẵn có | docs/AUTOCAD_BRIDGE.md | Code, pairing, readback/write |
| T6 live E2E + installer | NOT_IMPLEMENTED nghiệm thu | Chưa có | NOT RUN | — | CONTINUE_02_AUDIT.md | T1–T5; chưa build NSIS |

Evidence filenames trong bảng nằm ở `docs/evidence/continue02/`. Tổng lần chạy
Windows: 40 Rust/MCP/vault tests + 9 frontend tests; không cộng kết quả cached Linux.
App executable debug đã có; chưa có installer hoặc bridge DLL.

Bảng bên dưới là kết quả **lượt 05/10**, không phải kết quả mới của continuation 02.
Giới hạn phạm vi đợt 01 không được áp dụng để loại OAuth/bridge khỏi đợt 02.

# Trạng thái triển khai — 05/10/2026

Continuation 01 có đường headless chat → MCP process → mock CAD → readback.
**Foundation tổng thể chưa hoàn thành. Native Windows, credential Windows và live OpenAI chưa nghiệm thu.**

| Capability | Implemented? | Test kind | Test result | Environment | Evidence | Remaining |
|---|---|---|---|---|---|---|
| Project, conversation, run, events và migrations | Có | Core integration | PASS | Linux/WSL | app-core agent_tests.rs, migrations 0001–0003 | Retention/backup |
| MockProvider và agent tool loop | Có | Unit/integration | PASS | Linux/WSL | 15 core tests, provider mock tests | Không phải LLM tổng quát |
| Ask/Plan/Execute, hash/TTL/session/revision | Có | Policy tests | PASS | Linux/WSL | agent_tests.rs, core tests | Native interaction |
| MCP client + sidecar rmcp 3.5.0 | Có | Process thật | PASS | Linux/WSL | mcp_process.rs | Windows process lifecycle |
| Permit HMAC, frame bounds, duplicate protection | Có | MCP/permit tests | PASS | Linux/WSL | mcp_process.rs | OS sandbox, fuzz suite |
| Chat → MCP → create/resize/revert/reopen | Có | Headless integration | PASS | Linux/WSL | mcp_acceptance.rs | Native Windows E2E |
| Outcome unknown và recovery | Có | Commit/disconnect + local receipt fault | PASS | Linux/WSL | mcp_process.rs, mcp_acceptance.rs | Mất điện thật / DWG recovery |
| Import hình học cũ sang snapshot | Có | Migration/import integration | PASS | Linux/WSL | mcp_acceptance.rs + 0001 migration test | Backup/restore tooling |
| OpenAI Responses adapter | Có | HTTP/SSE local fixtures | PASS | Linux/WSL | 14 provider tests gồm mock/vault/HTTP | LIVE NOT RUN |
| WindowsCredentialStore | Có | Linux unsupported/session tests | PASS phần Linux | Linux/WSL | secret.rs, provider_service.rs | Windows ignored smoke NOT RUN |
| Chat/settings/preview UI | Có | React tests, typecheck/lint/build | PASS | Linux/WSL | 9 UI tests / 4 files | Native UI runtime |
| Sidecar externalBin + prepare hooks | Có | Script syntax + sidecar relocated process | PASS phạm vi nêu | Linux/WSL | prepare-desktop.mjs, tauri.conf.json | Windows hook/package NOT RUN |
| Native desktop Rust/IPC | Có source | cargo check | BLOCKED prerequisites | Linux/WSL | glib-2.0.pc / gobject-2.0.pc thiếu; baseline gdk-3.0.pc | Windows build/driver smoke |
| Windows NSIS installer | Có config/script | Chưa build | NOT BUILT | Chưa có Windows toolchain dùng được | package-windows.mjs, doctor.ps1 | Artifact exe/size/hash |
| CI Linux/Windows | Có workflow | Chưa chạy CI | NOT RUN | GitHub runner chưa chạy | .github/workflows | Chạy trên repository Git thật |
| C# bridge / AutoCAD / DWG | Chưa | Ngoài phạm vi continuation 01 | NOT RUN | AutoCAD required | Backlog master | Bridge, pairing, DWG readback/recovery |
| Skill registry/permissions/hash runtime | Chưa | Bundled files hiện có | NOT RUN runtime | — | docs/SKILLS.md | Registry theo master |
| SQLite patched WAL / backup / retention | Chưa | DELETE/FULL được dùng hiện tại | Partial | SQLite 3.46.0 | docs/DATABASE.md | Patched engine và recovery |

Lệnh/kết quả cụ thể xem HANDOFF.md. Không dùng frontend build làm bằng chứng có exe.
