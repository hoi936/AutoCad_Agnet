# MASTER PROMPT — CAD AGENT STUDIO / FOUNDATION V0

> Đây là đặc tả để giao cho AI lập trình xây repository và bản nền chạy được, không phải xác nhận một sản phẩm đã được xây hoặc kiểm thử.
> Ngôn ngữ trao đổi, README và giao diện mặc định: tiếng Việt. Tên mã nguồn, API, bảng dữ liệu và commit: tiếng Anh.
> Các quyết định dưới đây là kiến trúc đề xuất. Tài liệu kỹ thuật tham chiếu đã được đối chiếu ngày 05/10/2026; phải kiểm tra lại phiên bản khi triển khai.

---

## 0. VAI TRÒ VÀ CÁCH THỰC HIỆN

Bạn là kỹ sư trưởng chịu trách nhiệm kiến trúc, lập trình Rust/Tauri, React/TypeScript, C# AutoCAD, cơ sở dữ liệu và kiểm thử bảo mật.

Hãy XÂY DỰNG repository “CAD Agent Studio” theo đặc tả này. Không chỉ trả về kế hoạch, ảnh giao diện hay những đoạn mã rời rạc.

Trước khi sửa mã:
1. Kiểm tra thư mục làm việc, mã hiện có, hệ điều hành và các công cụ đã cài. Không ghi đè hoặc xóa công việc có sẵn.
2. Đọc toàn bộ đặc tả; ghi các quyết định và giả định không chặn tiến độ vào tài liệu.
3. Kiểm tra tài liệu chính thức của các dependency quan trọng. Chọn bản stable tương thích; khóa phiên bản thực sự đã thử. Không bịa tên package, SDK, model, phương thức AutoCAD hoặc trạng thái kiểm thử.
4. Lập danh sách công việc theo các mốc ở phần 20, sau đó triển khai ngay mốc đầu.
5. Với môi trường thiếu AutoCAD, SDK hoặc API key: hoàn thành đường chạy mô phỏng và các thành phần không phụ thuộc chúng; báo rõ phần chưa kiểm thử. Không để thiếu AutoCAD chặn việc xây frontend, backend, database và MCP.

Trong quá trình làm:
- Ưu tiên một luồng chức năng chạy xuyên suốt hơn nhiều màn hình giả.
- Không mở rộng sang cloud, thanh toán, nhiều agent hoặc marketplace khi Foundation chưa đạt nghiệm thu.
- Không chạy lệnh phá hủy, tắt kiểm tra bảo mật, sửa chính sách hệ thống hoặc cài SDK thương mại trái phép.
- Không yêu cầu cung cấp API key trong chat. Khóa được người dùng nhập trong ứng dụng sau khi chạy.
- Không đánh dấu một tác vụ hoàn thành chỉ vì đã tạo file.
- Nếu hết ngữ cảnh, ghi trạng thái thực tế vào HANDOFF.md: đã làm, lệnh đã chạy, lỗi còn lại và bước tiếp theo. Không tuyên bố làm nền hoặc hứa giao kết quả sau.
- Báo tiến độ theo mốc có kết quả; tránh hỏi lại các lựa chọn đã được chốt trong prompt.

## 1. MỤC TIÊU SẢN PHẨM

Xây ứng dụng desktop Windows dạng .exe, trải nghiệm tương tự agent trong IDE, hỗ trợ người dùng tạo và chỉnh sửa bản vẽ AutoCAD bằng ngôn ngữ tự nhiên.

Đường đi cốt lõi:
Prompt -> đọc ngữ cảnh -> chọn skill -> lập phương án hình học -> kiểm tra -> preview -> người dùng duyệt -> thực hiện -> đọc lại -> xác minh -> lưu lịch sử.

Người dùng phải phân biệt được:
- Lời giải thích hoặc phương án do LLM tạo.
- Kết quả kiểm tra bằng mã.
- Thay đổi mới chỉ được đề xuất.
- Thay đổi thực sự đã áp dụng vào mô phỏng hoặc AutoCAD.
- Thay đổi chưa được lưu thành file DWG.

Không quảng cáo rằng LLM có thể thiết kế mọi chuyên ngành hoặc bảo đảm an toàn kỹ thuật. Foundation chỉ làm hình học 2D có tham số trong phạm vi đã kiểm thử.

### Hai môi trường chạy độc lập

**MOCK:** chạy không cần AutoCAD và không cần API key; dùng bộ cung cấp phản hồi thử nghiệm cùng bộ mô phỏng CAD xác định. Có nhãn “MÔ PHỎNG — KHÔNG PHẢI AUTOCAD”. Không tạo JSON rồi đổi đuôi thành DWG.

**AUTOCAD:** kết nối AutoCAD đầy đủ trên Windows bằng plugin C#; dùng cùng schema, policy, approval và UI. Không hỗ trợ AutoCAD LT, macOS, Civil 3D hoặc các toolset chuyên dụng trong Foundation trừ khi có kiểm thử riêng.

Việc chọn LLM mock/live phải độc lập với chọn CAD mock/AutoCAD. Như vậy có thể thử LLM thật trên CAD mô phỏng trước khi cho phép sửa bản vẽ thật.

## 2. STACK ĐÃ CHỐT

| Phần | Công nghệ đề xuất |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend | React, TypeScript strict, Vite |
| UI | Tailwind CSS; component dễ truy cập, ưu tiên tái sử dụng một thư viện thống nhất |
| UI state | Zustand cho trạng thái giao diện; TanStack Query cho dữ liệu backend |
| Local backend | Rust stable, Tokio, Serde, thiserror, tracing |
| HTTP tới LLM | reqwest với kiểm tra TLS; chạy trong Rust |
| Database | SQLite qua SQLx; migration SQL thật |
| MCP | SDK Rust chính thức rmcp, bản stable được kiểm chứng |
| CAD bridge | C# + AutoCAD Managed .NET API |
| Secret store | Windows Credential Manager qua lớp bọc an toàn |
| Frontend tests | Vitest + React Testing Library |
| Desktop E2E | WebDriver theo tài liệu Tauri hiện hành, trên Windows |
| Backend tests | Rust unit/integration/property tests |
| C# tests | Bộ test .NET riêng cho DTO, validation và IPC không cần AutoCAD |
| Installer | Tauri Windows installer; cấu hình đóng gói sidecar và plugin |

Đây là ứng dụng local-first, không phải website cần REST server trên Internet. Không tạo NestJS, Express, FastAPI, Redis, Kubernetes, Supabase hay PostgreSQL cho bản đầu. Không mở HTTP listener chỉ để nối frontend với Rust; dùng Tauri IPC.

Rust giúp giảm một số lỗi an toàn bộ nhớ trong phần mã Rust an toàn, không tự giải quyết prompt injection, lỗi phân quyền, dependency độc hại hay máy người dùng đã bị chiếm quyền. Core nghiệp vụ dùng `#![forbid(unsafe_code)]`; nếu cần tương tác Windows mức thấp, cô lập ở lớp platform nhỏ hoặc dùng thư viện có safe wrapper, kèm kiểm tra riêng. Không tuyên bố toàn bộ dependency không có unsafe. Tham chiếu [R1–R3].

### Chọn phiên bản AutoCAD

Trước khi build bridge thật, xác định release, build/update và bộ managed assemblies thực tế.

