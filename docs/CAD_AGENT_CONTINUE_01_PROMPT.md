# PROMPT TIẾP NỐI 01 — HOÀN THIỆN AGENT BASE

> Dùng cùng CAD_AGENT_BASE_MASTER_PROMPT.md trong repository hiện có.
> Đây là yêu cầu triển khai tiếp, không phải yêu cầu tạo lại dự án.
> Giao diện, tài liệu và báo cáo: tiếng Việt. Tên mã nguồn: tiếng Anh.
> Ngày soạn: 05/10/2026. Kiểm tra lại API và phiên bản từ nguồn chính thức khi thực hiện.

## 1. NHIỆM VỤ VÀ PHẠM VI

Bạn là kỹ sư tiếp quản CAD Agent Studio. Hãy sửa và bổ sung mã trong repository hiện tại để hoàn thiện đường chạy agent trên CAD mô phỏng.

Giữ kiến trúc đã chốt:
- Tauri 2 + React + TypeScript cho desktop/frontend.
- Rust cho backend cục bộ, policy, agent runtime và MCP tool host.
- SQLite cho dữ liệu ứng dụng; migration thật, không phá dữ liệu cũ.
- Windows Credential Manager cho credential trên Windows.
- C# là công nghệ bridge AutoCAD về sau, không đổi sang điều khiển chuột.

Mục tiêu của lượt này:
Ứng dụng desktop -> chat -> Rust agent runtime -> công cụ đọc/đề xuất -> preview -> duyệt -> MCP sidecar -> CAD mô phỏng -> đọc lại/xác minh -> lịch sử.

Phải hỗ trợ MockProvider và một adapter OpenAI Responses API có mã triển khai thật. Không cần API key để chạy test mặc định và demo.

Đợt này chưa triển khai bridge ghi DWG thật, marketplace skill, nhiều model provider, OAuth, đăng nhập tài khoản sản phẩm, cloud sync hoặc server web. Giữ CadGateway và contract sẵn sàng cho bridge về sau. Không xóa hạng mục bridge khỏi backlog hoặc tuyên bố Foundation toàn bộ đã hoàn thành.

File master vẫn là đặc tả tổng thể. File này thu hẹp phạm vi và ưu tiên công việc tiếp theo; không hạ các yêu cầu bảo mật, approval hoặc recovery trong master.

## 2. BÁO CÁO CŨ CHỈ LÀ ĐIỂM XUẤT PHÁT

Báo cáo bàn giao cho biết:
- Có giao diện React/Tauri tiếng Việt, Rust và SQLite.
- Có luồng mô phỏng preview -> duyệt -> áp dụng -> xác minh -> resize -> hoàn tác.
- 9 test Rust và 3 test giao diện được báo là đạt; typecheck/lint/build frontend được báo là đạt.
- Chưa có chat/LLM, MCP sidecar, vault và bridge AutoCAD.
- Chưa có .exe; native build trong WSL bị chặn bởi dependency hệ thống.

Không coi báo cáo trên là bằng chứng bạn đã kiểm thử. Phải kiểm tra mã và chạy lại các kiểm tra khả thi. Nếu trạng thái repository khác báo cáo, lấy mã và log hiện tại làm căn cứ.

Không tuyên bố đã đọc README hoặc IMPLEMENTATION_STATUS khi chưa mở file thực tế.

## 3. S0 — KIỂM TRA REPOSITORY VÀ ĐƯỜNG BUILD

Trước khi sửa:
1. Xác định đúng repository, branch, working tree và thay đổi chưa commit.
2. Đọc master, README.md, IMPLEMENTATION_STATUS.md, HANDOFF.md nếu có.
3. Đọc package.json ở root và package desktop, pnpm-workspace.yaml, lockfile, Cargo.toml, Cargo.lock, tauri.conf.json, capabilities, migrations và test hiện có.
4. Xác định đường đi thật từ nút UI tới Rust và database. Phân biệt frontend stub, Rust mock và CAD gateway có state.
5. Chạy baseline tests theo scripts thực tế, lưu lệnh, thư mục chạy, phiên bản toolchain, exit code và lỗi đã làm sạch.

