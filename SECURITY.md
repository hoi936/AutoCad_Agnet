# R1B — credential boundary

OAuth callback chỉ 127.0.0.1, 180 giây, 16 KiB cap, một lần; browser ngoài privileged
WebView. JWT verify trước lưu identity; granted inference scope kiểm riêng. Account
registration không dùng email làm khóa. Credential không nằm trong SQLite hoặc IPC return.
DPAPI CurrentUser + protected owner/System DACL + atomic file replace; WinCred API key.
FFI Windows được cô lập trong crates/native-vault; mã caller vẫn forbid unsafe. Không
hứa chống malware đã chiếm user/backend. Account DB lock + async gate chống refresh race.
RSA private key dưới fixtures là test-only, không sử dụng làm live trust anchor.
Dependency vulnerability/license audit đầy đủ chưa chạy; không tuyên bố security-certified.

# Bảo mật và phạm vi

R1B: AppRuntime mở DB/account/chat ngay khi startup, không mở mock sidecar. Commands
CAD vẫn yêu cầu lựa chọn Demo/connection; account/chat độc lập với gate CAD. Migration 0004 thu hồi preview cũ bằng metadata/status,
không sửa immutable payload; approve cũ trả CONFLICT. Unknown operations giữ nguyên
để recovery. Global accounts và OAuth/DPAPI vault đã có mã/test; live catalog/probe/chat đã PASS với profile hiện tại. AutoCAD pairing chưa có.


Tài sản gồm project, hội thoại, hình học mock, lịch sử duyệt, credential và quyền ghi.
React/LLM/tool results là dữ liệu không tin cậy; App Core là backend tin cậy.
Không có bridge AutoCAD, DWG thật hoặc updater trong bản này.
Windows vault roundtrip/cleanup đã pass ngày 07/10/2026 với credential fixture riêng.
Bản native hiện tại là debug chưa ký; chưa có installer. Sau fault-injection tests,
sidecar thường được build lại. Không hạ Defender/SmartScreen, execution policy,
SECURELOAD hoặc TRUSTEDPATHS trong quá trình thiết lập Windows.

- Tauri capability chỉ main local, danh sách command cụ thể; CSP local IPC.
  Không cấp shell/filesystem/SQL/MCP tùy ý hoặc gọi provider trực tiếp từ frontend.
- Schema, finite/bounds và geometry được validate tại Rust. Tool allowlist/mode guards
  ở backend; model không thể tự duyệt, đổi policy, đọc key hoặc chạy shell.
- Approval gắn immutable hash, document/session/revision/before-state và TTL.
  Permit HMAC-SHA256 dùng thư viện chuẩn, key bootstrap qua private stdio;
  sidecar kiểm tra lại trước ghi và giữ receipt chống duplicate.
- Binary sidecar cố định, environment bị giới hạn; không truyền credential model.
  Giới hạn frame/arguments/time/tool calls/repair/output, shutdown và child exit detection.
- Windows keyring native; session storage opt-in, không tự fallback plaintext.
  UI gửi key một lần rồi xóa form. Chỉ trả configured/status, không có get-key command.
  SecretValue zeroize/redacted Debug; lỗi provider là thông báo an toàn.
- OpenAI endpoint HTTPS cố định, TLS verification, không redirect. Cloud cần consent.
  store:false không phải cam kết zero retention phía provider.
- Không log full request/response hoặc reasoning. Deltas tạm thời, milestone/final persist.
  Fixture tests kiểm tra key không xuất hiện trong DB/event/config.
- Snapshot và SQLite không atomic cùng nhau: lỗi sau gửi cần đối chiếu; không retry mutation.

Giới hạn: không chống malware chiếm cùng tài khoản/backend. Chưa có sandbox OS riêng
cho sidecar, named-pipe ACL/AutoCAD pairing, backup/retention/mã hóa hội thoại hoặc fuzz suite.
Windows vault/native ACL/CSP runtime và live provider chưa được nghiệm thu trên máy thật.
Các suite hiện tại không chứng nhận mọi prompt-injection hoặc mọi dạng secret leakage.
Dependency/license audit đầy đủ vẫn là gate trước phát hành. Installer development chưa ký.
Không đưa key, DWG hoặc database thật vào issue công khai.

Continuation 02 T0: Windows setup mặc định plan; -Install chỉ cho một component,
version tường minh (pnpm theo packageManager), review WinGet manifest và ShouldProcess.
Không nâng/hạ AutoCAD, sửa SECURELOAD/TRUSTEDPATHS, execution policy hay chạy app elevated.
Doctor chỉ đọc registry và metadata managed assemblies; không tải plugin hoặc ghi DWG.