Tại thời điểm đối chiếu:
- AutoCAD 2026 đến Update 1.1 dùng .NET 8.
- AutoCAD 2026 Update 1.2 trở đi dùng .NET 10.
- Tương thích plugin còn phải thử với bản AutoCAD và dependency thật. Không suy luận rằng tất cả DLL đều dùng chung được.

Ưu tiên một cấu hình AutoCAD 2026 đã xác minh. TargetFramework/SDK reference phải khớp môi trường mục tiêu, không hardcode net8 cho mọi bản 2026. Không tự nâng cấp AutoCAD của người dùng. Ghi cấu hình hỗ trợ vào COMPATIBILITY.md. Tham chiếu [R4].

## 3. PHẠM VI FOUNDATION V0

Phải có:
- Desktop app thật, frontend kết nối Rust backend thật.
- Quản lý project và workspace local.
- Database có migration, lịch sử và policy lưu dữ liệu.
- Chat streaming, bộ điều phối agent có giới hạn.
- MockProvider hoàn chỉnh và một adapter OpenAI Responses API có triển khai thực.
- Secret store Windows; UI quản lý provider không làm lộ khóa.
- MCP client + sidecar server hoạt động thực bằng stdio.
- Bộ mô phỏng CAD có trạng thái, dùng chung contract với bridge.
- Workflow preview/approve/apply/verify/revert có kiểm tra trạng thái.
- Một skill tạo/sửa tấm đế 2D có bốn lỗ.
- Source code plugin C# và cấu hình build có điều kiện theo SDK.
- Test, tài liệu chạy, cấu hình đóng gói Windows.

Chưa làm:
- OAuth ChatGPT, đăng nhập tài khoản sản phẩm, cloud sync, thanh toán.
- Nhiều nhà cung cấp LLM đã tích hợp hoàn chỉnh.
- Chạy AutoLISP, Python, C#, JavaScript hoặc shell do LLM/skill tùy ý sinh.
- Điều khiển chuột/phím toàn hệ điều hành.
- Đọc/sửa mọi loại DWG, 3D, BIM, tính kết cấu hoặc thiết kế MEP.
- Tự động lưu đè DWG, tự mở/xóa file ngoài workspace.
- Nhúng toàn bộ cửa sổ AutoCAD hoặc viết một CAD engine thay AutoCAD.
- Marketplace skill, thực thi mã từ ZIP, fine-tuning, vector database.

Không được gắn nhãn “production-ready” hoặc “AutoCAD integration verified” chỉ dựa trên mô phỏng.

## 4. KIẾN TRÚC VÀ QUYỀN SỞ HỮU DỮ LIỆU

```text
React UI trong Tauri WebView
          |
     Typed Tauri IPC
          |
Rust App Core
  - Project / Conversation Services
  - Agent Runtime / ModelProvider
  - Skill Registry / Geometry Planner
  - Policy / Approval / Recovery
  - SQLite Repository / SecretStore
          |
      MCP client
          |
     private stdio
          |
Rust cad-tool-host.exe
  - MCP server
  - schema / permit validation
  - CadGateway
        |                     |
   MockCadGateway        Named Pipe local
                              |
                       C# AutoCAD Bridge
                              |
                       AutoCAD DWG database
```

Quy tắc:
- App Core là chủ sở hữu duy nhất của SQLite và API key.
- Frontend không truy cập trực tiếp SQLite, named pipe, HTTP provider hoặc file hệ thống.
- Tool host không có API key của LLM, không có quyền tùy ý ghi SQLite.
- Plugin không gọi LLM, không chứa API key, không quyết định người dùng đã duyệt.
- MCP là lớp giao tiếp công cụ; không phải cơ chế tự cấp quyền cho model.
- AutoCAD/DWG là nguồn sự thật của hình học thật. SQLite lưu lịch sử, tham số, ánh xạ và bằng chứng thực thi; không thay thế DWG.
- Dữ liệu mô phỏng tách khỏi dữ liệu AutoCAD thật bằng mode và thư mục rõ ràng.
- Không gửi toàn bộ DWG lên cloud. Chỉ gửi phần ngữ cảnh cần thiết và được phép.

## 5. CẤU TRÚC REPOSITORY

Tổ chức dạng monorepo; có thể điều chỉnh nhỏ để phù hợp toolchain, không làm sai ranh giới:

```text
cad-agent-studio/
  apps/
    desktop/
      src/
        app/
        features/projects/
        features/chat/
        features/changes/
        features/skills/
        features/settings/
        features/diagnostics/
        components/
        lib/ipc/
      src-tauri/
        src/
        capabilities/
        permissions/
        tauri.conf.json
        Cargo.toml
  crates/
    app-core/
      src/
        agent/
        providers/
        projects/
        skills/
        geometry/
        policy/
        approvals/
        recovery/
        storage/
    contracts/
    cad-tool-host/
    platform-windows/
  dotnet/
    CadAgent.Contracts/
    CadAgent.Bridge/
    CadAgent.Bridge.Tests/
  contracts/
    schemas/
    fixtures/
  database/
    migrations/
    seeds/
  skills/
    mounting-plate-2d/
      SKILL.md
      manifest.json
      input.schema.json
      references/conventions.md
      tests/cases.json
  tests/
    integration/
    desktop-e2e/
    security/
  scripts/
  docs/
  .github/workflows/
  Cargo.toml
  Cargo.lock
  pnpm-workspace.yaml
  pnpm-lock.yaml
  README.md
  SECURITY.md
  IMPLEMENTATION_STATUS.md
  HANDOFF.md
```

Không tạo hàng chục package rỗng. Chỉ tách crate khi có ranh giới đáng bảo vệ hoặc cần chia sẻ contract. Workspace Rust mặc định không phụ thuộc SDK AutoCAD.

## 6. CONTRACT, COMMAND VÀ EVENT

Chọn một nguồn định nghĩa contract chuẩn. Sinh type TypeScript/C# khi phù hợp hoặc dùng JSON Schema cùng golden fixtures kiểm tra tương thích. Không duy trì ba bản DTO độc lập dễ lệch nhau.

Tối thiểu có:
- Project, ProviderConfigPublic, CadConnection, DocumentContext.
- EntitySummary, SemanticObject, PlateParameters.
- AgentRun, ToolCall, ChangeSet, ValidationReport.
- ApprovalSummary, ExecutionReceipt, AppError, RunEvent.

Mọi input validate trong backend dù frontend đã validate. Hạn chế trường lạ, số không hữu hạn, chuỗi quá dài, payload quá lớn. Không dùng TypeScript `any` để bỏ qua lỗi contract.

### Command giao diện -> backend

```text
projects_list / projects_create / projects_update / projects_archive
workspace_select / document_attach / document_detach
conversations_create / conversations_list / messages_list
providers_list / provider_save_config / provider_set_secret
provider_delete_secret / provider_test_connection
cad_discover / cad_connect / cad_disconnect / cad_get_context
agent_start_run / agent_cancel_run / agent_get_run
changes_get / changes_approve / changes_reject / changes_revert
skills_list / skills_get / project_skill_set_enabled
settings_get / settings_update
history_export / diagnostics_export / database_backup
```

Đây là API của ứng dụng cần tự xây, không phải API Tauri có sẵn. Viết contract và test cho từng command được thực hiện.

`provider_set_secret` nhận khóa một lần từ form; các command đọc tuyệt đối không trả lại khóa. `changes_approve` chỉ nhận ID phương án và hash mà UI đang hiển thị, không nhận một gói thao tác mới do frontend tự viết.