Tạo docs/BASELINE_AUDIT.md với mỗi capability gồm:
- File/module triển khai.
- Đường chạy thực tế.
- Test đang bao phủ.
- Trạng thái: implemented / partial / missing / not verified.
- Công việc nhỏ nhất cần làm tiếp.

Không xóa test để số test xanh. Không xóa lockfile, database, thư mục người dùng hoặc code có sẵn để dựng lại scaffold.
Nếu không có repository truy cập được, báo thiếu repository; không tạo một dự án khác rồi gọi là bản tiếp nối.

### Đường Windows và WSL

Mục tiêu phát hành là Windows desktop. Phân biệt host đang chạy, Rust target và loại artifact.

Nếu có Windows toolchain:
- Kiểm tra Rust MSVC, Microsoft C++ Build Tools và Windows SDK, WebView2 Runtime, Node/pnpm đúng cấu hình dự án.
- Dùng PowerShell/Developer PowerShell phù hợp để build và chạy desktop.
- Không chạy toàn bộ ứng dụng quyền administrator làm cách chữa lỗi mặc định.

Nếu chỉ có WSL/Linux:
- Ghi lỗi native build cụ thể và dependency thiếu; không đoán tên thư viện khi chưa có log.
- Tiếp tục unit/integration tests cho core, database, provider và MCP không phụ thuộc GUI.
- Tách app-core/contracts/tool-host khỏi Tauri/WebKit để headless tests không kéo GUI không cần thiết.
- Tạo scripts và CI Windows để kiểm chứng sau; cấu hình CI tồn tại chưa có nghĩa job đã chạy.
- Không coi cài thêm GTK/WebKit là đã chứng minh tạo/chạy được ứng dụng Windows.
- Không chuyển công việc chính thành bài toán cross-compile Windows từ Linux. Ưu tiên Windows local hoặc Windows runner khi có quyền sử dụng.

Tạo hoặc sửa scripts thực tế, phù hợp monorepo:
- pnpm dev: khởi chạy desktop Tauri cùng backend, không chỉ Vite.
- pnpm dev:web: frontend riêng, ghi rõ giới hạn.
- pnpm lint / pnpm typecheck / pnpm test.
- pnpm test:core / pnpm test:mcp.
- pnpm package:windows.
- scripts/doctor.ps1 hoặc công cụ chẩn đoán tương đương.

Không hardcode tên package chưa tồn tại. Không tạo vòng lặp giữa pnpm dev và beforeDevCommand. Không spawn hai sidecar từ hai đường startup.

Doctor chỉ đọc và báo thiếu prerequisite; không tự cài phần mềm, sửa policy hệ thống hoặc yêu cầu tắt bảo mật.

## 4. S1 — DỮ LIỆU, CONTRACT VÀ SECRET STORE

Tái sử dụng các service/repository/contract hiện có. Ưu tiên bổ sung adapter hơn thay toàn bộ implementation.

Bổ sung tối thiểu dữ liệu còn thiếu cho:
- conversations, messages.
- agent_runs, run_events, tool_calls.
- provider_configs với secret_ref, không chứa secret.
- Liên kết với projects, documents, change_sets, approvals và execution_receipts hiện có.

Không tạo bảng trùng chỉ vì tên hiện tại khác master. Lập bảng ánh xạ và migration nếu cần.
Không tự thay ORM chỉ để làm code giống một scaffold khác.

Migration phải:
- Có version và kiểm thử trên database mới lẫn database phiên bản hiện tại.
- Giữ project, drawing metadata và lịch sử cũ.
- Có FK/index/unique constraint phù hợp.
- Không seed lại dữ liệu demo vào project thực.
- Không cần database riêng của máy tác giả để build fresh clone.

