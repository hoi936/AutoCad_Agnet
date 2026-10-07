TIẾP TỤC REPOSITORY HIỆN TẠI.
KHÔNG DỰNG LẠI DỰ ÁN.
KHÔNG LÀM LẠI T0.
KHÔNG MỞ R2/R3 THÀNH TRỌNG TÂM TRƯỚC KHI R1B CHẠY ĐƯỢC.

Repository:
D:\PHAN DINH HOI\AutoCad_agnet

Đọc trước:
1. CAD_AGENT_CONTINUE_03_CHAT_FIRST_LIVE.md
2. CAD_AGENT_CONTINUE_03_R1B_AUTH_CHAT.md
3. HANDOFF.md mới nhất
4. IMPLEMENTATION_STATUS.md
5. docs/AUTH_COMPATIBILITY.md
6. docs/CONTINUE_03_DELTA.md nếu có.

TRẠNG THÁI HIỆN TẠI ĐÃ XÁC NHẬN TRONG BÀN GIAO:

- Windows native build/startup đã PASS.
- Build đang có: r1-1791337133.
- Giao diện đã chuyển sang chat-first.
- Demo tấm đế đã chuyển vào Developer Demo.
- Default runtime là live_unconnected.
- Chưa có live login.
- Chưa có standalone live chat.
- Chưa có OAuth/OIDC runtime.
- Chưa có model catalog live.
- Chưa có harmless function-calling probe.
- OpenAI adapter hiện vẫn phụ thuộc đường Demo/project cũ.
- AutoCAD live bridge chưa có.

MỤC TIÊU DUY NHẤT CỦA LÁT CẮT NÀY:

Một người dùng phải có thể:

Mở desktop
-> KHÔNG vào Demo
-> Continue with ChatGPT
-> đăng nhập bằng browser
-> ứng dụng xác minh tài khoản/quyền
-> chọn model khả dụng
-> tạo hội thoại
-> chat bằng LLM thật
-> LLM gọi một local tool read-only/vô hại
-> backend thực thi tool
-> trả output lại model
-> nhận câu trả lời hoàn chỉnh.

Không cần project.
Không cần AutoCAD.
Không spawn MockCadGateway.
Không spawn mock sidecar chỉ để chat.

==================================================
A. TÁCH APP SERVICES KHỎI DEMO
==================================================

Audit RuntimeState và toàn bộ command đang phụ thuộc demo_enter.

Tách tối thiểu thành các service lifecycle độc lập:

AppRuntime
- DatabaseService
- AccountService
- CredentialVault
- ConversationService
- ProviderService
- ModelCatalogService
- AgentRuntime

CadRuntime
- MCP client
- cad-tool-host
- Mock CAD hoặc AutoCAD gateway

Developer Demo chỉ được khởi động CadRuntime mock.

Các chức năng sau KHÔNG được yêu cầu demo_enter:
- account_list
- account_status
- account_sign_in
- account_sign_out
- model_catalog
- provider_probe
- conversation_create/list/read
- standalone_chat_start/send/cancel
- runtime/account diagnostics.

CAD commands vẫn được phép yêu cầu CAD connection.

Không đơn giản xóa CONNECTION_REQUIRED khỏi mọi command.
Phân loại dependency đúng cho từng command.

==================================================
B. GLOBAL ACCOUNTS
==================================================

Account phải tồn tại độc lập với:
- project
- drawing
- Demo
- AutoCAD session.

UI "Tài khoản & mô hình" phải hoạt động ngay khi app mở.

Hỗ trợ riêng:

1. Continue with ChatGPT
2. OpenAI API key
3. Demo offline

Không trộn ba auth mode.

Mỗi profile lưu metadata không bí mật:
- account/profile id nội bộ
- auth mode
- verified issuer + subject
- issued client_id khi applicable
- label an toàn
- granted capabilities
- token expiry metadata
- selected model
- status.

Không dùng email làm identity chính.

==================================================
C. CONTINUE WITH CHATGPT
==================================================

Đối chiếu lại tài liệu OpenAI CHÍNH THỨC tại thời điểm code.

