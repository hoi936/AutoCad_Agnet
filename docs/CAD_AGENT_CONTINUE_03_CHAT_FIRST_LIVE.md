# CAD AGENT STUDIO - CONTINUATION 03
# CHAT-FIRST, LLM THẬT, AUTOCAD THẬT

> Ngày 07/10/2026. Prompt giao việc cho AI lập trình có quyền sửa repository và chạy terminal.
> Đây là đặc tả triển khai, KHÔNG phải mã nguồn, bộ skill đã cài hay chứng nhận live integration.
> Giữ nền Rust/Tauri hiện có; thay trải nghiệm sản phẩm, mở rộng runtime và kết nối thật.
> Tài liệu này tự chứa các yêu cầu chính, kế thừa các contract an toàn từ Continuation 02.

## 0. QUYẾT ĐỊNH SẢN PHẨM VÀ THỨ TỰ ƯU TIÊN

Tiếp tục repository CAD Agent Studio hiện tại. KHÔNG dựng lại scaffold, đổi stack, xóa database hoặc bỏ tests cũ.

Mục tiêu: ứng dụng Windows có giao diện chính là CHAT. Người dùng kết nối quyền LLM của chính mình, chọn bản vẽ AutoCAD thật và giao việc bằng ngôn ngữ tự nhiên. LLM chọn skill/công cụ, hỏi khi thiếu dữ liệu, lập phương án. Backend kiểm tra, xin duyệt, thực hiện qua MCP/plugin, đọc lại và xác minh.

LIVE là trải nghiệm sản phẩm mặc định. Chưa kết nối thì hiện onboarding/trạng thái thiếu kết nối, KHÔNG tự chuyển sang mock. Giữ MockProvider/MockCadGateway và tấm đế cho test/regression và chế độ Demo do user chọn rõ. Không chỉ xóa nhãn 'mô phỏng' trong khi backend vẫn mock.

Không xây lại CAD editor. AutoCAD là cửa sổ vẽ và nơi quản lý bản vẽ thật. App là khung chat, điều phối và kiểm soát thực thi; KHÔNG phải proxy làm mọi lệnh LLM gửi xuống mà không kiểm tra.

Phạm vi: vẽ/sửa 2D qua công cụ có schema, mở rộng bằng skill, có skill phác thảo mặt bằng nhà. Không giới hạn chat vào hai câu tấm đế, nhưng cũng không hứa dùng mọi lệnh AutoCAD hay tự chứng nhận thiết kế thi công.

Đọc HANDOFF/IMPLEMENTATION_STATUS ngày 07/10/2026 và source trước. Prompt 03 ưu tiên về phạm vi sản phẩm; 02/01/master là lịch sử và tham chiếu. Bỏ giới hạn cũ để làm OAuth/bridge, KHÔNG bỏ approval, vault, readback, revision hay recovery.

Kết nối công cụ ở đây là MCP (Model Context Protocol), không phải MVC. Không yêu cầu mật khẩu/API key/token trong chat lập trình; user cấp quyền tại app và trang chính thức.

## 1. ĐIỂM XUẤT PHÁT 07/10/2026 - KHÔNG LÀM LẠI T0

Theo bàn giao user, chưa phải kết quả bạn tự kiểm chứng:

| Hạng mục | Trạng thái được bàn giao |
|---|---|
| Windows prerequisites/doctor | Native pass; không còn blocker build desktop |
| Tauri debug app/sidecar | Build/startup native pass; chưa native UI/IPC E2E |
| Rust/MCP/vault | 35 + 4 + 1 = 40 test Windows; không cộng cached Linux |
| Frontend | 9 tests và typecheck/lint/build pass |
| Windows vault | Fixture roundtrip/cleanup pass; chưa credential/inference thật |
| AutoCAD | Phát hiện 2026.1.2 / R25.1.179.0.0; chưa plugin/handshake |
| .NET SDK | 10.0.300 sẵn có theo bàn giao |
| OAuth/global accounts/skill runtime | Chưa triển khai; API-key adapter cũ có fixture |
| C# bridge/live LLM+CAD/NSIS | Chưa nghiệm thu; chưa bridge DLL/bộ cài |

Repository Windows: `D:\PHAN DINH HOI\AutoCad_agnet`.
Evidence cũ: `docs/evidence/continue02/`.
App đang mở là debug copy trong `target/native-smoke/`, thư mục Unicode 'Bản chạy thử'. Sửa source KHÔNG tự cập nhật bản copy đang mở. Mỗi bản mới phải có build ID và đường chạy rõ; không tự đóng app/DWG của user.

Tái sử dụng crates/model-provider, crates/cad-tool-host, app-core agent.rs/provider_service.rs/gateway_integration.rs, contracts, migrations 0001-0003, ChatPanel/ProviderSettings/ipc, Tauri permissions và scripts Windows. Đọc source để xác định đúng path; không tạo module trùng do đoán tên.

Giữ dependencies Windows/WSL tách biệt, helper PATH/PATHEXT cho process con, frozen lockfiles và rebuild sidecar thường sau fault-injection tests. Không hỏi lại quyền cài prerequisite đã được cấp; quyền đó KHÔNG thay thế consent login, cài plugin, pairing hoặc ghi DWG.

Hai khối handoff bị dán trùng không là hai lần test. Lỗi thiếu toolchain ở báo cáo 05/10 là lịch sử, không được dùng làm blocker hiện tại nếu không có lỗi mới.

## 2. KIẾN TRÚC GIỮ NGUYÊN, PHẠM VI BỔ SUNG

- React + TypeScript: giao diện.
- Tauri 2: host desktop Windows.
- Rust App Core: agent, provider, skill registry, policy, approval, persistence.
- Rust cad-tool-host: MCP sidecar, lựa chọn CAD gateway, quản lý transport.
- C#/.NET: plugin bên trong AutoCAD đầy đủ trên Windows.
- SQLite: dữ liệu ứng dụng, migration versioned.
- Windows Credential Manager: API key; OAuth token vault có thiết kế an toàn ở mục 6.

Đường chạy mục tiêu:

```text
React UI
  -> Tauri commands/events
  -> Rust App Core
       -> ModelProvider: Mock / OpenAI API key / ChatGPT OAuth
       -> SkillRegistry + GeometryPlanner + Policy + Approval
       -> SQLite repositories
       -> MCP client
            -> private stdio
            -> cad-tool-host
                 -> MockCadGateway
                 OR
                 -> AutoCadGateway -> local named pipe -> C# plugin -> DWG database
```

Không để frontend gọi provider trực tiếp. Không để provider cloud kết nối vào stdio hoặc named pipe trên máy. MCP host không được nhận API key/OAuth token. SQLite ứng dụng chỉ do App Core ghi; không cho plugin C# cùng mở database ứng dụng.

Ngoài phạm vi đợt này: cloud sync, marketplace công khai, arbitrary code execution, tự động điều khiển toàn bộ công cụ AutoCAD, BIM/MEP/3D đầy đủ, mô-đun tính toán kỹ thuật để tự chứng nhận thiết kế. Xây nền mở rộng nhưng không hứa các khả năng này đã có.

LLM thật đề xuất lời gọi công cụ, Rust thực thi local và gửi kết quả lại; không cần cloud kết nối trực tiếp named pipe [S3, N1]. Không thêm auth server/user-password sản phẩm, PostgreSQL, Redis hay multi-agent cho lát cắt này. Không thay AgentService Rust bằng runtime khác chỉ để có login.


## 3. PHÂN BIỆT TRẠNG THÁI, KHÔNG GỘP THÀNH MỘT NÚT CONNECTED

Mỗi phiên phải thể hiện độc lập:

| Thành phần | Trạng thái cần phân biệt |
|---|---|
| Runtime | Web preview / Desktop native |
| LLM | Mock / API key / ChatGPT OAuth / chưa cấu hình / lỗi |
| Provider authorization | Đã xác thực / thiếu quyền inference / hết hạn / bị thu hồi |
| MCP | Chưa khởi chạy / đã handshake / lỗi transport |
| CAD | Mock / AutoCAD chưa phát hiện / plugin chưa nạp / chưa ghép cặp / read-only / write-ready |
| Document | Chưa chọn / đúng tài liệu / đã đổi / busy / đã đóng |
| Skill | Enabled / disabled / thiếu tool / chưa tin cậy / lỗi tương thích |

