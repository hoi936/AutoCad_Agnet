# IPC v1

Wire DTO sinh từ `crates/contracts/src/lib.rs`; `pnpm contracts:generate` cập nhật
TypeScript/schema/fixture. Tham số lệnh Tauri dùng camelCase; field DTO dùng snake_case.

| Command | Input | Result |
|---|---|---|
| health | — | Rust/SQLite runtime/mode string |
| projects_list | — | Project[] |
| projects_create | name | Project |
| cad_get_context | projectId | DocumentContext |
| design_propose_plate | projectId, parameters, mode | ChangeSet |
| changes_list | projectId | 100 ChangeSet gần nhất |
| changes_approve | changeId, payloadHash | ExecutionReceipt |
| changes_reject | changeId | null |
| changes_revert | changeId | inverse ChangeSet chờ duyệt |

Approve không nhận operations từ frontend. Plan không thể duyệt. Ask không thể đề xuất.
Hash, session, TTL 600 giây, document, before geometry và base revision kiểm tra tại backend.
Idempotent retry trả receipt lịch sử cùng operation, không ghi lại.
Context polling 5 giây, sau mutation invalidate query. RunEvent polling theo snapshot 500 ms khi chạy, dedupe bằng run_id/sequence; deltas không ghi từng row.

AppError: code, message, retryable, correlation_id, safe_details. Storage error có UUID;
domain errors hiện correlation_id rỗng (cần bổ sung run/request context).
Không đưa lỗi SQL nội bộ hoặc full path lên UI.
Các command khác trong master prompt **chưa được triển khai**, không có stub báo success.

## Commands continuation 01

| Command | Input | Result |
|---|---|---|
| conversations_create | projectId, title | Conversation |
| conversations_list | projectId | Conversation[] |
| conversation_snapshot | conversationId | ConversationSnapshot |
| agent_start | conversationId, content, mode | AgentRun |
| agent_snapshot | runId | RunSnapshot |
| agent_stop | runId | null |
| provider_get_config | projectId | ProviderConfig, chỉ configured |
| provider_save_config | projectId, provider, model, cloudConsent, storage | ProviderConfig |
| provider_set_secret | projectId, secret | null |
| provider_delete_secret | projectId | null |
| changes_recover | changeId | ExecutionReceipt sau đối chiếu |

Không có command trả key hoặc nhận tool name tùy ý. App Core chọn provider từ cấu hình;
UI không gửi implementation/provider object. outcome_unknown xuất phát từ journal và
không được reject như proposal chưa gửi. not_applied phải lập phương án mới.