Không tạo command chung kiểu `execute_sql`, `run_command`, `read_any_file`, `execute_code` hoặc `invoke_any_tool`.

### Event backend -> UI

Có `run_id`, `sequence`, `timestamp`, `type`, `payload`; sequence tăng theo run. Ví dụ:
- run.started / run.status_changed / run.completed / run.failed
- assistant.delta
- tool.started / tool.completed / tool.failed
- change.preview_ready / change.approval_required
- change.applied / change.verified / change.reverted
- cad.connection_changed / cad.document_changed

UI xử lý event trùng và lấy lại trạng thái qua command khi kết nối lại. Không ghi từng token vào SQLite; gom message khi hoàn thành hoặc theo checkpoint có giới hạn. Không phát API key, permit hoặc dữ liệu bí mật trong event.

AppError tối thiểu:
`code`, `message`, `retryable`, `correlation_id`, `safe_details`.
Không trả stack trace nội bộ hoặc toàn bộ request có Authorization header về UI.

## 7. FRONTEND: MỘT ỨNG DỤNG CÓ THỂ DÙNG, KHÔNG CHỈ KHUNG CHAT

Phong cách: công cụ kỹ thuật desktop; bố cục gọn, dễ đọc, panel co giãn, dark/light theme, bàn phím và focus rõ ràng. Không sao chép nhãn hiệu hoặc tài nguyên riêng của VS Code.

### Bố cục

- Trái: danh sách project, bản vẽ đã gắn, hội thoại, skill được bật.
- Giữa: chat, kế hoạch ngắn, tool activity và ô nhập nhiều dòng.
- Phải: preview hình học 2D, bảng thay đổi, lỗi validation, nút duyệt/từ chối.
- Thanh trạng thái: CAD mock/real, document đích, đơn vị, provider/model, trạng thái run và dữ liệu có được gửi cloud hay không.
- Settings: provider, khóa API, quyền dữ liệu, theme, retention và diagnostics.

### Luồng onboarding

Cho phép dùng ngay Demo Mode mà không đăng nhập. Giải thích rõ demo không phải LLM thật hoặc AutoCAD thật.
Tạo project local, chọn workspace qua hộp thoại hệ thống, chọn CAD mode, chọn model provider.
Khi bật cloud provider, hiển thị loại dữ liệu dự kiến rời máy và yêu cầu chấp nhận theo project/provider.

### Chat và thay đổi

Ba chế độ:
- Ask: giải thích/đọc; không sửa.
- Plan: tạo phương án/preview; không áp dụng.
- Execute with approval: được đề xuất thay đổi nhưng phải duyệt trước khi ghi.

Hiển thị:
- Nội dung assistant, trạng thái thực thi, công cụ và kết quả đã làm sạch.
- Tóm tắt kế hoạch, không yêu cầu hoặc hiển thị suy nghĩ nội bộ của model.
- Tool call lỗi không biến thành thông báo thành công.
- Nút Stop có trạng thái “đã yêu cầu dừng” khi chưa thể dừng an toàn.

Preview dùng primitive có schema do backend tạo; React render SVG/canvas. Không chèn raw SVG/HTML từ LLM. Phải xử lý đúng trục Y, đơn vị và fit-to-view.
Preview Foundation chỉ là hình học dự kiến và đối tượng liên quan, không giả vờ là screenshot hoặc trình xem đầy đủ DWG.

Màn duyệt hiện rõ:
Tên document, mode, đơn vị, số đối tượng thêm/sửa/xóa, tham số cũ/mới, cảnh báo, kết quả validation và trạng thái base revision.
Nút Apply bị khóa khi validation lỗi, document đổi, approval hết hạn hoặc mất kết nối.
Một phương án bị chỉnh tham số phải tạo phiên bản mới và xin duyệt lại.

Mọi màn hình có loading, empty, error, disconnected và recovery state. Điều khiển chưa triển khai phải bị vô hiệu hóa kèm lý do; không để nút giả luôn báo thành công.

## 8. BACKEND VÀ VÒNG ĐIỀU PHỐI AGENT

Tách các interface:
`ModelProvider`, `CadGateway`, `SkillRepository`, `ProjectRepository`, `SecretStore`, `ApprovalService`, `Clock`, `IdGenerator`.

Inject clock/ID/provider/gateway giả để test xác định; tránh phụ thuộc global không kiểm thử được.

Agent nhận request người dùng, đọc project policy, chọn skill đã bật và lấy ngữ cảnh CAD tối thiểu. Sau đó:
1. Gọi model với tool definitions được policy cho phép.
2. Ghép đủ tool arguments khi streaming; validate schema và nghiệp vụ.
3. Chạy tool đọc hoặc tool đề xuất; trả kết quả có cấu trúc về model.
4. Khi có ChangeSet, dừng ở chờ duyệt. Không cho LLM tự phê duyệt.
5. Sau áp dụng, lấy receipt và kiểm tra lại trước khi trả lời hoàn tất.

Không cung cấp tool phê duyệt, tự cấp quyền, sửa policy, đọc secret hoặc chạy shell cho model.
Tool `cad.apply_change_set` là công cụ nội bộ được bảo vệ, không xuất hiện trong tập tool gửi cho LLM.

Giới hạn mặc định có thể cấu hình:
- Một mutation đang chạy trên mỗi document.
- Tối đa 12 tool calls cho một lượt tự động trước khi dừng có giải thích.
- Tối đa 2 lần sửa lỗi arguments/validation.
- Timeout từng tool; giới hạn tổng thời gian xử lý không tính thời gian đợi người duyệt.
- Giới hạn request, output, số entity, tổng token và số vòng lặp.
- Không tự retry mutation khi kết quả chưa rõ.

Chi phí chỉ hiển thị khi có dữ liệu usage và bảng giá cấu hình đáng tin. Nếu chỉ là ước tính, ghi “ước tính”; thiếu giá thì hiển thị token thay vì tự bịa số tiền.

## 9. MODEL PROVIDER, ĐĂNG NHẬP VÀ SECRET

### MockProvider

Hoạt động không dùng Internet và không cần credential.
Dùng fixture có tính xác định cho các câu lệnh demo; không pretend là mô hình hiểu mọi câu.
Mô phỏng được streaming, tool call, JSON sai, rate limit, timeout và cancel.
UI luôn phân biệt MockProvider với provider thật.

### OpenAI adapter

Triển khai trong Rust qua Responses API theo tài liệu hiện hành [R8].
Model ID cấu hình được; không hardcode tên model không được xác minh.
Hỗ trợ streaming text, tool arguments hoàn chỉnh, gửi tool outputs và tiếp tục hội thoại.
Xử lý timeout, 401/403, 429, 5xx, kết nối mất và response sai định dạng.
Không gửi các tham số mà model/API được chọn không hỗ trợ.
Dùng schema strict khi được hỗ trợ, nhưng vẫn validate phía ứng dụng.
Không cho API phía cloud kết nối trực tiếp tới named pipe hoặc tự gọi tool local; App Core là bên thực thi.

Chỉ dùng endpoint nhà cung cấp đã được duyệt. Không cho model sửa base URL.
Không theo redirect mang credential sang origin khác.
Không vô hiệu hóa TLS verification.
Profile cloud không được trỏ tới loopback/private network tùy tiện. Provider local là tính năng riêng để sau, với policy riêng.

### Secret và tài khoản

Foundation không có account cloud, user/password table hay JWT tự tạo.
“Local profile” là cấu hình ứng dụng, không phải cơ chế xác thực doanh nghiệp.