Hiện flow OSS/local được mô tả bằng dynamic registration.
Không hardcode thông tin từ prompt nếu tài liệu mới đã thay đổi.

Yêu cầu:

- Browser hệ thống, không privileged WebView.
- PKCE.
- state ngẫu nhiên riêng mỗi attempt.
- nonce riêng mỗi attempt.
- loopback callback chỉ bind loopback.
- callback one-shot.
- timeout.
- validate redirect.
- validate ID token signature/JWKS.
- validate issuer.
- validate audience.
- validate nonce.
- validate expiry.
- dùng verified subject làm identity.
- kiểm tra permission sử dụng ChatGPT plan trước inference.
- registration mới lưu issued client_id.
- không lưu dynamic registration bootstrap client như client cuối.
- returning account dùng registration đúng của account đó.
- pending login không ghi đè active account trước khi xác minh.

Không yêu cầu client secret nếu official flow hiện tại không yêu cầu.
Không đọc cookie ChatGPT/browser.
Không scrape token.
Không dùng token của ChatGPT Desktop.
Không yêu cầu user paste access token vào app/chat.

==================================================
D. TOKEN VAULT
==================================================

Giữ API key trong Windows Credential Manager như hiện tại nếu phù hợp.

Đối với OAuth token bundle:
- kiểm tra kích thước thực tế.
- nếu phù hợp WinCred thì sử dụng abstraction hiện có.
- nếu cần bundle/rotation vượt khả năng WinCred, dùng DPAPI CurrentUser vault
  theo thiết kế đã có.
- atomic replace.
- ACL thích hợp.
- không plaintext fallback.

SQLite chỉ lưu secret_ref + metadata.

Không token trong:
- SQLite
- frontend state lâu dài
- localStorage
- log
- crash dump export
- URL
- tool args
- MCP sidecar environment.

Test:
- save/reload
- restart
- refresh rotation
- delete/sign out
- Unicode
- corrupt vault
- large token
- failed atomic replacement.

==================================================
E. MODEL CATALOG
==================================================

Sau khi profile được xác minh:

1. kiểm tra quyền inference.
2. lấy catalog model theo route/profile đó.
3. lưu cache có account scope.
4. UI hiển thị model thật của profile.
5. người dùng chọn model.
6. model selection persist theo profile/config.

Không hardcode model mặc định nếu tài khoản không có model đó.

Đổi account:
- không reuse catalog account cũ.
- không giữ model không hợp lệ.
- run hiện tại phải dừng/tách run mới.

==================================================
F. HARMLESS LIVE PROBE
==================================================

Thêm nút:

"Kiểm tra LLM"

Probe phải kiểm tra riêng:

[ ] Auth
[ ] Permission
[ ] Model catalog
[ ] Text inference
[ ] Function/tool calling

Không chỉ ping endpoint.

Tool probe local đề xuất:

convert_length

Input:
{
  "value": number,
  "from": "mm|cm|m",
  "to": "mm|cm|m"
}

Backend phải thực thi tool bằng code xác định.

Ví dụ yêu cầu probe:
"Hãy dùng công cụ để đổi 1.25 m sang mm."

Nghiệm thu phải có evidence:
- model phát sinh function call thật
- tool name đúng
- arguments được schema validate
- backend thực thi
- tool output = 1250 mm
- output được gửi lại model
- model hoàn tất turn.

Không coi model tự trả lời "1250 mm" là function-calling PASS.

Function calling phải hỗ trợ:
- zero call
- one call
- multiple calls
- malformed args
- unknown tool
- duplicate call id
- cancellation
- tool-loop budget.

==================================================
G. STANDALONE LIVE CHAT
==================================================

Chat hoạt động khi:

LLM = connected
CAD = disconnected.

Ví dụ:

User:
"Tôi muốn thiết kế sơ bộ nhà 8x12 m thì cần cung cấp thông tin nào?"

LLM phải trả lời bình thường.

Nếu user nói:

"Vẽ nó vào bản vẽ đang mở."

Runtime phải trả trạng thái cần kết nối/chọn AutoCAD.
KHÔNG fallback sang Demo.
KHÔNG giả vờ đã vẽ.