Web preview không được âm thầm dùng localStorage thay backend. Đường web cho phép xem UI và dữ liệu mẫu có nhãn, nhưng không nhận hoặc lưu credential thật và không giả lập trạng thái native đã kết nối.

Có bốn tổ hợp kiểm thử độc lập:
1. Mock LLM + Mock CAD: kiểm thử offline.
2. Live LLM + Mock CAD: chứng minh hiểu prompt và gọi tool thật qua provider.
3. Mock LLM hoặc fixture có kiểm soát + Live AutoCAD: chứng minh bridge thật, không cần trả phí LLM.
4. Live LLM + Live AutoCAD: nghiệm thu tích hợp cuối.

Kết quả ở một tổ hợp không được thay thế bằng chứng của tổ hợp khác. Việc chuyển từ mock sang live CAD phải do người dùng chọn rõ, vô hiệu hóa change-set/approval cũ và ghép lại document/semantic mappings; không replay demo sang DWG.

## 4. R0 - AUDIT NGẮN RỒI VIẾT TÍNH NĂNG

Kiểm tra repository, mã và evidence mới. Chỉ chạy doctor để xác nhận môi trường; không làm lại đợt T0/setup khi không có lỗi mới.

Tạo docs/CONTINUE_03_DELTA.md: capability -> code/UI hiện có -> tái sử dụng -> bổ sung -> test -> mốc R1-R5. Kiểm tra baseline phù hợp. Không xóa test, lockfile, project, chat hay DB. Nếu không có Git hợp lệ, không báo commit/PR giả; không public source mà chưa có phép.

Xác định phần chưa có mã so với đã có mã/chưa test. Mỗi thay đổi cần đường UI -> IPC -> service -> provider/DB/gateway thật. Bắt đầu R1 ngay sau audit; không bàn giao chỉ một kế hoạch mới.

## 5. FRONTEND CHAT-FIRST VÀ PREVIEW ĐÚNG NGỮ CẢNH

### 5.1 Trải nghiệm chính

ChatWorkspace: conversations bên trái, chat ở giữa, panel ngữ cảnh/thay đổi có thể đóng bên phải. AutoCAD giữ cửa sổ vẽ riêng; không nhúng AutoCAD vào WebView hoặc xây CAD canvas riêng.

Header hiện account/model, CAD session/document, skill và quyền. Navigation: Chat / Kết nối / Skills / Lịch sử / Chẩn đoán. Settings/login truy cập được khi chưa có project. Chat Ask có thể làm rõ brief trước khi attach drawing, nhưng không giả vờ đã đọc CAD.

Bỏ form tấm đế khỏi workspace mặc định; chuyển vào Examples/Developer Demo. Prompt tự do là lối vào chính. Form tham số chỉ xuất hiện theo input schema của skill đang dùng.

Continue with ChatGPT là lối kết nối chính khi đủ điều kiện; API key là lựa chọn riêng, hiện rõ billing. Chưa có LLM/bridge thì hiện hướng dẫn và trạng thái đúng, không tự khởi động mock.

Preview vẫn cần, nhưng là thẻ phương án, không phải bằng chứng đã ghi DWG. Render từ cùng immutable ChangeSet backend sẽ thực hiện. Tiêu đề app/nhãn mock/live lấy từ runtime state, không hardcode.

### 5.2 Lỗi ngữ cảnh cần kiểm tra

Ảnh user cho thấy yêu cầu 260/300 mm bị mock báo không hỗ trợ, nhưng panel vẫn có preview 204 mm và nút duyệt. CHƯA kết luận 204 là output của run mới: có thể là proposal cũ/từ form. Phải kiểm tra provenance.

Mỗi proposal/preview gắn conversation_id, run_id, source_message_id, source_kind (chat/form), change_set_id, payload_hash, CAD session/document/revision. Hiện rõ yêu cầu nào sinh phương án nào.

Run không hỗ trợ hoặc không sinh proposal KHÔNG được nhận preview từ run cũ. Giới hạn một active pending proposal trên cùng conversation/document; bản bị thay thế chuyển superseded. Mở proposal lịch sử phải revalidate/replan trước duyệt, không dùng permit cũ.

Sửa text nháp chưa gửi không đổi payload preview. Gửi yêu cầu sửa mới, đổi document/account/skill phải invalidation theo policy. Kiểm tra stale event/late stream đến sau khi đổi context, chặn duyệt nhầm ở cả UI và backend.

### 5.3 Trạng thái thực thi

Phân biệt answering, needs_input, planning, awaiting_approval, applying, verifying, completed, unsupported, failed, outcome_unknown. Kết thúc một turn KHÔNG đồng nghĩa đã vẽ. Hiện tool activity/kế hoạch tóm tắt, không private chain-of-thought. Dừng không đồng nghĩa Undo. Applied in memory khác saved to disk.

## 6. AUTH, LLM THẬT VÀ VAULT

ƯU TIÊN MỚI: OAuth với quyền dùng gói ChatGPT là hạng mục chính, không đẩy sang backlog chỉ vì API-key adapter dễ hơn. API key là lựa chọn riêng và reuse adapter cũ. Eligibility, scope, model và hạn mức phải được xác minh, không giả OAuth.

Phân biệt token xác thực OAuth với token input/output đo usage. Hiện usage provider trả về hoặc estimate có nhãn; không bịa số dư hạn mức tài khoản từ bộ đếm local. Hết quyền/hạn mức thì báo và dừng; không tự đổi account/billing route.


### 6.1 Phân biệt ba loại đăng nhập

Tài khoản sản phẩm CAD Agent, quyền gọi LLM và tài khoản/giấy phép Autodesk là ba việc khác nhau. Đợt này KHÔNG cần backend user/password/JWT riêng cho sản phẩm. AutoCAD tự quản lý đăng nhập/giấy phép của nó.

API key OpenAI và quyền sử dụng gói ChatGPT qua OAuth là hai chế độ provider khác nhau. API billing và ChatGPT billing tách biệt [S2]. Không tự chuyển sang API key có thể phát sinh phí khi OAuth hết hạn mức; hỏi người dùng trước khi đổi chế độ.

### 6.2 OpenAI API key — bắt buộc triển khai thật

Dùng lại adapter Responses API nếu đã tồn tại; kiểm tra call path và serializer thay vì viết adapter trùng. Model ID cấu hình được, catalog theo khả năng endpoint và account, không hardcode một model chưa xác minh.

API key do người dùng nhập tại desktop. Rust lưu vault; frontend xóa form/state sau xử lý. Backend không có command trả plaintext secret. UI chỉ đọc trạng thái configured, account label và thông tin không bí mật.

Xử lý streaming text/tool arguments, nhiều tool calls, call ID, gửi lại tool outputs và continuation items theo API/model [S3]. Argument phải hoàn chỉnh và được validate schema/nghiệp vụ trước khi dispatch. Không parse từng token thành thao tác vẽ.

Có HTTP fixture tests cho 401/403/429/5xx, stream ngắt, malformed JSON, unknown tool, cancellation, vòng lặp và retry có giới hạn. Không có API key không cản việc triển khai adapter và fixture tests; chỉ đánh dấu live smoke NOT RUN.

### 6.3 Continue with ChatGPT — một hạng mục thật, không phải nút trang trí

Kiểm tra tài liệu chính thức tại thời điểm triển khai và ghi kết quả vào docs/AUTH_COMPATIBILITY.md, gồm loại ứng dụng, điều kiện phân phối, flow được hỗ trợ và ngày kiểm tra.

Ở thời điểm soạn, OpenAI có tài liệu cho quyền dùng gói ChatGPT trong ứng dụng nguồn mở/chạy cục bộ; ứng dụng trả phí/host từ xa có điều kiện riêng [S4]. Không mặc định mọi gói, workspace hoặc model đều được dùng. Không công khai repository hay thay license chỉ để đáp ứng điều kiện mà chưa có phép của chủ dự án.

Triển khai adapter OAuth khi loại ứng dụng hiện tại được hỗ trợ. Nếu provider yêu cầu quyền/đăng ký bên ngoài chưa có, vẫn triển khai phần client, UI states và tests có thể làm; ghi BLOCKED_EXTERNAL chính xác cho bước live, không đánh dấu hoàn thành OAuth. Ngược lại, không bịa yêu cầu xin client secret nếu flow chính thức không cần.