App Core là nơi duy nhất ghi SQLite ứng dụng. Sidecar không mở cùng SQLite để cập nhật tùy ý.
CAD mô phỏng có thể giữ snapshot riêng; phải có quy tắc phục hồi và nguồn sự thật rõ ràng, không giả định snapshot và SQLite commit đồng thời.

Dùng hoặc bổ sung các interface:
ModelProvider, SecretStore, CadGateway, ApprovalService, Clock, IdGenerator.
Clock và ID phải thay được trong test.

### Secret store

- Triển khai WindowsCredentialStore trong Rust qua wrapper được kiểm tra.
- Frontend gửi API key một lần qua command chuyên biệt, sau đó xóa khỏi state/form.
- Command đọc chỉ trả configured/status; không có command trả plaintext key.
- SQLite chỉ lưu cấu hình provider và secret_ref.
- Không ghi secret vào localStorage, sessionStorage, log, URL, .env của ứng dụng đã cài, crash report, prompt, tool argument hay environment của sidecar.
- Mock/test trên hệ điều hành khác dùng MemorySecretStore chỉ dành cho test. Không âm thầm fallback lưu plaintext.
- Có thể có chế độ chỉ giữ trong phiên, nhưng phải do người dùng chọn rõ và không tuyên bố đã lưu bền vững.
- Xử lý lỗi vault; không báo lưu thành công khi write thất bại.
- Có chức năng thay và xóa credential.
- Dùng credential namespace riêng và dọn credential test; không đọc/xóa credential khác của người dùng.

API key do người dùng nhập trong Settings khi chạy. Không yêu cầu họ gửi key trong chat với AI lập trình.

### Quyền frontend

Rà soát Tauri capabilities cùng quyền của custom commands; không giả định plugin permissions tự bảo vệ mọi invoke_handler.
Chỉ các window/webview được phép mới gọi command nhạy cảm.
Frontend không được gọi shell, MCP tùy ý, SQL tùy ý hoặc HTTP provider trực tiếp.
Không mở quyền rộng hoặc remote content chỉ để khắc phục lỗi cấu hình.

## 5. S2 — MCP SIDECAR THẬT, CAD VẪN MÔ PHỎNG

Tạo cad-tool-host bằng Rust và SDK MCP Rust chính thức, khóa phiên bản đã build/test. Kiểm tra API theo đúng phiên bản, không sao chép ví dụ cũ rồi bịa phương thức.

Đường production/demo chính sau mốc này:
Rust App Core -> MCP client -> private stdio -> cad-tool-host -> MockCadGateway.

In-process gateway chỉ còn dùng cho unit tests hoặc đường phát triển được ghi rõ, không dùng nó để giả vờ MCP đã hoạt động.

Yêu cầu:
- Có initialization/version negotiation, tool discovery và calls thật.
- Chỉ spawn binary của ứng dụng từ đường dẫn xác định; không để model chọn executable hoặc argument tùy ý.
- stdout chỉ chứa protocol; log đã làm sạch ra stderr.
- Không truyền API key sang process con. Giới hạn environment kế thừa.
- Có giới hạn kích thước, timeout, child exit detection và shutdown.
- App đóng thì dọn process con; restart có kiểm soát, không tự replay mutation.
- Đóng gói binary sidecar đúng target; kiểm tra chạy ở thư mục cài đặt, không chỉ thư mục source.
- Tool không hỗ trợ phải trả lỗi rõ, không trả success chung.

Các tên sau là contract ứng dụng, không phải tool có sẵn từ Autodesk:
- cad.get_status
- cad.get_document_context
- cad.get_selection
- cad.query_entities
- cad.validate_entities
- cad.get_operation_status

Tool đề xuất nằm ở App Core:
- design.propose_plate
- design.propose_plate_resize

Tool ghi nội bộ:
- cad.apply_change_set
- cad.revert_change_set

Tool ghi và approval không xuất hiện trong danh sách đưa cho LLM. Tuy nhiên, không chỉ dựa vào việc ẩn tên: sidecar vẫn phải từ chối mutation không có permit hợp lệ.