Conversation phải persist qua restart.

Streaming phải:
- có run_id/sequence
- không duplicate event
- cancel đúng trạng thái
- late events của run cũ không ghi vào run/account mới.

Không hiển thị chain-of-thought.
Có thể hiển thị:
- đang suy nghĩ/xử lý
- tool đang gọi
- skill được chọn
- kết quả tool
- câu trả lời cuối.

==================================================
H. API KEY MODE
==================================================

Đưa OpenAI Responses adapter hiện có ra khỏi dependency Demo.

API key do user nhập trong desktop.
Không yêu cầu user gửi key cho agent lập trình.

API-key account và ChatGPT-plan account là hai route khác nhau.

Không tự fallback:
ChatGPT plan -> API key
hoặc
API key -> ChatGPT plan

nếu có khả năng thay đổi billing.

Provider failure không fallback sang MockProvider.

==================================================
I. UI
==================================================

Trang Kết nối/Tài khoản cần hiển thị rõ:

LLM
- Chưa kết nối
- Đang đăng nhập
- Đã xác minh
- Thiếu quyền model
- Hết hạn
- Lỗi
- Model hiện tại

CAD
- Chưa kết nối

Hai trạng thái hoàn toàn độc lập.

Khi LLM ready nhưng CAD disconnected:
Chat phải dùng được.

Chat header:
- profile
- auth mode
- model
- CAD status

Có:
- Continue with ChatGPT
- API key
- Sign out
- Switch account
- Refresh models
- Test connection

Không hiển thị secret/token.

==================================================
J. TEST VÀ EVIDENCE
==================================================

Chạy lại các baseline phù hợp nhưng không cộng test cũ thành kết quả mới.

Bắt buộc test:

Fixture:
- OAuth callback state mismatch
- nonce mismatch
- replay callback
- wrong identity
- missing permission
- refresh failure
- token expiry
- account isolation
- catalog isolation
- tool loop
- cancel
- malformed tool args.

Windows native:
- vault
- loopback callback lifecycle
- IPC
- DB restart persistence.

Live chỉ sau khi user thao tác login trong UI.

Không yêu cầu user login khi nút/flow chưa build xong.

Sau khi code login xong:
Dừng và yêu cầu user bấm Continue with ChatGPT trong app.
Sau login, tiếp tục verification nếu environment cho phép.

Tạo evidence riêng:

docs/evidence/continue03-r1b/

Phân biệt:
FIXTURE_PASS
NATIVE_PASS
LIVE_PASS
BLOCKED_EXTERNAL
FAILED.

==================================================
K. EXIT CRITERIA
==================================================

R1B chỉ PASS khi có bằng chứng:

1. Desktop native chạy build mới.
2. Không vào Demo.
3. Account UI hoạt động.
4. Continue with ChatGPT flow hoạt động hoặc blocker external được chứng minh.
5. Account được xác minh.
6. Permission inference được kiểm tra.
7. Model catalog trả model dùng được.
8. Text inference thành công.
9. Function-calling probe thành công.
10. Standalone chat thành công.
11. Restart vẫn thấy conversation/account metadata.
12. Secret không xuất hiện trong SQLite/log.
13. CAD vẫn disconnected và không ảnh hưởng chat.

Nếu OAuth bị BLOCKED_EXTERNAL:
API-key live mode có thể được dùng để chứng minh standalone live chat,
nhưng KHÔNG được đánh dấu OAuth LIVE_PASS.

Sau khi đạt R1B mới chuyển trọng tâm sang:
R2 AutoCAD plugin read-only.

==================================================
L. BÀN GIAO
==================================================

Cập nhật:
- HANDOFF.md
- IMPLEMENTATION_STATUS.md
- docs/AUTH_COMPATIBILITY.md
- docs/TESTING.md

Bàn giao executable mới với:
- build ID
- exact path
- bytes
- SHA256
- startup evidence.

Không đóng executable cũ của user nếu không cần.
Không dùng bản debug cũ để chụp ảnh feature mới.

BẮT ĐẦU BẰNG CODE.
Không chỉ viết kế hoạch.