Khóa API:
- Người dùng nhập trong ô password, chuyển ngay qua Tauri IPC và xóa khỏi form/state sau xử lý.
- Rust lưu trong Windows Credential Manager; SQLite chỉ lưu `secret_ref`.
- Không dùng localStorage, sessionStorage, file JSON hay .env làm nơi lưu credential của ứng dụng đã cài.
- Không ghi khóa vào log, analytics, URL, crash report, prompt hoặc MCP.
- Không có API lấy lại khóa rõ; chỉ trả `configured: true/false`.
- Có thao tác thay/xóa khóa và xử lý lỗi vault không truy cập được.
- Test không-Windows dùng MemorySecretStore dành riêng cho test; không âm thầm fallback lưu plaintext.
- Có thể hỗ trợ session-only secret để người dùng không lưu bền vững.

Credential Manager bảo vệ theo tài khoản Windows nhưng không phải phòng thủ tuyệt đối khi cùng tài khoản hoặc hệ điều hành đã bị chiếm quyền [R9].
Không thu thập mật khẩu/cookie ChatGPT hoặc Autodesk; không tự động đăng nhập bằng browser session. OAuth chính thức là hạng mục tương lai, không giả lập màn đăng nhập thành công.

## 10. BÀI TOÁN HÌNH HỌC ĐẦU TIÊN: TẤM ĐẾ 2D

Đây là workflow bắt buộc chạy xuyên suốt, không phải demo UI.

Prompt mẫu:
“Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm lỗ cách hai mép gần nhất 20 mm. Tách layer và ghi kích thước.”

Quy ước Foundation:
- Model space, hình 2D phẳng trong WCS XY, Z = 0, không xoay.
- Điểm gốc mặc định (0, 0) phải xuất hiện trong preview để người dùng duyệt.
- Đơn vị domain là mm; conversion sang đơn vị bản vẽ được ghi rõ.
- Bản vẽ unitless/không rõ đơn vị phải yêu cầu người dùng xác nhận đơn vị, không âm thầm giả định.
- Không thay INSUNITS hoặc thiết lập toàn bản vẽ chỉ để khớp giả định của agent.
- Entity không hỗ trợ, paper space, hệ tọa độ hoặc block transform ngoài phạm vi phải báo rõ; không chỉnh sửa suy đoán.

Hình học do Rust tính bằng thuật toán xác định:
- Một closed polyline hình chữ nhật bốn đỉnh.
- Bốn circle bán kính 5.
- Tâm lỗ: (20,20), (180,20), (180,100), (20,100).
- Layer đề xuất: AI_OUTLINE, AI_HOLES, AI_DIM; không thay thuộc tính layer sẵn có ngoài phạm vi đã duyệt.
- Bridge tạo dimension thật cho chiều dài, chiều rộng và đường kính đại diện; không dùng text giả để che sai kích thước.

Parameter schema có version: width, height, hole_diameter, edge_offset, origin, units, add_dimensions.
Kiểm tra finite numbers, bounds, radius > 0, edge_offset > radius, width - 2*edge_offset > 2*radius, height - 2*edge_offset > 2*radius.
Sai số hình học có cấu hình rõ ràng theo đơn vị/scale; không so sánh float bằng == một cách mù quáng.

Prompt tiếp:
“Tăng chiều dài lên 240 mm, giữ khoảng cách từ tâm lỗ tới mép.”
Kết quả:
- Tấm cũ được sửa, không vẽ chồng thêm tấm mới.
- Hai tâm lỗ phải sang x = 220; hai tâm bên trái giữ x = 20.
- Chiều rộng 120, bán kính 5, vị trí theo Y và offset được giữ.
- Dimension và semantic parameters phải cập nhật.
- Entity mappings được kiểm tra trước khi sửa.

SemanticObject quản lý toàn bộ chi tiết; bindings nối từng vai trò với entity handle.
Không coi handle là ID toàn cầu. Luôn gắn handle với document identity và kiểm tra lại khi reopen, Save As, copy, undo hoặc đối tượng bị xóa.
Metadata trong DWG thuộc namespace riêng của plugin; không ghi đè metadata ứng dụng khác.

Validator đọc lại dữ liệu gateway, kiểm tra số entity, kiểu, tọa độ, radius, closed polyline, layer, dimension và quan hệ tham số. Không dùng câu trả lời của LLM làm bằng chứng hình học đúng.

## 11. MCP TOOL HOST VÀ AUTOCAD BRIDGE

### MCP thực sự hoạt động

Dùng rmcp chính thức [R5]; khóa phiên bản protocol/SDK đã thử.
App Core spawn đúng sidecar do ứng dụng đóng gói bằng đường dẫn cố định, không tìm executable tùy ý qua PATH.
Dùng private stdin/stdout; stdout chỉ chứa thông điệp protocol, log ra stderr và được làm sạch.
Không truyền API key sang process con; giảm environment được kế thừa.
Có lifecycle negotiation, tool discovery, timeout, shutdown, restart có kiểm soát và giới hạn kích thước.
Không tự viết một giao thức JSON rồi quảng cáo là MCP nếu không tương thích SDK đã chọn.

Các tool đọc tối thiểu:
`cad.get_status`, `cad.get_document_context`, `cad.get_selection`, `cad.query_entities`, `cad.validate_entities`, `cad.get_operation_status`.

Tool domain phía App Core:
`design.propose_plate`, `design.propose_plate_resize`.
Chúng chỉ tạo ChangeSet/preview; không ghi DWG.

Tool nội bộ được bảo vệ:
`cad.apply_change_set`, `cad.revert_change_set`.
Không đưa chúng hoặc tool approval vào tool definitions của model.
Sidecar vẫn phải kiểm tra authorization permit: ẩn tool khỏi LLM không phải biện pháp phân quyền duy nhất.
MCP annotation là metadata, không phải bằng chứng caller được phép ghi.

### Named pipe Windows

Plugin mở pipe local với quyền chỉ tài khoản Windows phù hợp, kiểm tra cùng mức elevation; ưu tiên CurrentUserOnly và ACL tối thiểu [R10].
App và AutoCAD chạy quyền người dùng thường; không dùng “Run as administrator” làm cách sửa lỗi mặc định.
Nếu elevation không khớp, báo nguyên nhân, không hạ bảo mật pipe.
Discovery không chứa secret; xác minh server process, user và session trước khi tin kết nối.
Có bước ghép cặp/kết nối được người dùng xác nhận trong AutoCAD cho phiên thực thi.
Phiên mới tạo session identity mới; permit của phiên cũ mất hiệu lực.
Chọn framing cụ thể có length prefix, UTF-8, version, request ID, tối đa 1 MiB/frame, timeout và giới hạn queue; test framing trong Rust/C#.
Pipe listener chỉ parse/validate dữ liệu. Không gọi AutoCAD API trực tiếp từ background thread.

### Plugin C#

Tách DTO/validation/transport khỏi phần tham chiếu Autodesk để test phần độc lập.
Đọc release/build/runtime và capability thật khi handshake.
Hàng đợi đưa thao tác về AutoCAD main thread và đúng document/command context; không dùng async continuation tùy tiện.
Dùng document lock khi ngữ cảnh yêu cầu, transaction và dispose đúng. Không giữ lock hoặc transaction trong lúc đợi LLM/người dùng. Tham chiếu [R6–R7].
Không dùng SendStringToExecute làm đường vẽ chính.
Không gọi ExecuteInCommandContextAsync hoặc API khác bằng chữ ký đoán; kiểm tra SDK thật.