Đọc và thực hiện flow native từ [S5], dùng thư viện OAuth/OIDC đã được kiểm tra:
- Mở trình duyệt hệ thống, không đưa trang đăng nhập vào WebView có Tauri privileges [S9].
- Giữ callback, state, nonce và PKCE riêng cho từng lần đăng nhập; callback chỉ được chấp nhận một lần.
- Theo flow đăng ký động được tài liệu cho phép: dùng entrypoint client ban đầu đúng mục đích; lưu issued client_id của registration, không dùng entrypoint thay cho issued client_id khi exchange.
- Giữ host identity ổn định, không chứa tên máy/tên người dùng, theo hướng dẫn chính thức.
- Kiểm tra chữ ký và các ràng buộc OIDC, identity/workspace và quyền inference trước khi chuyển active account.

Không tự đoán authorization/token endpoint, scope, audience, redirect URI hoặc hỗ trợ device flow. Nếu dùng loopback callback, bind đúng loopback, dùng path/port hợp lệ theo nhà cung cấp, timeout, giới hạn input và đóng listener khi xong. Không mở callback trên 0.0.0.0.

Lưu registrations theo identity/workspace/registration phù hợp, không lấy email làm primary key. Pending sign-in không ghi đè active account. Có refresh an toàn, single-flight refresh, đăng xuất local, xử lý revoke/expiry và đổi tài khoản; tham chiếu hướng dẫn sessions [S6].

Tách request profile API-key và OAuth. Luồng ChatGPT plan có catalog và điều kiện request riêng: lấy model cho tài khoản đang chọn, gửi public Responses endpoint, không dùng backend-api của ChatGPT [S7]. Với flow được tài liệu [S8] mô tả, áp dụng store=false, stream=true; không gửi các field bị cấm, không dùng previous_response_id HTTP hoặc tool schema sai route. Nạp history cần thiết theo cách route hỗ trợ. Giới hạn token/tool loop phải còn được thực thi trong App Core dù API không nhận một field giới hạn.

MCP vẫn chạy local; ánh xạ tool vào cấu trúc function/custom tool mà route hỗ trợ. Không vì hosted MCP không khả dụng trên một route mà đổi sang public server hoặc lộ pipe ra Internet.

Kiểm thử OAuth: hủy đăng nhập; callback sai state/nonce; callback replay; identity không khớp; thiếu quyền inference; refresh fail; phiên hết hạn; tài khoản cùng email khác registration; đổi account; token revoke; catalog khác giữa API key/OAuth; request serializer loại đúng field không hỗ trợ.

Live OAuth chỉ PASS sau khi người dùng đăng nhập thật, chọn model hợp lệ và hoàn tất một tool-calling probe. Fixture PASS hoặc chỉ lấy được danh tính không được gọi là live inference PASS.

### 6.4 Vault và quyền dữ liệu

Không lưu credential trong SQLite, localStorage, JSON plaintext, log, prompt, tool arguments, URL, crash reports hay environment của sidecar. Không đọc cookie browser hoặc credential của ứng dụng khác. Không giả danh client của một sản phẩm bên thứ ba.

Giữ Windows Credential Manager cho API key. Kiểm tra giới hạn blob của WinCred; không mặc định có thể nhét mọi bộ access/refresh/ID token vào một credential [S10]. Nếu token bundle vượt giới hạn hoặc cần atomic rotation, triển khai OAuthTokenVault riêng dùng DPAPI current-user, file mã hóa trong vùng dữ liệu ứng dụng với ACL chặt và atomic replace [S11]. Đây là cơ chế được tài liệu hóa, không phải fallback plaintext. Không dùng machine-wide protection mặc định.

Chỉ lưu secret_ref và metadata an toàn trong SQLite. Vault write fail phải được trả lỗi; rollback metadata cho nhất quán. Có test secret lớn, Unicode, restart, delete, refresh rotation và lỗi bảo vệ dữ liệu. Không claim vault chống được malware đã chiếm cùng user hoặc backend.

Provider cloud cần consent theo project/tài khoản/phạm vi dữ liệu; cho xem loại dữ liệu sẽ gửi. Không upload DWG/screenshot tự động. Không quảng cáo zero-retention chỉ bằng một request flag. Không chuyển sang mock khi provider thật lỗi.

Mô hình khác chỉ thêm qua ModelProvider + capability profile riêng. Chưa triển khai provider nào thì UI ghi chưa hỗ trợ; không dựng nút đăng nhập Claude/Gemini/local rồi dùng cùng logic OpenAI để giả tương thích.

### 6.5 Đăng xuất và phân quyền usage

Thực hiện sign-out theo tài liệu sessions hiện hành [S6]: dừng request mới, thử revoke phiên renewable qua endpoint chính thức, rồi xóa token local. Nếu chưa xác nhận revoke remote, báo rõ thay vì tuyên bố đã thu hồi trên server. Giữ mapping registration/client và host ID khi phù hợp; không giữ access/refresh/ID token sau sign-out. Không chia sẻ credential giữa các registration cùng email.

## 7. SKILL RUNTIME VÀ MIỀN THIẾT KẾ NHÀ

Skill là gói quy trình, schema, quy ước và kiểm tra cho agent, không phải việc huấn luyện lại trọng số LLM. Viết SKILL.md nhưng không có loader/registry/router/runtime thì chưa hoàn thành hệ thống skill.

Đây là định dạng nội bộ của CAD Agent Studio, không tự nhận tương thích hoàn toàn với skill của mọi nền tảng.

### 7.1 Cấu trúc và contract

```text
skills/
  mounting-plate-2d/
    SKILL.md
    manifest.json
    schemas/input.schema.json
    references/drawing-rules.md
    tests/cases.json
  primitives-2d/
    SKILL.md
    manifest.json
    schemas/input.schema.json
    references/drawing-rules.md
    tests/cases.json
  drawing-audit-2d/
    SKILL.md
    manifest.json
    schemas/input.schema.json
    references/checks.md
    tests/cases.json
```

SKILL.md: frontmatter name/description và hướng dẫn ngắn, quy trình, điều kiện hỏi lại, kết quả mong đợi. Metadata giúp nhận diện khi cần nạp; không nhét toàn bộ thư viện vào mọi lượt.

Manifest tối thiểu: schema_version, name, version, display_name, entrypoint, input_schema, required_tools, requested_permissions, supported_document_modes, minimum_bridge_capabilities, references. Tên trường là contract của dự án; map sang schema hiện có khi hợp lý.

Registry tính content_hash của package theo định nghĩa canonical rõ ràng, không dùng hash tự tham chiếu của chính manifest. Lưu/pin version/hash theo project. Hash phát hiện thay đổi, không chứng minh tác giả đáng tin.

### 7.2 Các module phải có

SkillRegistry: scan bundled và thư mục do người dùng chọn, validate, list, enable/disable theo project, kiểm tra dependency/capability.

SkillRouter: explicit selection/slash command và lựa chọn theo yêu cầu; không chỉ if prompt chứa một từ thì chạy fixture. Model có thể đề xuất skill, backend xác minh skill được bật và có quyền.

SkillLoader: chỉ đưa metadata ban đầu; khi chọn mới đọc SKILL.md, schema và reference cần thiết. Giới hạn kích thước, số file, token và depth.

SkillExecutor: phối hợp tool đọc, planner, preview/approval hiện có và verifier. Chỉ registry/backend định nghĩa quyền. Không có SKILL.md nào được tự cấp quyền hoặc thay policy.

Skill provenance: mỗi run ghi skill_id/version/hash, tool/schema version và kết quả validator. Khi file skill đổi giữa run, giữ snapshot đã pin hoặc hủy/replan; không đổi hướng dẫn âm thầm giữa preview và apply.

### 7.3 Skill nghiệp vụ mới; tấm đế chỉ là regression

Các gói dưới là yêu cầu CẦN TRIỂN KHAI trong repository, không phải skill bên ngoài đã cài. Giữ tên/module cũ khi chức năng tương đương, không đổi tên làm mất binding.

