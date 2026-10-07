# Browser authorization failure

## OAuth live — user report 07/10/2026

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

