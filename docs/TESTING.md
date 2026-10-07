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

# R1B — kết quả 07/10/2026

`scripts/verify-r1b.ps1` chạy Rust/vault, UI và native build; `-SkipNativeBuild` bỏ build.
Cwd Windows `D:\PHAN DINH HOI\AutoCad_agnet`; Node 24.12.0, pnpm 10.18.3,
Rust 1.99.0 MSVC. Evidence riêng: `docs/evidence/continue03-r1b/`.

- 49 core/contracts/provider/OAuth/DPAPI tests pass, 1 WinCred ignored mặc định.
  WinCred chạy opt-in riêng và cleanup pass: tổng 50 unique tests. Không cộng rerun.
- 16 frontend tests, typecheck/lint/Vite và native Tauri debug: exit 0.
- 22 PowerShell parser/guard checks và native argument test pass.
- Loopback bind/cancel/one-shot, DPAPI large Unicode/rotation/corrupt/delete/replacement
  failure và DB close/reopen chạy trên Windows. JWT fixtures ký thật kiểm signature,
  issuer/audience/nonce/subject/expiry. Tool loop kiểm output được replay vào model.
- Native startup: build r1b-1791343994, responding, 0 mock sidecar. UIAutomation không
  truy cập được navigation control: native IPC/UI E2E chưa chạy. Không coi startup là E2E.

Live OAuth/API inference/catalog/probe NOT RUN. User phải login trong build mới rồi
Refresh models → chọn model → Kiểm tra LLM. Probe chỉ PASS khi model gọi convert_length
1.25 m → mm, backend trả 1250 mm, gửi output lại và model completed. Text 1250 không đủ.
Sau đó thử chat brief khi CAD disconnected, rồi restart kiểm lịch sử. Không tự đổi route.

# Kiểm thử

## Continuation 03 — R1 đầu

25 Rust core/contracts/MCP integration tests và 12 frontend tests đạt trên Windows.
Ca mới: preview form 204 bị thu hồi sau prompt unsupported; old chat approval bị chặn;
provider/new proposal invalidation; migration 0003 giữ outcome_unknown; UI navigation
trước project và stale snapshot lệch run. Typecheck/lint/Vite/native build exit 0.
Evidence mới: `docs/evidence/continue03/`. Startup report ghi không spawn mock sidecar.
Native WebDriver script đã thêm bước kiểm tra unconnected → chọn Demo, **chưa chạy**.
OAuth/global accounts/live CAD/NSIS còn thiếu implementation; không tính 40+9 cũ vào lượt này.


## Kết quả Windows — 07/10/2026

Build source Tauri debug và startup cửa sổ thật: PASS. App chạy từ thư mục có
khoảng trắng/Unicode; có sidecar process. Đây chưa phải native IPC/WebDriver E2E.

`powershell -File scripts/verify-windows-build.ps1`: typecheck/lint, 9 frontend tests,
Vite và Tauri debug build exit 0. `powershell -File scripts/verify-windows-core.ps1`:
35 core/contracts/provider + 4 MCP tests và 1 vault roundtrip/cleanup đều pass.
Vault dùng credential fixture riêng; không gọi inference thật. Sau test fault-injection,
sidecar thường đã build lại exit 0.

Evidence: `docs/evidence/continue02/windows-*.txt`, `windows-native-startup.json`.
Doctor và PowerShell guard/argument tests exit 0. Toolchain và lệnh chạy xem
[Windows setup](WINDOWS_SETUP.md). Không chạy pnpm Linux trên node_modules Windows.

Live LLM/OAuth, AutoCAD read-only/write, native WebDriver và installer smoke: NOT RUN.


Core và frontend không cần AutoCAD/API key:

```sh
pnpm install --frozen-lockfile
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
pnpm test:core
pnpm test:mcp
pnpm typecheck
pnpm lint
pnpm test
pnpm build
cargo run --locked -p cad-app-core --example demo
```

Core test tạo database tạm: preview chưa ghi, hash mismatch, Ask/Plan chặn ghi,
reject, TTL, stale revision, đổi session, double approve, resize, inverse approval,
persistence, FK/cross-project rejection và rollback khi insert receipt lỗi.
Contract drift tests so Rust với TS/schema/fixture. Validator kiểm tra layer/role/radius/
coordinates/closed/measurement bằng so sánh với planner; chưa có CAD engine độc lập.

Frontend tests dùng React Testing Library; không gọi native backend.
Browser `dev:web` không mock invoke và không áp dụng hình học.

## Windows native IPC/WebDriver smoke (chưa chạy)

Cài prerequisite Tauri và tauri-driver theo
https://v2.tauri.app/develop/tests/webdriver/ và driver Edge khớp WebView2.
Chạy trên profile test riêng vì bài test tạo một project có tên Native test trong app data.

```powershell
pnpm --filter @cad/desktop tauri build --debug --no-bundle
cargo install tauri-driver --locked
# Terminal riêng, bind local:
tauri-driver --host 127.0.0.1
# Terminal test:
pnpm test:desktop
```

Script dùng W3C WebDriver gọi IPC từ WebView thật rồi assert project/proposal/approve/
readback/idempotency thông qua chat agent và MCP child process. Không giả lập invoke. Không thêm driver endpoint vào binary release.
Đây là native IPC smoke, chưa phải toàn bộ user-interaction E2E hoặc nghiệm thu Windows.
`CAD_TEST_BINARY` có thể chọn binary test; không dùng dữ liệu người dùng thật.

CI core/UI và Windows compile đã viết nhưng chưa chạy trên GitHub.
MCP process và HTTP fixture tests đã có; C# contracts, security fuzz và AutoCAD smoke chưa có.
AutoCAD thật: **NOT RUN — AUTOCAD REQUIRED**.

Nguồn WebDriver đã kiểm tra ngày 05/10/2026. Không suy luận browser pass = native pass.

## Coverage continuation 01

- Core: conversation/run persistence, Ask/Plan/Execute, malicious tools, repair/loop bounds,
  cancel, provider failure, cloud consent, redaction và migration từ 0001.
- Provider: deterministic fixtures, HTTP/SSE local fixtures cho auth/rate-limit/server error,
  timeout/cancel, malformed stream, tool call continuation và secret reflection.
- MCP: process thật, negotiation/discovery, permit tamper, paths có dấu cách,
  apply/readback/idempotency/restart và frame quá cỡ. Feature fault-injection kiểm tra
  process thoát sau commit; không bật feature này khi đóng gói.
- Core integration: chat → MCP → approve → resize → revert → restart; lỗi lưu receipt
  sau CAD commit phải khóa mutation tiếp và recovery không tăng revision lần nữa.
- UI: streaming event merge, composer/cancel, mode guards và form credential.

Test core integration yêu cầu binary cad-tool-host được build trước; pnpm test:core
thực hiện bước này. Test mặc định không gọi OpenAI thật và không cần API key.
Kết quả thực tế của phiên được ghi trong HANDOFF.md; CI/native/live là các mức kiểm chứng riêng.

## Continuation 02 — T0

`pnpm test:windows-scripts` chạy PowerShell parser và setup guard fixtures trên Windows.
Không cài prerequisite. Audit 07/10/2026: 10 checks đạt; start/build -CheckOnly chặn đúng
vì thiếu toolchain. Test Windows scripts không phải native desktop hoặc vault test.
Rust toolchain tạm đã mất; chỉ chạy lại cached binaries (40 pass), chưa rebuild Rust.
Xem CONTINUE_02_AUDIT.md và docs/evidence/continue02 để tránh nhầm với lượt trước.