Duy trì các kiểm tra từ master:
- Approval gắn immutable payload hash, document, session, base revision, phạm vi và thời hạn.
- Permit do backend tạo, không do UI/LLM tạo; dùng cơ chế chống sửa đổi với thư viện chuẩn, không tự sáng chế crypto.
- Document/session/revision được kiểm tra lại sát thời điểm mutation.
- Double click và gọi lại cùng operation_id không tạo thêm hình.
- Không bootstrap được phiên tin cậy thì không cho ghi.
- Secret session không nằm trong argument dòng lệnh, log hoặc prompt.
- Không quảng cáo cơ chế local này chống được malware đã chiếm cùng tài khoản/backend.

MockCadGateway phải có state, entity IDs, revision, receipt và fault injection.
Chuyển sang sidecar không được làm mất dữ liệu mô phỏng hiện có hoặc tạo hai nguồn state trái nhau.
Viết integration test spawn process thật; mock một trait trong cùng process không phải test MCP.

## 6. S3 — CHAT VÀ AGENT RUNTIME

### Frontend

Tái sử dụng giao diện hiện tại, không thiết kế lại toàn bộ.

Bổ sung:
- Tạo/mở hội thoại thuộc project; lịch sử sau khi khởi động lại.
- Ô nhập nhiều dòng, gửi, dừng, trạng thái đang xử lý và lỗi có thể hiểu.
- Streaming text và tool activity đã làm sạch.
- Khu preview/change-set hiện có, không làm một đường apply mới bỏ qua policy.
- Ask / Plan / Execute with approval.
- Provider/model đang chọn.
- Hai trạng thái riêng: LLM mock/live và CAD mock/AutoCAD.
- AutoCAD chưa có bridge phải hiện chưa khả dụng; không hiện Connected giả.
- Khi provider lỗi, không chuyển âm thầm sang mock rồi báo câu trả lời thật.

MockProvider chỉ hỗ trợ các fixture đã mô tả; UI ghi rõ không phải LLM tổng quát.

Event có run_id, sequence và type. UI xử lý trùng event và lấy lại snapshot khi reconnect.
Không persist mỗi token thành một row; lưu message theo checkpoint có giới hạn hoặc khi hoàn tất.
Không yêu cầu/hiển thị private chain-of-thought; chỉ hiển thị kế hoạch ngắn, tool activity và kết quả.

### Vòng agent

1. Nhận message và kiểm tra project policy/mode.
2. Lấy ngữ cảnh CAD tối thiểu qua MCP.
3. Chọn tool đọc/đề xuất nằm trong allowlist.
4. Gọi provider.
5. Ghép đủ arguments, validate schema và nghiệp vụ, rồi mới dispatch.
6. Trả kết quả tool đúng call ID cho provider.
7. Khi có change-set, hiển thị preview và dừng chờ duyệt; không ghi CAD.
8. Apply qua service hiện có sau thao tác người dùng.
9. Readback/verify qua MCP, lưu receipt và cập nhật hội thoại theo kết quả thật.

Có giới hạn số tool calls, số lần sửa arguments, tổng thời gian xử lý, kích thước dữ liệu và token.
Chỉ một mutation đang chạy trên mỗi document.
Ask không được sửa; Plan không được apply.
Không dựa vào câu "hãy xin phép" trong system prompt để thực thi phân quyền.

Stop trước mutation hủy công việc chưa bắt đầu.
Stop khi mutation đã gửi phải phản ánh trạng thái thực tế, không tuyên bố rollback.
Mất kết nối sau gửi mutation -> outcome_unknown -> đối chiếu operation status/readback.
Không retry mutation tự động khi chưa biết kết quả.
Revert tạo inverse change-set được duyệt, kiểm tra after-state; không undo mù.

### Provider mock và provider thật

MockProvider:
- Streaming xác định, câu tạo tấm đế, câu resize.
- Fixture JSON sai, tool lạ, timeout, rate limit, vòng lặp và cancellation.
- Không Internet, không key, không kiểm tra bằng cách trả success cho mọi prompt.