| Skill | Input | Output và kiểm tra |
|---|---|---|
| autocad-operator | Yêu cầu, document, selection | Đọc ngữ cảnh, đơn vị, chọn skill/tool hợp lệ; không raw command |
| primitives-2d | Line/circle/rectangle/polyline/arc; layer/text/dimension có schema | Hình học tham số đúng; rectangle là polyline kín, dimension thật |
| modify-selection-2d | Selection thật hoặc semantic object; move/rotate/sửa thuộc tính | Sửa đúng đối tượng, không tạo chồng; conflict khi type/fingerprint đổi |
| floorplan-concept-2d | Brief nhà, footprint, phòng, tường, cửa, đơn vị | Design graph, phương án 2D, entity CAD và báo cáo kiểm tra |
| drawing-audit-2d | DWG/selection/nhóm được quản lý | Kiểm tra read-only; báo giới hạn, không tự sửa |
| mounting-plate-2d | Tham số tấm đế hợp lệ | Giữ regression create/resize/revert; không định nghĩa toàn bộ app |

Mở rộng cấu trúc thư mục 7.1 cho các gói mới theo cùng quy ước. SKILL.md không thay thế geometry handler. Manifest yêu cầu tool chưa có thì báo thiếu capability, không gọi stub luôn thành công. Skill chọn tự động hoặc thủ công phải đều ghi provenance.

### 7.3.1 Skill nhà: brief -> layout -> hình học -> sửa tiếp

Với yêu cầu 'vẽ giúp tôi một căn nhà', hỏi ngắn về kích thước khu đất/footprint, số tầng, công năng phòng và đầu ra 2D/3D. Khi câu nói mơ hồ, hỏi lại; không tự đổi 'ba gian' thành 'ba tầng'. User trao đổi bằng chat, không cần tự viết JSON.

Phạm vi skill đầu: mặt bằng một tầng, tường thẳng vuông góc, footprint hình chữ nhật, WCS XY. Nhiều tầng/3D là mở rộng sau khi lát cắt này đạt. Nói rõ phạm vi chưa hỗ trợ; không vẽ một tầng rồi tuyên bố đã xong ba tầng.

DesignBrief lưu riêng yêu cầu user xác nhận và giả định model đề xuất. Kích thước tổng phải rõ theo mép ngoài, tim tường hay thông thủy; không đánh tráo. Mét/mm phải chuyển đổi có metadata, không đổi INSUNITS âm thầm.

Quy trình: brief -> quan hệ phòng/vùng -> đề xuất layout -> kiểm ràng buộc -> preview -> duyệt -> tạo tường/cửa/nhãn/dimension -> readback -> audit. Đầu ra có nhãn PHÁC THẢO PHƯƠNG ÁN, không phải hồ sơ thi công đã thẩm tra.

Design graph tối thiểu:
- Building/Level: footprint, hệ tọa độ, đơn vị, level identity.
- Room: polygon, tên/công năng, diện tích đo, ràng buộc và quan hệ kề.
- Wall: tim, bề dày, vai trò và liên kết phòng.
- Opening: wall_id, offset, width, loại cửa, chiều mở.
- Annotation: liên kết đối tượng/hình học thật.

Geometry compiler độc lập LLM biến graph thành entity, xử lý nút giao tường và khoảng mở cửa. Không vẽ ký hiệu cửa lên tường kín rồi coi là cửa hợp lệ. Đây là semantic object của app tạo bằng CAD 2D, KHÔNG phải native BIM/AEC object khi chưa có toolset tương ứng.

Validator kiểm footprint, polygon kín/không tự cắt, room không chồng, area theo quy ước, bề dày tường dương, cửa trên đúng wall/không vượt đầu tường, reachability phòng qua cửa, nhãn và kích thước. Nêu rõ check chưa thực hiện. Không suy từ các check này ra tuân thủ quy chuẩn, an toàn kết cấu, MEP hay phòng cháy.

Chỉ sửa phần graph bị ảnh hưởng và các đối tượng phụ thuộc. 'Dịch cửa này 300 mm' phải đọc selection/semantic ID, giữ width, kiểm offset mới, không tạo thêm một căn nhà. Layout sửa làm dịch phòng khác phải hiện đầy đủ diff.

DWG cũ không có graph: chỉ nhận diện khi đủ bằng chứng, yêu cầu user chọn/xác nhận mapping khi mơ hồ. Read-only không tự ghi metadata để adopt bản vẽ.

### 7.4 UI Skills

Hiển thị danh sách, chi tiết hướng dẫn, version/hash, nguồn, quyền yêu cầu, tool cần thiết, tương thích gateway hiện tại, enable/disable theo project và chạy test trên mock.

Có “Tạo skill từ mẫu” để tạo gói dữ liệu/hướng dẫn nháp, validate và test trước khi bật; không cho nháp tự đăng ký code executable. Cho import từ thư mục người dùng chọn. Chưa cần ZIP marketplace hoặc downloader.

Chặn path traversal, symlink/junction thoát thư mục, reference tùy ý ra Internet và file thực thi. Render markdown an toàn; không script/HTML chủ động hoặc remote image tự tải. Skill update tăng quyền phải được duyệt lại. Test thiếu tool, sửa hash, schema sai, disabled skill và instruction injection.

## 8. AGENT RUNTIME: KHÔNG CÒN CHỈ NHẬN HAI CÂU FIXTURE

Giữ MockProvider để test; đường provider thật phải dùng output mô hình thật. Không dùng regex nhận câu mẫu để thay LLM rồi gắn nhãn live.

Vòng xử lý:
1. Snapshot project, provider account/model, mode, CAD target, policy và skill registry.
2. Lấy ngữ cảnh tối thiểu qua MCP: document/session/units/selection/capabilities.
3. Chọn skill hợp lệ; nạp instructions/schema cần thiết.
4. Model gọi tool đọc hoặc tool đề xuất trong allowlist.
5. Validate tool name, schema, số lượng, đơn vị, target IDs và preconditions.
6. GeometryPlanner tạo immutable ChangeSet, preview và validator sơ bộ.
7. Dừng ở awaiting_approval; không giữ transaction/lock trong thời gian chờ.
8. Chỉ thao tác duyệt từ UI tin cậy mới cho ApprovalService cấp permit.
9. Apply qua MCP/bridge, readback, verify và cập nhật kết quả theo sự thật.

Ask không được ghi; Plan chỉ đề xuất. Model/skill không có tool approve, lấy secret, chạy shell hoặc đổi policy. Bộ tool visible cho LLM không phải toàn bộ tool nội bộ.

Prompt mơ hồ về đơn vị, đối tượng hoặc kích thước thì hỏi lại hoặc đề xuất giả định để duyệt; không tự chọn tài liệu khác. Nhắc “làm đẹp hơn” không tự thay đổi kích thước kỹ thuật.

Có ngân sách tool calls, retry, thời gian, input/output và max entity query. Event có run_id và sequence; UI chống trùng và resync bằng snapshot khi reconnect. Không ghi một row SQLite cho từng token.

Khi người dùng bấm dừng: dừng phần chưa thực hiện; nếu mutation đã gửi phải chờ/đối chiếu trạng thái, không tuyên bố đã undo. Không tự resume tác vụ có mutation sau restart nếu chưa có quyết định phục hồi.

Ask/thu thập brief có thể chạy khi chưa attach document; bỏ qua các bước CAD cần target, không tự tạo document giả. Khi chuyển sang lập proposal có geometry/apply, bắt buộc attach target và lấy snapshot mới. Cuộc hội thoại độc lập với trạng thái AutoCAD đã kết nối.

### 8.1 Contract tổng quát, không hardcode tấm đế

Tách DesignProposal, ChangeSet và ExecutionReceipt. Proposal chứa ý định/graph; backend compile/validate thành ChangeSet bất biến; receipt mô tả thực tế đã xảy ra.

Contract tối thiểu: schema_version, proposal_id, run_id, source_message_id, document_ref, base_revision, units, coordinate_frame, assumptions, semantic_patch, operations, affected_entities, preconditions, validation_report, preview_ref, content_hash. Backend bind target/context từ phiên user đã chọn; không tin document ID LLM tự bịa. Runtime đọc units và transform theo drawing trước khi compile.

Phân biệt read tools, propose tools và internal mutation tools. LLM có thể gọi proposal cho primitives/selection/floorplan, KHÔNG có approve, vault, raw SQL, shell hay arbitrary script. Backend ánh xạ catalog vào tool schema được auth route hỗ trợ, rồi thực thi local qua MCP.

Một lần duyệt cho một change-set/hạng mục đã xác định, không duyệt từng đoạn thẳng. Replan/thay payload/mở rộng phạm vi phải duyệt lại. Auto-execute không giới hạn KHÔNG thuộc đợt này.

