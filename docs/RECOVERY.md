# Khôi phục MCP mock

Nguồn thật của hình học và operation receipt là mock/state.json do sidecar quản lý.
app.sqlite giữ proposal, approval, run và bản cache hình học đã xác minh. Hai file không
commit đồng thời; không lấy transaction SQLite làm bằng chứng CAD đã rollback.

Trước mutation, core ghi journal sending. Sidecar ghi hình học và receipt cùng snapshot
bằng atomic replace. Core đọc receipt, context và validate_entities rồi mới đánh verified.
Timeout, disconnect hoặc lỗi lưu receipt ở core thành outcome_unknown; lần approve tiếp
không gửi lại. Mutation mới của dự án bị khóa đến khi thao tác cũ được đối chiếu.

Sau khi mở lại app, chọn “Đối chiếu kết quả” trong lịch sử:
- Receipt khớp operation/hash/revision và after-state: đánh verified, không ghi CAD lần nữa.
- Không có receipt, revision và before-state vẫn khớp: not_applied; lập proposal mới.
- Bằng chứng không khớp: recovery_required, giữ dữ liệu để kiểm tra thủ công.

Nếu sidecar đã chết, đóng/mở lại app để tạo phiên kết nối mới trước khi đối chiếu.
Phương án chưa duyệt của phiên cũ cần lập lại. Không tự retry mutation hoặc replay run.
Snapshot đã được khởi tạo mà bị mất: startup báo RECOVERY_REQUIRED, không seed lại từ cache.
Lần đầu nâng cấp DB cũ mới được import hình học sang snapshot; giữ lịch sử cũ.

Kiểm thử bao gồm process thoát ngay sau commit, receipt còn sau restart, và core lưu
receipt lỗi sau CAD đã commit. Đây là mô phỏng process failure; chưa thử mất điện thật.
Không xóa DB/snapshot để chữa lỗi. Sao lưu nhất quán khi mọi instance đã đóng.
Bridge DWG sau này vẫn cần marker/undo/reopen/unsaved-document recovery riêng.
