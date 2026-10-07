# Continuation 03 — delta 07/10/2026

R0: đọc prompt 03, source và evidence 02. Không dựng lại scaffold hoặc cài lại tools.
Repo không có Git hợp lệ; không có commit/PR. Debug copy 02 không tự cập nhật theo source.

| Capability | Code/UI hiện có | Tái sử dụng | Bổ sung | Test | Mốc |
|---|---|---|---|---|---|
| Workspace | App.tsx mặc định form/mock | ChatPanel, Preview, history | Shell chat-first; demo opt-in; lazy MCP; build ID | Browser/native build | R1 |
| Stale preview | run_change_sets, hash/revision/TTL | ApprovalService | Invalidation khi run/config mới; supersede cùng document; clear UI | Core + React regression | R1 |
| Accounts/OAuth | API adapter + vault theo project | ProviderService/OpenAiProvider | Global accounts, OAuth/OIDC/DPAPI/catalog/probe chưa có | Live NOT RUN | R1 còn thiếu |
| Skills | mounting plate bundled | GeometryPlanner, policy | Registry + primitives/selection rồi floorplan | Chưa có | R3/R4 |
| AutoCAD | Detection 2026.1.2 | MCP process + permit | C# pipe/plugin/pairing/read-only chưa có | Live NOT RUN | R2 |
| Packaging | debug Tauri/sidecar | Windows scripts | Build ID/artifact mới; NSIS/UI E2E sau | Build/startup tách E2E | R5 |

Lát cắt này chưa hoàn thành R1. Không dùng test mock để báo live inference/AutoCAD PASS.
