# Bundled skill

`skills/mounting-plate-2d` có manifest, workflow, schema, conventions và fixture cases.
**Chưa được dynamic registry nạp hoặc kiểm tra quyền trong ứng dụng**; UI hiện gọi planner cứng.
Không có loader filesystem, ZIP, downloader hoặc executable script từ skill.

Contract schema lấy từ Rust generator. content_hash tính SHA-256 của từng file trong
resources theo đúng thứ tự: UTF-8 relative filename + NUL + bytes + NUL; manifest không
nằm trong tập hash để tránh tự tham chiếu. Hash phát hiện thay đổi, không chứng nhận tác giả.
Khi có registry, phải kiểm tra path/symlink/size/types, pin version/hash theo project,
và yêu cầu duyệt lại khi hash hoặc quyền thay đổi. Không tự cấp mutation/approval từ manifest.
