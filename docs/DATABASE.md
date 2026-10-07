## R1B — migration 0005

llm_accounts là global account metadata/secret_ref/catalog/model, khác provider_configs
legacy gắn project. standalone_conversations/runs/messages không có project/document FK.
app_settings lưu host identity UUID ổn định. Không raw token/key. Token bundle ở DPAPI;
API key ở WinCred. DELETE/FULL giữ nguyên, không xóa các bảng/migration cũ.
R1B AppRuntime mở DB khi startup; Demo mở CAD riêng. Process accounts.lock ngăn hai
runtime refresh cùng bundle; SQL writer/gate giới hạn một lượt chat active. Restart
đánh interrupted cho run còn running, không replay. Fixtures kiểm message/model/catalog
isolation qua close/reopen. Chưa có backup/retention bổ sung.

## Continuation 03 — migration 0004

`0004_proposal_invalidation.sql` thêm proposal_invalidations với reason. Giữ các bảng,
FK và payload lịch sử. Preview pending trước nâng cấp bị thu hồi; operation chưa rõ
kết quả được giữ nguyên. Trigger cho phép tối đa một proposal pending/planned mới
trên cùng document bằng supersede bản cũ (chặt hơn phạm vi conversation).
Run mới/provider change ghi invalidation và rejected trong transaction writer; API
chiếu status superseded. Run IDs vẫn liên kết qua run_change_sets. Full design_proposals,
source_message provenance/global accounts chưa thêm. DELETE/FULL giữ nguyên.
Fixture upgrade 0001 và 0003 đã pass; không có backup tự động mới.

# Database của slice mock

Migration `0001_mock_foundation.sql`: projects → documents → change_sets → approvals/receipts,
audit_events. Ràng buộc FK RESTRICT, CHECK JSON, enum trạng thái, unique approval và receipt/change.
History giới hạn 100 phương án gần nhất mỗi project. Không seed tự động, không xóa dự án tự động.

Đây là subset schema của master prompt. Migration 0002 bổ sung conversations/messages/agent_runs/run_events/tool_calls/provider_configs/run_change_sets. Migration 0003 bổ sung gateway_state/gateway_operations. Còn thiếu
semantic_objects/entity_bindings riêng, skills/policy/artifacts. Binding mock dùng role ID
trong Geometry. Timestamp hiện dùng Unix seconds UTC, khác quy ước chuỗi ISO dự kiến.

SQLx 0.9.0 locked vẫn sử dụng libsqlite3-sys 0.30.1, runtime đã đọc bằng
`SELECT sqlite_version()` là 3.46.0. Vì chưa có bản vá WAL-reset, core hiện dùng
DELETE journal + FULL synchronous, busy_timeout 5 giây, pool 1 connection,
foreign_keys bật và xác minh. **Yêu cầu WAL với engine đã vá chưa hoàn thành.**
Không đổi PRAGMA WAL cho database này trước khi khóa engine đã vá và kiểm thử recovery.

Harness in-process cũ ghi geometry/receipt cùng transaction. Đường desktop MCP dùng
journal gửi trước mutation và receipt/readback sau; lỗi core không rollback snapshot sidecar.
Xem RECOVERY.md. Không giữ transaction
trong lúc người dùng xem preview. New process tạo session mới để chặn approval phiên cũ.
SQLite không mã hóa metadata/hình học. Chưa có retention hoặc consistent backup API.
Muốn sao lưu thủ công: đóng tất cả instance rồi dùng SQLite backup; không copy file đang chạy.
Không tự xóa database khi migration hoặc I/O lỗi. Migration đã phát hành phải thêm file mới.

Nguồn kiểm tra 05/10/2026:
- https://docs.rs/sqlx/0.9.0/sqlx/
- https://sqlite.org/foreignkeys.html
- https://sqlite.org/wal.html
- https://sqlite.org/backup.html
