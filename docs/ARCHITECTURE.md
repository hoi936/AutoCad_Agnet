# Kiến trúc hiện tại

React → typed Tauri commands → AgentService / ProviderService / Studio → MCP client
rmcp → child process cad-tool-host qua stdio → MockCadGateway có snapshot riêng.
Desktop luôn dùng Studio::open_mcp; Studio::open chỉ là harness in-process tường minh.
Không fallback sang cache khi MCP hỏng. Không có shell, HTTP server hoặc cổng localhost
cho IPC ứng dụng; HTTP fixture chỉ tồn tại trong kiểm thử provider.

AgentService có giới hạn thời gian/tool call/repair, cancel và một run active mỗi project.
Tool allowlist chỉ đọc và đề xuất. Model không nhận API approve/apply/revert.
ApprovalService kiểm tra hash, TTL, Execute, session, revision và before-state;
McpGateway ký permit HMAC theo operation/document/session/hash/revision/scope/expiry.
Key permit bootstrap qua stdin riêng, không qua CLI/environment/model/frontend.
Sidecar kiểm tra lại permit và hình học. Tool discovery MCP không đồng nghĩa model có quyền gọi.

App Core là nơi duy nhất ghi SQLite. Sidecar không mở database ứng dụng.
Snapshot JSON mock ghi geometry và receipt cùng atomic replace, giữ exclusive file lock.
Journal SQLite ghi sending trước dispatch, verified sau receipt và readback; lỗi giữa hai
nguồn dữ liệu thành outcome_unknown. Xem RECOVERY.md.

ModelProvider có MockProvider xác định và OpenAiProvider dùng Responses streaming,
continuation/tool call IDs, endpoint HTTPS cố định, không redirect và không retry mutation.
SecretStore có WindowsCredentialStore và session opt-in. Chỉ config/status trở lại UI.
Deltas nằm trong bộ nhớ để UI polling; milestone/final nằm trong DB, không ghi mỗi token.
Không công bố hoặc lưu reasoning/opaque continuation của provider.

cad-contracts là nguồn DTO/schema/TS. Geometry planner/validator dùng chung để kiểm tra
8 entity, role/layer/mapping và kích thước. Validator này không phải CAD engine độc lập.
Clock và IdGenerator thay được trong test. C# bridge/named pipe/DWG thuộc backlog tiếp theo.