OpenAI adapter:
- Rust gọi Responses API theo tài liệu hiện hành.
- Model ID cấu hình được; không bịa model và không hardcode model chưa kiểm chứng.
- Streaming text và function calls; xử lý nhiều output item/call ID đúng contract.
- Tool arguments chỉ được thực thi khi đã hoàn chỉnh và hợp lệ.
- Gửi lại tool outputs; giữ các continuation item mà API/model yêu cầu, không làm rơi chúng hoặc phơi bày suy nghĩ nội bộ.
- Có lớp chuyển đổi tên tool/schema phù hợp provider; mapping xác định, không đụng tên, không forward toàn bộ MCP tool list vào model.
- App Core tự gọi MCP local; không yêu cầu OpenAI cloud truy cập stdio trên máy người dùng.
- Xử lý 401/403, 429, 5xx, timeout, cancellation, stream ngắt và payload sai.
- TLS verification bật; endpoint allowlist; không follow redirect mang credential sang origin khác.
- Cloud requests cần consent theo project/provider, chỉ gửi ngữ cảnh cần thiết.
- Dùng chính sách lưu phía provider phù hợp đặc tả và khả năng API; không hứa "zero retention" chỉ vì đặt một request flag.
- Log/metrics không có key hoặc full request chứa credential.

HTTP fixture tests chạy mặc định không tốn phí.
Live smoke test chỉ chạy khi người dùng chủ động bật và cấu hình credential.
Không có key vẫn phải xây adapter và test HTTP fixtures; ghi LIVE NOT RUN thay vì fake success.
Một response 200 hoặc chat text hoạt động không chứng minh tool-calling loop và CAD mutation đã được nghiệm thu.

## 7. S4 — KIỂM THỬ NGHIỆM THU VÀ ĐÓNG GÓI

Giữ các test cũ và bổ sung kiểm thử theo hành vi; không đặt mục tiêu một con số test tùy ý.

Bài chính:
A. Tạo project, mở chat, chọn MockProvider + CAD mock.
B. Gửi: "Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm."
C. Có preview; trước approval số entity không tăng.
D. Reject không tạo hình.
E. Approve chỉ tạo một chi tiết, readback/validator pass.
F. Tâm lỗ đúng (20,20), (180,20), (180,100), (20,100), bán kính 5.
G. Gửi: "Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép."
H. Sửa đúng chi tiết cũ; tâm lỗ phía phải có X = 220; giữ height, radius và offset.
I. Revert qua duyệt trả về trạng thái trước resize khi không có sửa ngoài.
J. Đóng/mở ứng dụng giữ project, chat và lịch sử; CAD state được đối chiếu đúng.

Các ca chặn lỗi bắt buộc:
- Hai lần approve/call cùng operation không tạo trùng.
- Revision đổi sau preview -> conflict, không ghi.
- UI gửi payload khác hash đã xem -> từ chối.
- Permit hết hạn, sai document hoặc phiên cũ -> từ chối.
- Mutation commit rồi mất kết nối -> outcome_unknown/recovery, không vẽ lại.
- Provider stream có JSON chưa đủ/sai -> không gọi mutation.
- Tool đọc key, chạy shell, đổi policy hoặc tự duyệt -> không có quyền.
- Sidecar crash -> UI báo mất kết nối; restart không replay thay đổi.
- Vault lỗi -> không fallback plaintext.
- Provider 401/429/timeout -> trạng thái lỗi đúng, không fake mock success.
- Migration database cũ giữ dữ liệu.
- Kiểm tra secret không xuất hiện trong DB/log/event/artifact kiểm thử.
- Native app gọi Rust thật; browser với mock invoke không được gọi là native E2E.

Phân biệt kết quả:
1. Unit/integration/HTTP fixture.
2. MCP integration với process thật.
3. Native Windows smoke/E2E.
4. Windows credential integration.
5. Live provider smoke.
6. AutoCAD live: ngoài phạm vi lượt này, chưa được xác minh.

### Windows package