Giới hạn entity/batch và thời gian giữ AutoCAD thread. Nhiều batch phải có checkpoint, expected post-state, partial outcome và cách phát hiện sửa ngoài. Không hứa một transaction phủ hết mọi bước. Event dến UI gắn run/sequence, chặn trùng và stale events.


## 9. MCP VÀ BRIDGE AUTOCAD THẬT

Profile ưu tiên: AutoCAD 2026.1.2 theo handoff, .NET SDK 10 sẵn có. Chọn x64/.NET 10 phù hợp sau khi kiểm tra host assemblies; không tự quay về .NET Framework 4.8 hoặc nâng AutoCAD. Autodesk nêu nhóm Update 1.2 trở đi dùng .NET 10 [S14]. Detection pass chưa phải plugin load pass.


### 9.1 Giữ MCP sidecar, kiểm chứng đường process thật

Dùng SDK MCP Rust chính thức theo dependency đã khóa [S12]. Nếu ba MCP integration tests đã có, đọc xem chúng spawn process thật hay chỉ mock trait. Bổ sung lifecycle/timeout/crash tests khi thiếu; không viết lại server chỉ vì đổi tên.

App Core -> private stdio -> cad-tool-host. stdout chỉ protocol, stderr log đã làm sạch. Spawn binary đóng gói từ đường xác định; không từ prompt hoặc PATH tùy ý. Sidecar được đóng gói theo cơ chế external binaries của Tauri [S13].

Tool đọc tối thiểu, map sang tên hiện có nếu tương đương:
- cad.get_status
- cad.list_documents
- cad.get_document_context
- cad.get_selection
- cad.query_entities
- cad.list_layers
- cad.get_operation_status
- cad.validate_entities

Tool domain chỉ đề xuất: design.propose_primitives, design.propose_selection_edit, design.propose_floorplan; giữ tool tấm đế cho regression. Tên là contract đề xuất, phải map sang handler/schema thật.

Tool mutation nội bộ: cad.apply_change_set, cad.revert_change_set. Không đưa chúng vào catalog LLM; vẫn phải xác minh permit ở sidecar và bridge, không chỉ ẩn tên.

### 9.2 Phát hiện AutoCAD và profile build

Tạo trang Kết nối AutoCAD với: bản cài phát hiện được, release/build, đường cài đã kiểm tra, plugin version, session, document, capabilities và last error.

C# plugin mục tiêu AutoCAD đầy đủ trên Windows. Chốt MỘT profile phù hợp máy đang có trước. Build profile chứa TFM, Autodesk reference path, assembly versions và release/update đã kiểm thử.

Không chỉ suy ra runtime từ “2026”. Autodesk hiện phân biệt AutoCAD 2026 trước/sau Update 1.2 về .NET 8/.NET 10; tài liệu cập nhật cũng nêu một số plugin target .NET 8 có thể tiếp tục chạy trên host mới [S14]. Kiểm tra SDK/runtime/API thật và test, không bắt buộc dựng nhiều binary nếu không cần, không tự cập nhật/hạ AutoCAD của người dùng.

Nếu thiếu Autodesk SDK/reference, báo rõ missing dependency; vẫn build/test phần DTO/transport/validator độc lập. Không tạo assembly giả cùng namespace Autodesk để vượt compile. Không commit DLL Autodesk hoặc mặc định có quyền phân phối lại chúng. Đặt host references không copy nhầm vào bundle.

### 9.3 Bundle, cài đặt và ghép cặp

Tạo .bundle thật với PackageContents.xml và DLL build được khi có môi trường. Dùng cơ chế ApplicationPlugins và yêu cầu secure loading của Autodesk [S15].

Provide scripts build/install/uninstall bridge có dry-run và phạm vi rõ; chỉ xóa/cập nhật bundle của dự án. Đường user profile hay Program Files phải được chọn theo policy máy và trust requirements, không tự cho tất cả thư mục người dùng là trusted.

Không đặt SECURELOAD=0, không whitelist cả ổ đĩa/downloads, không khuyên tắt bảo vệ [S16]. Unsigned dev plugin phải ghi rõ; hướng dẫn phê duyệt đúng package/path khi policy cho phép. Nếu cần quyền admin cho một thao tác cài cụ thể, yêu cầu riêng; không chạy cả app và AutoCAD elevated làm mặc định.

Plugin cung cấp command do dự án định nghĩa: CADAGENTCONNECT, CADAGENTSTATUS, CADAGENTDISCONNECT. Đây không phải lệnh Autodesk có sẵn; phải thực sự đăng ký và triển khai.

Người dùng kết nối/ghép cặp từ AutoCAD; ứng dụng chọn đúng process/session/document. Có heartbeat, timeout, disconnect thu hồi quyền. Nút AutoCAD Connected chỉ bật sau khi handshake và đọc được document từ plugin, không chỉ phát hiện acad.exe.

### 9.4 Threading, lock và read-only

Tách Bridge.Protocol/Transport khỏi Bridge.AutoCAD. Listener pipe parse DTO và xếp hàng; tất cả truy cập AutoCAD API chạy trên main thread và đúng document/command context. Không giả định lock làm API thread-safe. Tham chiếu hướng dẫn Autodesk [S17, S18].

Không giữ document lock hoặc transaction khi chờ mạng, model hoặc approval. Dùng API dispatch theo SDK thật; không bịa chữ ký method. Busy document trả trạng thái có thể xử lý, không treo UI hoặc tự gửi Escape làm mất lệnh người dùng.

Read-only handshake trả đủ protocol version, plugin/runtime version, process/session identity, capability list. Document DTO trả identity, name/path được phép, active state, units, coordinate context, space, dirty state và revision token do plugin quản lý.

Read-only APIs không tự tạo layer, ghi metadata hay sửa INSUNITS. Doc chưa lưu dùng identity phiên; chỉ cấp persistent metadata khi có mutation được duyệt hoặc thao tác cấp quyền riêng.

Test live read-only trên tài liệu test không bí mật: đọc danh sách, selection, layer và hình học mà không thay dirty state/đối tượng.

## 10. TRANSPORT VÀ PERMIT Ở RANH GIỚI RUST/C#

Dùng named pipe local, DACL phù hợp user/logon session, từ chối remote client, kiểm tra peer/process/session và elevation theo threat model [S19]. Không dùng pipe name hoặc PID làm bằng chứng xác thực duy nhất; không để Everyone/Anonymous có quyền ghi.

Chọn framing có version, length prefix và giới hạn kích thước; giữ giới hạn master khi đã có. Test fragmented frames, UTF-8, oversized payload, timeout, duplicated request và protocol mismatch ở cả Rust/C#.

Pairing tạo session mới và cơ chế bootstrap quyền có tài liệu. Không đặt session secret trong command-line args, discovery file plaintext, log hoặc UI. Dùng primitive mật mã và thư viện chuẩn; không tự thiết kế thuật toán.

Đề xuất envelope khái niệm, điều chỉnh cho contract sẵn có:
protocol_version, request_id, operation_id, cad_session_id, document_id, base_revision, payload_hash, issued_at, expires_at, payload, authorization.

Hash/permit phải gắn byte payload bất biến hoặc canonical representation đã định nghĩa. Có golden vectors Rust/C# cho số thực, Unicode, thứ tự property và null. Không giả định serializer hai ngôn ngữ tạo cùng hash.

Phạm vi approval gồm project, document, session, payload hash, revision, operations và expiry. Bridge kiểm lại dưới context/lock sát thời điểm ghi. UI gửi change_set_id để duyệt; không gửi một danh sách sửa khác với preview. Double click/call lại cùng operation_id phải được đối chiếu receipt, không vẽ lại.

Mức bảo vệ local này không được quảng cáo là chống được malware đã chiếm cùng user/backend. Không mở HTTP/WebSocket công khai để “giải quyết” pipe.

## 11. HÌNH HỌC THẬT, SEMANTIC MAPPING VÀ READBACK

ƯU TIÊN MỚI: generic primitives/read/edit trước, tấm đế là regression. Sau đó bổ sung graph/geometry compiler floorplan ở 7.3.1. Không mở rộng form tấm đế để thay cho công cụ tổng quát.

Các entity theo từng mốc: Line, Circle, Polyline kín/mở, Arc, MText, layer và dimension. Move/rotate/property update chỉ ở selection/semantic group được phép; erase cần diff và duyệt rõ. Không sửa xref/entity type chưa hỗ trợ.


### 11.1 Phạm vi hình học ban đầu

