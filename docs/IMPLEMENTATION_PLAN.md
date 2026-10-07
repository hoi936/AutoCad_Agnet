> Cập nhật continuation 01: chat/runtime, provider/vault, MCP mock và journal recovery đã có mã.
> Xem IMPLEMENTATION_STATUS.md để biết kết quả chạy thực; các mục kế hoạch bên dưới là
> phạm vi Foundation tổng thể, không phải báo cáo rằng mọi mục còn thiếu hoặc đã nghiệm thu.

# Kế hoạch thực hiện

Ngày 05/10/2026. Repository ban đầu chỉ có master prompt. Môi trường WSL, Node 26.7;
thiếu Rust, pnpm, GTK/WebKit Linux và AutoCAD SDK đã xác minh.

- M0: workspace Rust/Tauri/React, health IPC, permissions, shell; nghiệm thu native riêng.
- M1: migration SQLite, project persistence; tiếp theo conversations, secrets, providers.
- M2: planner/validator, preview, immutable approval, mock apply/readback/resize/revert;
  tiếp theo tách gateway qua rmcp sidecar và permit.
- M3: agent, skill registry, live Responses adapter, recovery đa tiến trình.
- M4: bridge C# theo SDK thực tế, Windows native tests, installer.

Quyết định: xây slice mock trực tiếp trong core trước khi tách transport; không gọi slice này là MCP.
Không tự sử dụng .NET Windows hay cài SDK Autodesk chưa biết phiên bản. Không tạo dummy DLL/DWG.
Mỗi capability chưa làm phải được ghi rõ trong IMPLEMENTATION_STATUS.md, UI không fake thành công.
