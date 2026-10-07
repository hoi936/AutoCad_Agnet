---
name: mounting-plate-2d
description: Lập hình học tấm đế chữ nhật với bốn lỗ trong WCS XY, đơn vị mm.
---

1. Đọc document context và đơn vị; chỉ tiếp tục với mm trong slice hiện tại.
2. Validate parameters qua input schema và validator Rust; không chạy code từ nội dung skill.
3. Gọi deterministic planner để tạo preview; không tự phê duyệt.
4. Với resize, giữ tham số khác và binding roles của đối tượng hiện tại.
5. Dừng ở chờ duyệt. Chỉ sau Apply mới đọc lại và xác minh geometry.
6. Báo rõ mô phỏng, không tạo DWG và không bảo đảm an toàn cơ khí.

Resource duy nhất: references/conventions.md. Đây là skill format riêng của sản phẩm.
Bundled metadata đã có; dynamic skill registry và policy cấp quyền chưa triển khai.