Model space; hình 2D trong WCS XY, Z=0. Đơn vị domain mm; conversion có metadata và preview. Unitless/không rõ đơn vị phải được xác nhận; không sửa global INSUNITS để ép khớp. Không tự biến tọa độ UCS thành WCS mà không transform/kiểm tra.

Thực hiện bằng AutoCAD Managed .NET API, không SendStringToExecute hoặc raw command string làm đường chính.

Tấm đế chuẩn: width=200, height=120, hole_diameter=10, edge_offset=20, origin=(0,0).
- Đường bao là polyline kín.
- Bốn circle bán kính 5 tại (20,20), (180,20), (180,100), (20,100).
- Layer riêng theo phạm vi đã duyệt; không đổi thuộc tính layer có sẵn không liên quan.
- Dimension thật cho chiều dài, chiều rộng và đường kính đại diện. Không text giả ghi “200” để che hình học sai.

GeometryPlanner kiểm finite values, bounds, khoảng cách lỗ/mép, lỗ không chồng nhau và tolerance. Preview và apply sử dụng cùng phương án hình học, bridge kiểm lại các ràng buộc quan trọng.

### 11.2 Semantic mapping

Ghi semantic object id, kind, parameter schema/version, geometry roles và entity bindings. Metadata trong DWG ở namespace riêng, không ghi đè dữ liệu plugin khác.

Không coi ObjectId hay entity handle là ID toàn cầu. Binding luôn gắn tài liệu/session/file identity phù hợp, kiểm lại type/fingerprint khi reopen, Save As, copy, erase hoặc Undo. Tài liệu cùng GUID do sao chép phải được phát hiện/ghép lại trước khi sửa, không tự nhận là cùng target.

Resize width=240 giữ height=120, radius=5, offset=20: hai lỗ bên phải có X=220; không tạo tấm thứ hai. Cập nhật dimension và metadata. Nếu người dùng đã thay hoặc xóa entity, báo conflict và replan, không dựng lại mất dấu một cách âm thầm.

Primitives skill phải thực hiện thêm được line/circle/rectangle/polyline theo schema; không chỉ nhận được prompt mà trả lại tấm đế fixture.

### 11.3 Transaction và readback

Mutation đi qua main-thread context, lock khi cần, transaction ngắn, dispose đúng. Chỉ báo applied sau khi AutoCAD xác nhận commit. Readback đọc entity thực tế và validator kiểm loại, số lượng theo vai trò, tọa độ, closed state, radius, layer, dimension measurement và quan hệ tham số.

Không dùng số entity tổng của toàn DWG làm kỳ vọng cố định; dimension có thể tạo dữ liệu hỗ trợ. Kiểm nhóm entity thuộc operation/semantic object và phần không liên quan không bị đổi.

Phân biệt applied_unverified, verified, verification_failed và outcome_unknown. Validator thất bại không được biến thành success. Không tự xóa toàn bộ operation như cách sửa lỗi nếu người dùng chưa duyệt phương án phục hồi.

## 12. RECOVERY, IDEMPOTENCY VÀ HOÀN TÁC

SQLite ứng dụng và DWG không nằm trong cùng transaction phân tán. Thiết kế phải có reconciliation; không hứa exactly-once dưới mọi dạng crash.

Lưu intent trước gửi; plugin trả receipt gắn operation_id/hash/target. Khi phù hợp, ghi marker/receipt trong DWG cùng transaction với hình học; marker này cũng chịu ảnh hưởng của Undo và việc DWG đã lưu hay chưa.

Nếu mất kết nối sau gửi mutation, trạng thái là outcome_unknown. Tra operation status và đọc lại trước mọi retry. Một app receipt cũ không chứng minh hình vẫn còn sau AutoCAD Undo, đóng không lưu hoặc mở phiên bản DWG cũ.

Không reapply tự động khi AutoCAD restart. Phiên mới thu hồi permit cũ, đối chiếu document/mappings và tạo approval mới khi cần. Tách applied_in_memory với saved_to_disk; không báo saved khi chưa có sự kiện/kết quả lưu xác minh được.

Hoàn tác trong ứng dụng là inverse change-set cho operation được quản lý, có preview/approval và kiểm tra after-state. Không gọi UNDO mù để có thể vô tình hoàn tác thao tác người dùng khác. Nếu đã có chỉnh sửa chen vào, báo conflict hoặc tạo phương án khôi phục mới.

Nếu người dùng chạy AutoCAD UNDO/REDO trực tiếp, plugin phải invalidation/reconcile state và không giữ revision cũ như thể bản vẽ chưa đổi. Tối thiểu phát hiện được ở lần đọc/ghi kế tiếp trước khi cho mutation.

Backup/migration theo master; không sao chép riêng SQLite main file đang có WAL như backup đầy đủ. Không tự ghi đè DWG nguồn. Test trên drawing tạm hoặc bản sao do người dùng chọn, không dùng đồ án quan trọng làm thử nghiệm đầu tiên.

## 13. DATABASE — BỔ SUNG BẰNG MIGRATION, KHÔNG TẠO LẠI

Audit schema trước. Các tên sau là logical entities; nếu đã có cấu trúc tương đương thì map/tái sử dụng thay vì tạo bảng trùng.

| Logical entity | Dữ liệu cần bổ sung/lưu |
|---|---|
| provider_accounts | provider kind, auth_mode, label, validated identity/workspace metadata, registration/client reference, status, secret_ref |
| provider_configs | model/options/capability profile, account relation, enabled state; không raw token |
| model_catalog_cache | account/auth-route scoped catalog, fetched_at, capabilities; không dùng cache lẫn giữa account |
| skills / project_skills | version/hash, manifest, trust, enabled, config và permissions đã duyệt |
| run_skill_bindings | run, skill version/hash snapshot, tool schema versions, validator result |
| cad_installations | profile/release/build/reference-path metadata đã xác minh; không coi installed là connected |
| cad_sessions | process/session/protocol/plugin/capability và lifecycle; trạng thái connected phải hết hiệu lực khi restart |
| documents / entity_bindings | mock/live discriminator, identity/revision/fingerprint và semantic roles |
| agent_runs / tool_calls | account/model/skill/CAD context snapshot và trạng thái thực thi |
| execution_receipts | operation/hash, applied/verified/unknown, observed geometry state, persistence state |
| audit_events | consent, connect/disconnect, approval, mutation, recovery; dữ liệu an toàn |

OAuth state/PKCE đang chờ chỉ giữ trong bộ nhớ có timeout trừ khi có thiết kế recovery an toàn cần thiết. Private token và pairing secrets không nằm trong bảng sessions.

App Core là writer SQLite duy nhất. Session được lưu để audit không tự trở thành phiên đã xác thực ở lần chạy mới. Thay account không xóa lịch sử, cũng không tự replay run cũ.

Migration giữ project/chat/demo hiện có, FK/index/unique constraints phù hợp, backup trước migration rủi ro và test từ snapshot schema cũ. Không cần database cá nhân của tác giả để fresh clone chạy.

Không đổi engine/ORM chỉ vì sở thích. SQLite thường chưa mã hóa toàn bộ chat/metadata: ghi rõ giới hạn, vault tách riêng. Giữ retention/export diagnostics an toàn và cơ chế không lưu chat khi người dùng chọn.

### 13.1 Dữ liệu cho chat-first/floorplan

Tiếp tục migration sau schema thực tế; không gán số migration khi chưa audit. Đây là logical entities, reuse bảng hiện có khi tương đương.

| Entity | Dữ liệu |
|---|---|
| design_briefs | Project/conversation/version, units, constraints, confirmed/assumed fields |
| design_proposals | Run/message/source, target/revision, graph patch, geometry hash, status/preview |
| design_objects | Project/document, semantic ID, kind/schema version, properties/fingerprint |
| design_relations | Object from/to, host/adjacency/dependency relation, version |
| entity_bindings | Semantic ID/role -> drawing-scoped handle/type/fingerprint |
| run_skill_bindings | Skill version/hash đã pin, tool/schema capabilities |
| usage_records | Account/auth route/model/run, usage thật hoặc estimate có nhãn; không auth token |

Project -> conversations -> runs -> proposals -> approvals -> executions. Document -> semantic objects -> entity bindings. Mỗi run pin account/model/skill/target/policy. Chỉ một active pending proposal cho cùng conversation/document; chặn race bằng constraint/index/transaction.

