# CAD Agent Studio

Ứng dụng Tauri 2 / React tiếng Việt, Rust và SQLite theo master prompt và các prompt tiếp nối.
Màn hình chính là Chat; mặc định chưa kết nối live. Trang Kết nối/Skills/Chẩn đoán
mở được trước project. Tài khoản và hội thoại live mở độc lập ngay khi khởi động; chọn Demo rõ ràng mới khởi động Mock CAD/MCP.
Có chat agent Demo, adapter OpenAI Responses theo project và MCP sidecar CAD mô phỏng.
**Foundation chưa hoàn thành; chưa có bridge AutoCAD hoặc DWG thật.**

## Chạy trên Windows

Xem [chẩn đoán Windows](docs/WINDOWS_SETUP.md). Cần Rust MSVC, C++ Build Tools,
Windows SDK, WebView2, Node và pnpm 10.18.3. Không dùng node_modules Linux cho Windows.

```powershell
cd "D:\PHAN DINH HOI\AutoCad_agnet"
powershell -File scripts/doctor.ps1
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` chạy Tauri; hook build sidecar rồi chạy Vite. Rust chỉ khởi động
sidecar stdio sau thao tác chọn Demo. `pnpm dev:web` chỉ xem giao diện, không giả lập thành công IPC.
Windows debug app đã build và mở được cửa sổ thật ngày 07/10/2026.
Xem [bằng chứng và giới hạn kiểm thử](docs/TESTING.md); chưa có installer NSIS hoặc bridge AutoCAD.

Bản chat-first mới: `powershell -File scripts/launch-chat-first.ps1` sau build.
Build ID trong Chẩn đoán/title, artifact path/hash trong
`docs/evidence/continue03-r1b/windows-startup.json`. R1B đã có global accounts/OAuth/API-key/standalone chat và live acceptance trên ChatGPT profile hiện tại. AutoCAD chưa tích hợp.

## Chat thật không cần Demo hoặc AutoCAD

Trong build R1B: **Kết nối → đồng ý quyền dữ liệu → Continue with ChatGPT**.
Đăng nhập browser; ứng dụng tự lấy catalog. Chọn model rồi Kiểm tra LLM (có thể dùng hạn mức). Có thể bấm Refresh models nếu cần.
API key là lựa chọn riêng, billing API; nhập trong app. Sau đó mở Chat để trao đổi brief.
CAD vẫn disconnected; app không giả đã vẽ. Probe/chat live đã PASS với gpt-5.6-luna.
Xem [bàn giao build và bằng chứng](HANDOFF.md).

## Demo không cần API key

1. Mở Kết nối → Chọn Demo mô phỏng, tạo dự án, chọn MockProvider và Execute.
2. Gửi: “Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm.”
3. Xem preview rồi Duyệt & áp dụng: 1 polyline, 4 circle, 3 dimension mô phỏng.
4. Gửi: “Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép.” rồi duyệt.
5. Đề xuất hoàn tác từ lịch sử và duyệt inverse change.

MockProvider chỉ hỗ trợ fixture đã công bố, không phải mô hình ngôn ngữ tổng quát.
Ask chỉ đọc; Plan chỉ lập phương án. Model không có công cụ approve/apply.

Provider OpenAI yêu cầu chọn model ID, bật đồng ý cloud cho dự án và lưu key qua
command chuyên biệt. Windows dùng Credential Manager; storage session phải chọn rõ
và mất khi đóng app. Không tự chuyển về mock khi live lỗi. Live API chưa được thử với key thật.

## Kiểm tra và đóng gói

```sh
pnpm test:core
pnpm test:mcp
pnpm typecheck
pnpm lint
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
```

`pnpm build` chỉ frontend. `pnpm package:windows` build installer development chưa ký,
bao gồm sidecar; cấu hình này chưa được xác minh bằng artifact Windows.
`pnpm test:desktop` là native WebDriver smoke, cần Windows và driver đang chạy.
CLI `pnpm demo:core` là harness in-process cũ, không chứng minh MCP/native.

## Dữ liệu

App Core giữ app.sqlite; sidecar giữ mock/state.json và receipt bền vững riêng.
SQLite lưu hội thoại chưa mã hóa, cấu hình chỉ có secret_ref; key không nằm trong DB.
Mất kết nối sau mutation cần đối chiếu receipt/readback, không tự gửi lại.
Xem [recovery](docs/RECOVERY.md), [kiểm thử](docs/TESTING.md),
[trạng thái](IMPLEMENTATION_STATUS.md) và [bàn giao](HANDOFF.md).

## Tiếp nối 02

Mốc T0 đã bổ sung Windows doctor/setup/start/build và audit bản cài AutoCAD 2026.1.2.
Chạy `scripts/setup-windows.ps1` để xem kế hoạch, không tự cài. Chi tiết tại
[CONTINUE_02_AUDIT.md](docs/CONTINUE_02_AUDIT.md). OAuth, skill runtime và live bridge
đã mở phạm vi nhưng **chưa triển khai trong lát cắt T0**, không phải live-ready.