Có lệnh AutoCAD để connect/disconnect/status với tên riêng của plugin; disconnect thu hồi session.
Kiểm tra document đích vẫn mở, còn đúng identity, đúng model space, chưa đổi revision và không đang bận trước khi sửa.
Base có thể từ chối mutation khi document đích không active; không tự chuyển document âm thầm.

Build:
- Autodesk managed references lấy từ cấu hình đường dẫn SDK/AutoCAD hợp lệ.
- Không commit hoặc giả lập DLL Autodesk.
- Assemblies host không copy nhầm vào bundle như dependency riêng.
- Missing SDK gây lỗi build bridge rõ ràng; không làm build app/mock thất bại.
- Bundle manifest và hướng dẫn cài theo cơ chế Autodesk.
- Không tự đặt SECURELOAD = 0 hoặc thêm mọi thư mục vào trusted paths.

### MockCadGateway

Dùng cùng request/response/precondition/permit và validation như gateway thật.
Có state, revision, entity IDs, operation receipts và khả năng giả lập busy/disconnect/conflict.
Mock app mode có thể lưu snapshot JSON riêng bằng atomic replace; test dùng in-memory state.
Không dùng cùng namespace document ID với AutoCAD thật.
Không trả thành công mặc định cho thao tác chưa hỗ trợ.

## 12. APPROVAL, THỰC THI, RECOVERY VÀ HOÀN TÁC

### ChangeSet bất biến

Tối thiểu gồm:
`id`, `project_id`, `document_id`, `cad_session_id`, `base_revision`,
`schema_version`, `operation_type`, `parameters`, `operations`,
`affected_entities`, `preconditions`, `preview`, `validation`,
`payload_hash`, `created_at`, `expires_at`.

Base revision là session epoch + revision counter của plugin và fingerprint các điều kiện/đối tượng liên quan. Không giả định AutoCAD có sẵn một revision token toàn cục phù hợp.
Revision/fingerprint phải được kiểm tra lại tại điểm mutation dưới lock thích hợp, không chỉ lúc preview.
Một người dùng sửa bản vẽ trong lúc AI suy nghĩ phải làm phương án cũ bị từ chối hoặc phải lập/duyệt lại.

### Approval và permit

Chỉ ApprovalService trong App Core tạo approval khi nhận thao tác người dùng từ cửa sổ được phép.
Approval gắn với project, document, CAD session, immutable payload hash, base revision, phạm vi quyền và thời hạn.
Thay bất kỳ tham số hoặc đối tượng đích nào làm approval hết hiệu lực.
UI không được gửi lại một danh sách thao tác khác trong lệnh approve.

Backend tạo permit ngắn hạn, chống sửa đổi, không đưa vào prompt, log hoặc UI event. Gateway/bridge xác minh permit và payload tương ứng trước khi ghi.
Có thể dùng MAC từ thư viện chuẩn với session key bootstrap qua kênh local đã ghép cặp; tài liệu hóa cách phân phối key, đổi session và chống replay. Không tự phát minh thuật toán mật mã.
Permit là quyền cho một operation ID. Retry cùng operation phải trả receipt đã có sau đối chiếu, không thực hiện thêm lần nữa.
Process không được bootstrap như child/session tin cậy mặc định chỉ được đọc; không có write permit thì từ chối mutation.
Không coi những biện pháp này là phòng thủ tuyệt đối trước malware cùng tài khoản Windows hoặc backend đã bị chiếm quyền.

### Trạng thái

Agent run:
queued -> planning -> waiting_approval -> executing -> verifying -> succeeded.
Có trạng thái riêng: rejected, cancelled, failed, conflict, recovery_required. Ask/Plan có thể kết thúc sau đọc/lập phương án mà không đi qua nhánh executing; không gán succeeded cho một thay đổi chưa áp dụng.

ChangeSet:
draft -> validated -> pending_approval -> approved -> applying -> applied -> verified.
Có nhánh rejected, expired, failed, conflict, outcome_unknown, revert_pending, reverted.

Chuyển trạng thái qua service/transaction có compare-and-set. Không cho frontend tự cập nhật status trực tiếp.
Cancel khi mutation đang chạy là yêu cầu dừng, không đồng nghĩa rollback đã xong. Nếu đã commit, trả kết quả thật và đề xuất quy trình revert riêng.

### Idempotency và crash recovery

Mỗi mutation có operation_id duy nhất và payload_hash.
Chống double click, tool call lặp và request gửi lại.
Nếu mất kết nối sau khi gửi mutation, trạng thái là outcome_unknown; gọi get_operation_status và kiểm tra DWG trước khi quyết định.
Không tuyên bố “exactly once” giữa SQLite, pipe, DWG và filesystem.

Bridge ghi metadata/receipt marker của operation cùng transaction hình học nếu API và phạm vi hỗ trợ.
Marker và hình học còn trong database AutoCAD đang mở không có nghĩa DWG đã được lưu xuống đĩa.
Khi reopen, phải đối chiếu trạng thái thật: thao tác chưa lưu có thể không còn.
Nếu không chứng minh được applied/not-applied, chuyển recovery_required; chặn tự retry.
Kiểm tra các tình huống crash trước commit, sau commit trước receipt, app restart, document đóng không lưu, undo bên ngoài và receipt cũ.

### Revert

Foundation chỉ revert các kiểu entity/phép sửa được hỗ trợ và có before/after snapshot.
`changes_revert` tạo inverse ChangeSet chờ duyệt với operation_id mới và liên kết thao tác gốc; không phải quyền chạy Undo tùy ý ngay lập tức.
Kiểm tra đối tượng hiện tại vẫn khớp after-state rồi mới áp dụng inverse change đã được duyệt.
Nếu người dùng đã sửa tiếp, báo conflict; không hoàn tác mù.
Không gọi Undo nhiều lần rồi vô tình xóa công việc của người dùng.
Không tuyên bố một transaction hoàn tác được file xuất ra ngoài hoặc thao tác ngoài DWG.
Không tự Save/QSAVE/lưu đè bản vẽ gốc. Khi thành công, UI nói rõ thay đổi trong document đang mở chưa chắc đã được lưu.

## 13. THIẾT KẾ DATABASE

Dùng SQLite cho dữ liệu ứng dụng cục bộ. Tạo migration SQL thật, repository và integration test.
Không lưu nguyên DWG, thư viện block lớn hoặc API key dạng plaintext trong SQLite.
Không tạo bảng users/passwords/refresh_tokens cho một app chưa có backend tài khoản.

### Quy ước

- Primary key ứng dụng là UUID dạng TEXT; timestamp UTC thống nhất.
- Foreign key, NOT NULL, UNIQUE và CHECK cho enum/range thiết yếu.
- JSON dùng cho nội dung có cấu trúc thay đổi theo schema, không thay thế toàn bộ mô hình quan hệ.
- JSON fields có schema_version khi cần và kiểm tra json_valid.
- Validate project ownership: không lấy conversation thuộc project A nhưng gắn document thuộc project B.
- SQL parameter binding; không nối chuỗi từ người dùng thành SQL.
- Migration đã phát hành không được sửa lại; thêm migration mới.
- Xác định ON DELETE rõ ràng. Archive project mặc định không xóa DWG hoặc audit history.

### Các bảng tối thiểu và trường chính