SQLite không thay DWG và không lưu raw credential. Geometry thật là nguồn kiểm chứng hiện trạng CAD; graph/cache phải reconcile sau sửa tay/Undo/reopen.

Giữ DELETE/FULL đang dùng nếu chưa có thay đổi được kiểm thử. Không bật WAL hay nâng engine chỉ để đánh dấu backlog. Đối chiếu advisory chính thức và test migration/backup/recovery khi thay. Không xóa lockfile để lấy mọi dependency mới nhất.


## 14. SECURITY GATES KHÔNG ĐƯỢC BỎ QUA

- Tauri capabilities/custom commands kiểm tra quyền ở backend; không mặc định invoke_handler là default-deny.
- UI không có arbitrary SQL, shell, file read, provider HTTP hoặc quyền gọi mọi MCP tool.
- Remote content không được chạy trong privileged WebView; markdown/link/image từ model, DWG, skill đều không tin cậy.
- Endpoint/redirect validation không mang token sang origin khác; TLS verification luôn bật.
- Model không được chọn executable, pipe path hoặc tự mở rộng workspace/file permissions.
- Chỉ backend phát hành permit; chỉ plugin đã ghép cặp được phép nhận mutation.
- Bound prompt/context, tool results, frame, queue, query và operation size; timeout/cancel có trạng thái thật.
- Secret scanning và structured-log allowlist; không chỉ che bằng regex sau khi đã log raw request.
- Không bật telemetry/upload âm thầm; diagnostic export có xem trước và loại credential/chat/DWG mặc định.
- Dependency locks, vulnerability/license checks và báo các finding chưa xử lý.
- Debug/test command hỗ trợ bypass chỉ được có trong build test, không đóng gói release.

## 15. MA TRẬN KIỂM THỬ BẮT BUỘC

Giữ test cũ. Thêm test theo hành vi, không đặt mục tiêu số lượng test cho đẹp.

### A. Runtime và frontend

Desktop native gọi Rust thật, đọc/ghi project, khởi động lại giữ dữ liệu. Browser preview không cho lưu credential hay apply live. Settings/Skills truy cập được trước project. Mode và backend status không bị hardcode.

### B. Provider và OAuth

API fixture và OAuth fixture như mục 6; đăng nhập thành công nhưng thiếu inference scope không được hiện ready. Token không xuất hiện trong SQLite/log/events/artifacts. Không tự đổi billing route. Live inference/tool-calling phải có bằng chứng riêng, người dùng chủ động cho phép.

### C. Skill engine

Bật/tắt, thiếu tool, input invalid, tampered hash, reference path traversal, skill update tăng quyền và instruction injection. Kiểm thử các skill theo mốc R3/R4; mounting-plate là regression. Chứng minh skill disable không còn được router chọn và một run đã pin không đọc hướng dẫn bị đổi giữa chừng.

### D. Bridge read-only và authorization

Handshake thật, chọn đúng document, lỗi version, frame lớn, pipe peer không hợp lệ, permit sai/hết hạn/phiên cũ, sai document, sai payload hash, document busy/closed. Không có approval thì không thay hình học.

### E. Luồng tạo/sửa/hoàn tác chuẩn

1. Project test, đơn vị mm, origin (0,0), chọn bản vẽ test.
2. Gửi: “Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm. Ghi kích thước.”
3. Có preview, trước duyệt không thay document.
4. Từ chối -> không tạo entity.
5. Tạo lại và duyệt -> một nhóm hình học đúng; readback/validator pass.
6. Gửi: “Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép.”
7. Preview diff rõ; duyệt -> sửa nhóm cũ, X lỗ bên phải=220.
8. Revert có duyệt -> về trạng thái trước resize khi không có sửa chen vào.

### F. Chống hardcode

- Tấm 260 x 160 mm, đường kính lỗ 12 mm, offset 25 mm: tâm (25,25), (235,25), (235,135), (25,135), radius=6.
- Resize width=300: hai tâm bên phải X=275, giữ các ràng buộc khác.
- Primitive: rectangle 100 x 60 tại (300,0), circle tâm (350,30), radius=10; không sinh tấm bốn lỗ ngoài yêu cầu.
- Thiếu kích thước/đối tượng đích -> hỏi hoặc chờ xác nhận, không lấy fixture default âm thầm.

### G. Recovery và thao tác ngoài

Double approve không tạo trùng. User sửa hình sau preview -> conflict. Commit rồi mất pipe -> unknown/reconcile, không vẽ lại. Sidecar crash -> không replay. AutoCAD Undo/Redo -> state invalidation. Save As/copy/close without save -> mappings/receipt được đối chiếu. Hai tài liệu có tên giống nhau không bị chọn nhầm.

### H. Môi trường và bằng chứng

Tách cột kết quả: unit/HTTP fixture, MCP process integration, Windows native, Windows vault/OAuth callback, AutoCAD protocol simulator, AutoCAD live read-only, AutoCAD live write, live LLM+live CAD, installer smoke.

Simulator C# chạy độc lập không chứng minh DLL nạp trong AutoCAD. dotnet build thành công không chứng minh AutoCAD load thành công. Ảnh trong preview không chứng minh có entity trong DWG.

Live CAD evidence gồm release/build/runtime, plugin version, document test identity, operation receipt, entity readback/validation report và log đã làm sạch. Screenshot là bổ sung; không thay readback. Nếu có save test, ghi riêng đường file test được cấp quyền và kết quả reopen.

### I. Nghiệm thu CHAT-FIRST mới - bắt buộc

1. Native build mới mở được Chat/Accounts/Connections khi chưa project. Không bị buộc vào form tấm đế. About có build ID; restart giữ chat/settings, không replay mutation.
2. OAuth: login browser thật -> validated identity/scopes -> account catalog -> harmless function-call probe -> completion. API-key route test riêng. Thiếu quyền/hạn mức phải báo đúng. Không tự chuyển billing.
3. Plugin được AutoCAD nạp, handshake thật, đọc drawing/selection/units/layer không đổi dirty state.
4. Prompt: 'Vẽ hình chữ nhật 8000 x 12000 mm, góc trái dưới (0,0), layer AI_TEST. Ghi hai kích thước.' Chưa duyệt không ghi. Duyệt -> polyline kín/bốn đỉnh đúng/hai dimension đo thật/readback pass. Thử thêm line/circle kích thước khác; không sinh tấm đế ngoài yêu cầu.
5. Selection rectangle: 'Dịch 500 mm theo +X.' Preview/duyệt/sửa nhóm cũ, không tạo rectangle thứ hai. Annotation phụ thuộc phải cập nhật hoặc báo giới hạn trước duyệt. Revert có duyệt, không UNDO mù.
6. Floorplan: 'Phác thảo mặt bằng nhà 8 x 12 m, một tầng, hai phòng ngủ, phòng khách, bếp và một WC.' Agent làm rõ brief/giả định, không tự coi kích thước tổng là thông thủy. Sau xác nhận, đề xuất layout có ràng buộc, duyệt -> entity tường/cửa/nhãn/dimensions thật, graph bindings và báo cáo checks. Layer gợi ý AI_WALL/AI_DOOR/AI_WINDOW/AI_ROOM/AI_DIM; không đổi layer cũ ngoài quyền.
7. Sửa cửa: chọn một cửa đủ khoảng trống, dịch 300 mm dọc tường. Giữ width, update opening/wall geometry, không tạo bản nhà thứ hai. Ca vượt đầu wall phải báo lỗi/hỏi, không ghi sai.
8. Chống hardcode: thay footprint/số phòng và bộ tham số primitives. Kiểm geometry/invariants, không yêu cầu LLM trả một chuỗi văn bản cố định.
9. Stale preview: form/run cũ có preview 204; run mới unsupported. Không gắn 204 vào run mới hoặc cho duyệt nhầm qua stale UI/API. Thử late event sau account/document switch.
10. Save/reopen: user cấp quyền lưu DWG test, đọc lại sau reopen và reconcile bindings. Phân biệt applied/saved. Giữ đủ các ca double approve, commit/disconnect, manual edits, Undo/Redo, Save As ở phần G.

Evidence phải có build ID, auth route/model (không secrets), trace tool, plugin/protocol version, target test, proposal/operation IDs, receipt và readback/validator. 40 + 9 tests cũ không thay các ca live mới. Chưa có login/key thì live NOT_RUN/BLOCKED, không gắn nhãn PASS từ fixture.


