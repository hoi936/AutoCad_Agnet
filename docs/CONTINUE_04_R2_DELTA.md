# R2 audit — 07/10/2026

| Capability | Source hiện có | Test hiện có | Thiếu gì | Quyết định tái sử dụng |
|---|---|---|---|---|
| ChatGPT accounts/stream/tool loop | `crates/app-core/src/standalone.rs`, `crates/model-provider` | R1B LIVE_PASS; 53 Rust/vault, 19 UI | CAD read tool route, pin session/document | Giữ nguyên auth/catalog/chat, thêm CAD runtime độc lập |
| MCP client/sidecar | `crates/cad-tool-host` | Process integration với mock | Read-only MCP server + Windows pipe backend | Giữ rmcp/stdio, không đi vào mock `CadServer` khi live |
| CAD contracts | `crates/contracts`; mock `cad.get_status/context/selection/query/validate` | Demo fixture | DTO/session, layer/doc listing, bounded query | Tái sử dụng tên công cụ đọc, schema mới riêng R2 |
| Desktop | Tauri account commands; Kết nối AutoCAD còn disabled | R1B native UI | detect/install/pair/disconnect/status/diagnostics | Mở command/capability riêng; không mở Demo |
| AutoCAD bridge | Chưa có C# source | Doctor thấy exe/DLL, không live | Protocol, transport, plugin, bundle, main-thread read | Tạo `bridges/autocad` từ đầu; tham chiếu DLL tại installation, CopyLocal=false |
| AutoCAD target | `C:\Program Files\Autodesk\AutoCAD 2026`, 2026.1.2 | Read-only PE/assembly metadata | plugin load/handshake/readback | `net10.0-windows`, x64 theo `acdbmgd.runtimeconfig.json` và Autodesk compatibility guide |

AutoCAD PID 2076 đang chạy, nhưng chưa ghép cặp. `docs/BRIDGE_PROTOCOL.md` chưa tồn tại.
R2 không dùng `Studio::open_mcp` vì constructor đó bootstrap mock và có mutation tools.
R1B `AppRuntime` không có CAD gateway, phải thêm kết nối chỉ đọc và chỉ bật model tools khi
handshake + document readback đạt. Không coi dotnet build, pipe tồn tại hoặc process đang chạy
là CAD connected.