| Bảng | Trường chính / vai trò |
|---|---|
| settings | key PK, value_json, updated_at; chỉ cấu hình không bí mật |
| provider_configs | id, kind, label, endpoint, model_id, secret_ref nullable, options_json, enabled, created_at, updated_at |
| projects | id, name, workspace_path, default_units, privacy_policy_json, settings_json, archived_at, created_at, updated_at |
| documents | id, project_id FK, mode, location_kind, relative_path nullable, approved_external_path nullable, file_identity nullable, last_fingerprint nullable, unit_metadata_json, created_at, last_seen_at |
| semantic_objects | id, document_id FK, kind, schema_version, parameters_json, created_at, updated_at |
| entity_bindings | id, semantic_object_id FK, role, entity_handle, expected_geometry_hash, updated_at; UNIQUE(semantic_object_id, role) với role riêng như hole_1, hole_2 |
| conversations | id, project_id FK, title, created_at, updated_at |
| messages | id, conversation_id FK, role, ordinal, content_json, redaction_state, created_at; UNIQUE(conversation_id, ordinal) |
| agent_runs | id, conversation_id FK, document_id FK nullable, provider_config_id FK nullable, mode, status, limits_json, usage_json nullable, error_code nullable, started_at, finished_at |
| run_events | id, run_id FK, sequence, type, safe_payload_json, created_at; UNIQUE(run_id, sequence) |
| tool_calls | id, run_id FK, provider_response_id nullable, provider_call_id nullable, tool_name, input_hash, safe_input_json, safe_output_json nullable, status, started_at, finished_at |
| change_sets | id, run_id FK, document_id FK, cad_session_id, base_revision_json, operation_kind, schema_version, immutable_payload_json, payload_hash, status, created_at, expires_at |
| approvals | id, change_set_id FK, payload_hash, decision, decision_source, permission_scope_json, session_id, decided_at, expires_at; không lưu permit/key rõ |
| execution_receipts | id, change_set_id FK, operation_id UNIQUE, payload_hash, status, before_state_json nullable, after_state_json nullable, validation_json nullable, observed_revision_json nullable, created_at, updated_at |
| skills | id, name, version, manifest_json, content_hash, installed_path, trust_state, created_at; UNIQUE(name, version, content_hash) |
| project_skills | project_id FK, skill_id FK, enabled, config_json, permissions_json; PK(project_id, skill_id) |
| artifacts | id, project_id FK, run_id FK nullable, kind, relative_path, sha256, size_bytes, retention_policy, created_at |
| audit_events | id, project_id FK nullable, run_id FK nullable, actor_kind, action, target_kind, target_id, outcome, safe_details_json, created_at |

Đường dẫn external chỉ được ghi bằng kết quả chọn file được người dùng cho phép, không lấy từ prompt; document record giữ phạm vi quyền đó. Document chưa lưu có path NULL và identity phiên tạm thời.
Provider config bị xóa không được làm mất lịch sử; dùng soft delete hoặc FK behavior phù hợp.
Execution receipt cho phép outcome_unknown; không ép mọi lỗi thành failed/not-applied.
Run event là nhật ký hoạt động đã làm sạch, không ghi private chain-of-thought hoặc token delta vô hạn.
Unique constraint cho provider tool calls phải xử lý namespace response/run và NULL đúng, không chặn nhầm tool call hợp lệ.

Quan hệ cốt lõi:
projects -> documents -> semantic_objects -> entity_bindings.
projects -> conversations -> messages / agent_runs.
agent_runs -> tool_calls / run_events / change_sets.
change_sets -> approvals / execution_receipts.
projects <-> skills qua project_skills.
projects -> artifacts; audit_events liên kết khi có.

Tạo index cho các FK truy vấn thường xuyên, lịch sử theo project/time, messages theo conversation/ordinal, run_events theo run/sequence, change_sets theo status/document, audit theo project/time.

Seed idempotent chỉ cho demo. Không tự chèn lại dữ liệu mẫu vào mọi project hoặc database thật.

## 14. STORAGE, PRIVACY, MIGRATION VÀ BACKUP

Ba vùng dữ liệu riêng:

```text
Thư mục dữ liệu ứng dụng của người dùng Windows
  app.sqlite
  skills/
  cache/
  logs/
  mock/
  db-backups/

Workspace được người dùng chọn
  .cad-agent/
    artifacts/
    previews/
    recovery/
  drawings hoặc các file DWG người dùng gắn vào project

Windows Credential Manager
  credential theo provider profile
```

Đường dẫn thực tế lấy qua API hệ điều hành/Tauri, không hardcode tên tài khoản.
Không lưu database đang chạy trên network share hoặc thư mục đồng bộ cloud làm mặc định.
Nếu DWG ở ngoài workspace, cần cấp quyền explicit qua thao tác chọn file; không cho model mở rộng phạm vi.
Quản lý path bằng canonicalization và kiểm tra filesystem/reparse point; không chỉ dùng string startsWith.
Không đi theo symlink/junction để thoát vùng quản lý. Export/cleanup không được xóa file DWG người dùng.
Uninstall giữ project và bản vẽ; xóa dữ liệu ứng dụng chỉ khi người dùng chọn rõ.

SQLite:
- Bật và xác minh foreign_keys cho từng connection [R11].
- Dùng WAL, busy_timeout và synchronous phù hợp cho dữ liệu lịch sử quan trọng; ghi quyết định trong tài liệu.
- Một đường ghi có điều phối; transaction ngắn, không giữ trong lúc chờ model.
- Kiểm tra SELECT sqlite_version() lúc chạy. Chọn engine có bản vá WAL-reset; tài liệu SQLite nêu bản vá ở 3.51.3 trở đi và một số backport [R12]. Không chỉ nhìn version crate SQLx.
- Backup consistent bằng cơ chế SQLite hỗ trợ hoặc checkpoint/close đúng; không copy riêng app.sqlite đang có WAL chưa xử lý [R13].
- Trước migration rủi ro tạo backup; có test migrate database cũ và restore.
- Không tự “sửa lỗi database” bằng cách xóa database và tạo trắng.

Bảo mật lưu trữ:
SQLite thường trong Foundation CHƯA mã hóa toàn bộ metadata/chat. Hiển thị và tài liệu hóa điều này; không gắn nhãn encrypted.
API key vẫn nằm riêng trong vault. Có lựa chọn không lưu nội dung hội thoại và có retention policy.
Mã hóa database toàn bộ là hạng mục mở rộng riêng, phải lựa chọn/kiểm thử engine, license, key recovery và cả backup/WAL; không tự giả lập bằng một PRAGMA không được engine hỗ trợ [R14].
Không hứa việc xóa một row sẽ xóa sạch dữ liệu trong backup hoặc mọi dấu vết trên đĩa.
Xuất diagnostics mặc định không gồm key, chat, hình học, full file path hoặc bản vẽ. Người dùng xem trước nội dung trước khi xuất.
Không có telemetry hay cloud upload ngầm.

## 15. BẢO MẬT BẮT BUỘC

Viết SECURITY.md gồm tài sản cần bảo vệ, trust boundaries, nguồn đầu vào không tin cậy, biện pháp và giới hạn.

Tài sản: API key, bản vẽ, workspace, lịch sử, quyền sửa CAD, binary updater.
Đầu vào không tin cậy: prompt, phản hồi LLM, text/metadata DWG, skill ngoài, tool output, file và dependency.
Backend/plugin là thành phần tin cậy cần audit. Không hứa bảo vệ khi OS, backend hoặc tài khoản Windows đã bị chiếm quyền.