## 16. ĐÓNG GÓI WINDOWS

Ưu tiên Windows native build; NSIS installer .exe có thể là lựa chọn đầu nếu hợp repository [S20]. Phân biệt installer .exe, app executable, MCP sidecar executable và AutoCAD plugin DLL/.bundle.

Đóng gói sidecar theo target, skill resources và bridge profile đã build/kiểm thử. Không bắt người dùng cuối cài Node, pnpm, Cargo hoặc source checkout để chạy một bộ cài đã phát hành. Prerequisite phát triển và runtime phải có hướng dẫn riêng; AutoCAD phù hợp vẫn cần được cài/cấp phép cho chế độ live.

Thử app từ thư mục cài thực tế, path có khoảng trắng/Unicode, restart, uninstall giữ project/DWG/credential theo lựa chọn rõ ràng. Không auto-update khi chưa có cơ chế ký/xác minh. Unsigned dev build ghi rõ, không yêu cầu tắt Windows security để dùng.

Chỉ báo đã có .exe khi file thực tồn tại, có size/hash và log build. Không đổi đuôi file khác thành .exe. Có CI config không có nghĩa đã có artifact từ CI.

## 17. LỘ TRÌNH R1-R5 - ĐƯỜNG CHẠY THẬT TRƯỚC

| Mốc | Thực hiện | Đầu ra có thể nghiệm thu |
|---|---|---|
| R0 | Audit delta + regression smoke | Giữ T0 đã pass; xác định source/build đang chạy |
| R1 | Chat-first, global accounts, OAuth/API-key, vault, catalog/probe | LLM thật chat và harmless tool call; login thiếu quyền báo đúng; sửa stale preview |
| R2 | C# plugin build/load, pairing, AutoCAD read-only | Đọc drawing/selection/layer/units thật không mutation |
| R3 | Generic primitives, modify-selection, minimum skill runtime, approval/readback | LLM thật vẽ/sửa rectangle/circle/line/dimension trong AutoCAD thật; verify/revert |
| R4 | Skill runtime đầy đủ, floorplan concept, semantic graph/audit | Brief -> layout -> duyệt -> mặt bằng 2D thật -> sửa cửa -> readback |
| R5 | Native UI E2E, recovery, save/reopen, NSIS | Bộ cài, plugin/sidecar/resources đúng, smoke trên đường cài |

SkillRegistry/Loader/provenance tối thiểu là dependency của R3; không đợi R4 mới bắt đầu. Ngược lại, không chờ đủ skill nhà mới làm bridge.

Có thể làm R2/tests hình học khi R1 chờ user login. Không vượt write gate khi read-only chưa kiểm chứng. R1/R2 chưa là hoàn thành sản phẩm. Mốc R thay lộ trình T cũ về thứ tự; yêu cầu kỹ thuật kế thừa vẫn giữ.

Lát cắt ngay: R0 ngắn -> R1 có mã/test -> R2 theo điều kiện. Chỉ đề nghị user login/mở AutoCAD/duyệt pairing khi đường tương ứng đã chạy; không đòi key/token trong chat.

## 18. BÀN GIAO PHẢI GẮN VỚI BẢN BUILD MỚI

Cập nhật README, IMPLEMENTATION_STATUS, HANDOFF, SECURITY và tài liệu auth/skills/bridge/database/testing hiện có. Evidence mới: docs/evidence/continue03/. Không tạo nhiều bản sao trùng trạng thái.

Bảng báo cáo: capability | code status | test kind | result | environment | evidence | remaining blocker. Tách NOT_IMPLEMENTED, IMPLEMENTED_NOT_TESTED, FIXTURE_PASS, NATIVE_PASS, LIVE_READ_PASS, LIVE_WRITE_PASS, LIVE_E2E_PASS, FAILED, BLOCKED_EXTERNAL. Không cộng hai lần handoff dán trùng.

Báo user làm được gì/mode nào; file/module đã sửa; lệnh/cwd/exit code; bước kế tiếp. Mỗi executable mới cần đường chạy/build ID/size/hash. About/Diagnostics hiện build ID để phân biệt bản mới với debug copy cũ.

Không đóng app đang dùng hoặc đè đường dữ liệu không được phép. Khi cần restart để dùng build mới, hướng dẫn rõ và giữ dữ liệu. Hết ngữ cảnh thì giữ source/test nhất quán, ghi handoff thật; không hứa làm nền.

## 19. NGUỒN KỸ THUẬT CẦN ĐỐI CHIẾU

[S1-S20] kế thừa từ prompt 02. Ngày 07/10/2026 đã đối chiếu lại OpenAI plan usage/sign-in/sessions/models/preview limitations/billing/function calling; Autodesk .NET compatibility/threading/locking; Microsoft WinCred/DPAPI; MCP architecture. AI triển khai phải kiểm tra lại tài liệu đúng phiên bản thực tế.

Contract, database, skill nhà và mốc trong prompt là yêu cầu thiết kế của sản phẩm, KHÔNG phải tính năng nhà cung cấp đã tặng sẵn.

[S1] Tauri prerequisites:
`https://v2.tauri.app/start/prerequisites/`

[S2] ChatGPT và API billing:
`https://help.openai.com/en/articles/9039756-managing-billing-for-chatgpt-and-the-api-platform`

[S3] OpenAI function calling:
`https://developers.openai.com/api/docs/guides/function-calling?api-mode=responses`

[S4] Sign in with ChatGPT / ChatGPT plan usage overview:
`https://developers.openai.com/siwc/token-sharing-open-source`

[S5] Registration and sign-in:
`https://developers.openai.com/siwc/token-sharing-open-source/sign-in`

[S6] Accounts and sessions:
`https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions`

[S7] Models and inference:
`https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference`

[S8] Preview limitations:
`https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations`

[S9] OAuth for native apps, RFC 8252:
`https://www.rfc-editor.org/rfc/rfc8252`

[S10] Windows CREDENTIALW và giới hạn credential blob:
`https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw`

[S11] DPAPI CryptProtectData:
`https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata`

[S12] MCP Rust SDK chính thức:
`https://github.com/modelcontextprotocol/rust-sdk`

[S13] Tauri sidecar/external binaries:
`https://v2.tauri.app/develop/sidecar/`

[S14] AutoCAD Managed .NET compatibility:
`https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6C680F2-DE2E-418A-A182-E4884073338A.htm`
AutoCAD 2026.1.2 release notes về khả năng tương thích:
`https://help.autodesk.com/view/ACD/2026/HUN/?guid=AUTOCAD_2026_1_2_FIXED_ISSUES`

[S15] Autodesk plugin deployment:
`https://help.autodesk.com/view/ADSTPR/2026/ENU/?guid=GUID-5E50A846-C80B-4FFD-8DD3-C20B22098008&p=ACD`

[S16] AutoCAD SECURELOAD và TRUSTEDPATHS:
`https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-541566C6-2738-49DD-87C3-C1490E924A02.htm`
`https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-2FB4611D-F141-48D5-9B6E-460EB59351AF.htm`

[S17] Autodesk document locking:
`https://help.autodesk.com/cloudhelp/2026/DEU/OARX-DevGuide-Managed/files/GUID-A2CD7540-69C5-4085-BCE8-2A8ACE16BFDD.htm`

[S18] Autodesk network I/O và AutoCAD API threading:
`https://blog.autodesk.io/running-network-io-in-autocad-net-commands/`

[S19] Windows named pipe security:
`https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights`

[S20] Tauri Windows installer:
`https://v2.tauri.app/distribute/windows-installer/`

[N1] MCP architecture:
`https://modelcontextprotocol.io/docs/learn/architecture`


## 20. BẮT ĐẦU THỰC HIỆN

Đọc prompt này và handoff 07/10, kiểm tra repository. Tạo delta ngắn rồi viết R1 ngay. Giữ native foundation, chuyển mock ra khỏi trải nghiệm mặc định, triển khai login/provider thật và bridge AutoCAD thật. Chứng minh primitives end-to-end trước, rồi mở rộng skill mặt bằng nhà.

Thành công không phải 'có màn hình', 'có SKILL.md' hay 'DLL build được'. Đích là prompt -> LLM thật gọi tool -> proposal đúng -> duyệt -> AutoCAD thật thay đổi -> readback xác minh -> sửa tiếp đúng đối tượng. Không chỉ trả về một kế hoạch mới.