- Build Tauri bằng Windows toolchain khi có môi trường.
- Ưu tiên installer NSIS .exe cho bản thử nghiệm; giữ lựa chọn khác nếu dự án đã dùng và có lý do.
- Đóng gói sidecar đúng cấu hình, không phụ thuộc source checkout hay cargo khi người dùng chạy app đã cài.
- Thử chạy app từ thư mục cài đặt/đường dẫn có khoảng trắng khi môi trường cho phép.
- Không cần API key hoặc AutoCAD để mở demo.
- Installer và app executable là hai artifact khác nhau; ghi đường dẫn và vai trò.
- Nếu thực sự tạo được artifact, báo size/hash và log build. Không tạo file rỗng/đổi đuôi để giả .exe.
- Dev build unsigned phải ghi rõ; không yêu cầu tắt Defender/SmartScreen hoặc tắt kiểm tra chữ ký để "hoàn thành".
- Không bật auto-update khi chưa có quy trình xác minh/ký.
- Thiếu Windows runner: để NOT BUILT / NOT RUN tương ứng, không bỏ phần code độc lập còn làm được.

## 8. TÀI LIỆU VÀ CÁCH KẾT THÚC

Cập nhật:
- README.md: đúng thư mục/lệnh chạy; desktop khác frontend; WSL khác Windows.
- docs/BASELINE_AUDIT.md.
- docs/ARCHITECTURE.md và docs/DATABASE.md.
- docs/TESTING.md; docs/WINDOWS_SETUP.md nếu chưa có.
- SECURITY.md.
- IMPLEMENTATION_STATUS.md.
- HANDOFF.md.

Trong status, tách các cột:
Capability | Implemented? | Test kind | Test result | Environment | Evidence | Remaining.

Không ghi chung "đã test" khi thực ra mới typecheck, mới có fixture hoặc chỉ có workflow CI chưa chạy.
Không đếm 9 + 3 test cũ thành kết quả mới nếu chưa chạy lại.

Kết thúc mỗi mốc bằng:
- Đường chạy nào thực sự hoạt động.
- File/module đã sửa.
- Lệnh + cwd + exit code/kết quả.
- Mock / live / native / chưa kiểm thử.
- Blocker thật và bước tiếp theo cụ thể.

Không hỏi lại stack hoặc phạm vi đã chốt. Không đưa estimate thời gian và không hứa làm nền.
Nếu bị giới hạn phiên, giữ repository nhất quán, cập nhật HANDOFF với bước tiếp theo; không tuyên bố xong toàn dự án.

BẮT ĐẦU: kiểm tra repository và baseline (S0), sửa đường build/startup, rồi triển khai S1 -> S2 -> S3 -> S4. Mỗi mốc có kiểm tra. Không chỉ trả về một kế hoạch mới.

## 9. NGUỒN CHÍNH THỨC ĐỂ ĐỐI CHIẾU

Các tài liệu này được xem khi soạn prompt; AI triển khai phải kiểm tra theo dependency thực tế. Ví dụ code trong tài liệu không thay cho test trong repository.

- Tauri prerequisite Windows/Linux:
  `https://v2.tauri.app/start/prerequisites/`
- Tauri development và lệnh khởi chạy desktop:
  `https://v2.tauri.app/develop/`
- Tauri Windows installer và giới hạn cross-compilation:
  `https://v2.tauri.app/distribute/windows-installer/`
- Tauri sidecar và externalBin:
  `https://v2.tauri.app/develop/sidecar/`
- Tauri capabilities, bao gồm custom application commands:
  `https://v2.tauri.app/security/capabilities/`
- MCP Rust SDK chính thức:
  `https://github.com/modelcontextprotocol/rust-sdk`
- OpenAI function calling:
  `https://developers.openai.com/api/docs/guides/function-calling?api-mode=responses`
- OpenAI streaming:
  `https://developers.openai.com/api/docs/guides/streaming-responses`
- Windows Credential Manager write/read:
  `https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew`
  `https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw`