Các yêu cầu:
- Tauri capabilities nhỏ nhất, chỉ cửa sổ cần thiết; không wildcard quyền filesystem/shell/remote content.
- Cấu hình riêng quyền custom command. Không cho rằng đăng ký invoke_handler tự tạo default-deny [R3].
- CSP production nghiêm ngặt, khớp cơ chế IPC Tauri hiện hành; không để dev origin hoặc quyền eval tùy tiện trong release.
- Không load remote website trong WebView có quyền IPC.
- Markdown được render an toàn; không HTML tùy ý, script, iframe, remote image tự tải hoặc URL javascript/file.
- Chỉ mở liên kết hợp lệ bằng hành động người dùng qua cơ chế cho phép rõ ràng.
- Mọi path/ID được kiểm tra lại ở Rust và ranh giới bridge, không tin validation frontend.
- Bounded tool output, payload, queue, entity query và số operations; mặc định tối đa 1.000 operations/change set.
- Bí mật bị loại khỏi structured log theo allowlist dữ liệu được ghi; không dựa riêng vào regex che khóa.
- Không upload DWG hoặc ảnh screenshot sang provider nếu chưa có quyền theo project.
- Nội dung DWG/skill chỉ là dữ liệu ngữ cảnh, không được biến thành system instruction có quyền cao hơn.
- Không chạy code từ skill hoặc model trong Foundation.
- Supply-chain: lockfile, dependency audit, secret scanning, license review, checksum artifact.
- Driver/debug IPC phục vụ kiểm thử không được đóng gói hoặc bật trong bản release.
- Không tắt bảo vệ Windows, antivirus, TLS, AutoCAD secure loading hoặc kiểm tra chữ ký để “làm cho chạy”.

Kiểm soát prompt injection phải thể hiện bằng policy và tests, không chỉ viết một câu “hãy bỏ qua lệnh độc hại” trong system prompt.

## 16. SKILL ENGINE CỦA ỨNG DỤNG

Đây là định dạng skill riêng được đề xuất cho sản phẩm, không tự nhận tương thích hoàn toàn với mọi nền tảng skill khác.

SKILL.md có frontmatter name/description và workflow ngắn. manifest.json quản lý:
name, version, description, entrypoint, input_schema,
required_tools, requested_permissions, supported_document_modes,
minimum_bridge_capabilities, resources, content_hash.

Base chỉ cài skill từ thư mục được người dùng chọn hoặc bundled skill; giới hạn file type, kích thước và path.
Không có ZIP marketplace, downloader hoặc executable scripts trong Foundation.
Checksum phát hiện thay đổi, không chứng minh tác giả đáng tin. Trust state và quyền là quyết định của người dùng/app.
Skill update làm thay đổi hash/quyền phải yêu cầu duyệt lại; project pin phiên bản/hash.
Không cho skill tự thay manifest permissions sau khi đã được duyệt.

Progressive loading:
- Khi khởi động, đọc metadata.
- Khi phù hợp mới đọc SKILL.md.
- Chỉ đọc reference thực sự được workflow cần.
- Không nhét toàn bộ thư viện vào mọi prompt.

Một skill mẫu mounting-plate-2d:
- Nhận schema tham số của phần 10.
- Đọc đơn vị/context.
- Tạo hoặc sửa SemanticObject.
- Gọi planner hình học xác định.
- Chạy validator.
- Đưa phương án chờ người duyệt.
- Đọc lại và kiểm tra sau thực thi.

Cung cấp reference quy ước layer và test cases JSON. Không tạo template.dwt giả hoặc file mẫu rỗng chỉ để có tên trong thư mục.
UI có danh sách skill, chi tiết phiên bản/quyền, bật/tắt theo project và trạng thái thiếu tool.

## 17. KIỂM THỬ VÀ TIÊU CHÍ NGHIỆM THU

### Unit / contract / database

- Geometry: tham số hợp lệ và không hợp lệ; resize giữ offset; tolerance và chuyển đổi đơn vị.
- Contract: round trip Rust/TypeScript/C#, schema version, unknown fields, Unicode, oversized payload.
- Policy: Ask/Plan không thể sửa; model không thể approve hoặc lấy secret.
- Approval: hash/document/session/revision mismatch, hết hạn, replay, double click.
- Storage: migrate mới/cũ, FK, cross-project validation, uniqueness, seed không lặp, backup/restore.
- Recovery: các điểm crash trước/sau commit/receipt; undo và reopen ngoài luồng agent.
- SecretStore: lưu/xóa, không trả khóa, lỗi vault; kiểm tra log không lộ credential.
- Provider: HTTP fixture cho streaming, tool-call fragments, invalid JSON, 401, 429, timeout/cancel.
- MCP: negotiation, list/call tool, malformed frame, disconnect, shutdown, không lẫn log vào stdout.

### Desktop end-to-end

Dùng desktop app thật và Rust backend thật cho ít nhất một bài test Windows qua phương thức được Tauri hỗ trợ [R15].
Test browser có mock invoke chỉ là test frontend, không được gọi là native end-to-end.
WebDriver/automation endpoint chỉ ở bản test; release không chứa khả năng điều khiển kiểm thử.

Bài nghiệm thu:
1. Cài/chạy demo không cần AutoCAD hoặc API key.
2. Tạo project, đóng/mở app, dữ liệu vẫn còn.
3. Nhập prompt tấm đế; thấy plan và preview; mô phỏng chưa thay đổi.
4. Reject: không có entity mới.
5. Approve: tạo đúng một polyline, bốn circle và dimension theo contract; validator đọc lại và pass.
6. Gửi lại cùng operation: không thêm hình.
7. Resize lên 240: sửa đúng đối tượng cũ; giữ offset/radius/height.
8. Revert khi chưa có thay đổi ngoài: khôi phục state trước resize.
9. Thay revision sau preview: Apply bị từ chối.
10. Giả lập commit xong mất kết nối: không tự vẽ lần hai; recovery đối chiếu.
11. Prompt/skill độc hại đòi đọc key, chạy shell, bỏ qua duyệt: bị chặn.
12. Chuyển provider thật khi thiếu key: báo cấu hình thiếu, không fake response thành công.

### AutoCAD thật

Chạy smoke test trên document thử mới, không dùng file gốc quan trọng.
Kiểm tra load plugin, handshake, read context, approve create, query/validate, resize, conflict, revert và disconnect.
Ghi release/build AutoCAD, runtime .NET, bridge version, kết quả và bước tái hiện.
Nếu không có môi trường: để NOT RUN — AUTOCAD REQUIRED. Mock pass không được thay trạng thái này.

Không bịa tỷ lệ pass, benchmark, coverage hoặc screenshot. Báo rõ test nào đã chạy, skipped, failed và vì sao.

## 18. BUILD, CI VÀ INSTALLER

Tạo script hoạt động thực, tài liệu hóa prerequisite và thư mục chạy:
- pnpm install --frozen-lockfile
- pnpm dev                 # desktop + backend + mock host; không chỉ web page
- pnpm dev:web             # tùy chọn, frontend test/demo giới hạn rõ
- pnpm lint
- pnpm typecheck
- pnpm test
- pnpm test:desktop
- pnpm build
- pnpm package:windows

Các script trên do dự án phải định nghĩa, không giả định có sẵn.
Rust cần chạy fmt check, clippy, test --locked cho workspace.
Nếu dùng SQLx checked macros, quản lý metadata/offline build đúng; fresh clone không cần một database bí mật của máy tác giả.

CI:
- Unit/integration frontend/backend không cần AutoCAD hoặc API key.
- Windows desktop smoke/E2E với môi trường tương ứng.
- C# contracts/transport tests độc lập; bridge thật là job tùy chọn có SDK hợp lệ.
- Không lưu Autodesk binaries vào public artifact.
- Dependency audit/secret scan/license check; mô tả ngoại lệ thay vì tắt kiểm tra.
- Test không gọi provider tính phí khi chưa được bật rõ.
- Không đưa khóa ký, credential hoặc .env thật vào repository.

Installer:
- Đóng gói đúng sidecar và kiểm tra binary có tồn tại lúc chạy.
- Tách app executable, sidecar và AutoCAD plugin DLL dù người dùng cài bằng một bộ cài .exe.
- Phát hiện prerequisite/runtime, không tự thay AutoCAD.
- Development có thể unsigned và phải ghi rõ. Release cần quy trình ký/phát hành được cấu hình bằng secret an toàn [R16].
- Không bật auto-update trước khi có verification/chữ ký và nguồn update đáng tin.
- Không ghi vào project hoặc xóa bản vẽ khi upgrade/uninstall.

Nếu môi trường hiện tại không thể build installer Windows, vẫn tạo cấu hình/script; ghi NOT BUILT ON WINDOWS thay vì đặt tên một file giả là .exe.

## 19. TÀI LIỆU VÀ ĐẦU RA PHẢI CÓ

- README.md: mục tiêu, prerequisite, chạy demo, chạy live, build và troubleshooting.
- docs/ARCHITECTURE.md: ranh giới FE/BE/MCP/plugin, ownership và luồng thay đổi.
- docs/DATABASE.md: schema, quan hệ, migration, retention và backup.
- docs/API_CONTRACTS.md: commands/events, errors, versioning.
- docs/SKILLS.md: format, quyền, ví dụ và test.
- docs/COMPATIBILITY.md: release/build AutoCAD, .NET, SDK và trạng thái thử.
- SECURITY.md: threat model, giới hạn bảo mật và cách báo lỗi.
- docs/TESTING.md: lệnh test và phân biệt mock/native/AutoCAD thật.
- docs/RECOVERY.md: outcome_unknown, reopen, undo và các trường hợp cần người dùng xử lý.
- IMPLEMENTATION_STATUS.md: IMPLEMENTED / TESTED / NOT RUN / DEFERRED cho từng capability.
- HANDOFF.md: trạng thái gần nhất, lỗi, việc tiếp theo.

Các nguồn tài liệu phải có URL thật và ngày kiểm tra; không dùng citation ID nội bộ của một cuộc chat làm tài liệu repository.

## 20. THỨ TỰ TRIỂN KHAI

M0 — Khởi tạo:
Toolchain, workspace, contract cơ bản, docs kiến trúc, threat model, UI shell.
Hoàn thành khi desktop mở và gọi Rust health command thật.

M1 — Dữ liệu và provider:
Migration, repositories, projects, conversations, SecretStore, settings, provider abstraction.
Hoàn thành khi dữ liệu bền vững, mock stream và secret tests chạy.

M2 — Luồng demo xuyên suốt:
MCP thật, MockCadGateway, geometry planner, preview, approval, apply, readback, resize, revert.
Hoàn thành khi có test tự động luồng tấm đế và chặn ghi trước approval.

M3 — Agent, skill và hardening:
Live OpenAI adapter, skill engine, giới hạn run, stale checks, idempotency, recovery, security tests.
Hoàn thành khi lỗi không bị báo thành công và những đường vượt quyền đã có kiểm thử.

M4 — Bridge và packaging:
Source C# đầy đủ trong phạm vi hỗ trợ, build profile theo SDK, native smoke tests nếu có môi trường, cấu hình installer và docs.
Hoàn thành Foundation với báo cáo trung thực; AutoCAD live chưa thử phải giữ nhãn chưa kiểm chứng, không gọi đó là xong tích hợp thật.

Không viết toàn bộ UI trước backend. Mỗi mốc nên có một vertical slice hoạt động.
Không chuyển sang cloud/multi-agent trước khi các mốc Foundation có kết quả nghiệm thu.
Nếu chỉ hoàn thành được một phần trong lần làm hiện tại, giữ repository ở trạng thái chạy được, ghi phần còn lại chính xác và không che placeholder.

## 21. CÁCH BÁO CÁO CUỐI MỖI MỐC

Báo cáo ngắn theo:
- Đã xây và đường đi thực sự hoạt động.
- File/thư mục chính đã thay đổi.
- Lệnh đã chạy và kết quả thực.
- Tính năng mô phỏng, tính năng live và phần chưa kiểm thử.
- Rủi ro/lỗi còn lại.
- Lệnh tiếp theo để người dùng chạy.

Không dùng “hoàn thành 100%”, “an toàn tuyệt đối” hoặc “production-ready” khi chưa có bằng chứng.

BẮT ĐẦU THỰC HIỆN: kiểm tra workspace/toolchain, tạo kế hoạch mốc M0–M4, rồi xây M0. Sau mỗi mốc, chạy kiểm tra và tiếp tục theo thứ tự khi môi trường cho phép. Đừng chỉ trả lại một bản mô tả kiến trúc khác.

## 22. NGUỒN CHÍNH THỨC ĐỂ AI TRIỂN KHAI KIỂM TRA LẠI

Các liên kết dưới đây là tài liệu tham chiếu, không phải yêu cầu sử dụng mọi tính năng nêu trong tài liệu. Thông số động phải được kiểm tra lại khi build.

[R1] Rust Book — Unsafe Rust:
`https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html`

[R2] Tauri 2 — Security:
`https://v2.tauri.app/security/`

[R3] Tauri 2 — Capabilities:
`https://v2.tauri.app/security/capabilities/`

[R4] Autodesk — Managed .NET Compatibility:
`https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6C680F2-DE2E-418A-A182-E4884073338A.htm`

[R5] MCP official Rust SDK:
`https://github.com/modelcontextprotocol/rust-sdk`

[R6] Autodesk — Lock and Unlock a Document:
`https://help.autodesk.com/cloudhelp/2026/ENU/OARX-DevGuide-Managed/files/GUID-A2CD7540-69C5-4085-BCE8-2A8ACE16BFDD.htm`

[R7] Autodesk Developer Blog — Running Network I/O in AutoCAD .NET Commands:
`https://blog.autodesk.io/running-network-io-in-autocad-net-commands/`

[R8] OpenAI — Function calling, Responses API:
`https://developers.openai.com/api/docs/guides/function-calling?api-mode=responses`

[R9] Microsoft — CredWriteW:
`https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew`

[R10] Microsoft — PipeOptions / CurrentUserOnly:
`https://learn.microsoft.com/en-us/dotnet/api/system.io.pipes.pipeoptions`

[R11] SQLite — Foreign Keys:
`https://sqlite.org/foreignkeys.html`

[R12] SQLite — WAL, including the WAL-reset fix:
`https://sqlite.org/wal.html`

[R13] SQLite — Backup:
`https://sqlite.org/backup.html`

[R14] SQLite — Encryption Extension:
`https://sqlite.org/see/doc/trunk/www/readme.wiki`

[R15] Tauri — WebDriver:
`https://v2.tauri.app/develop/tests/webdriver/`

[R16] Tauri — Windows Code Signing:
`https://v2.tauri.app/distribute/sign/windows/`

