# PROMPT TIẾP NỐI 02 — CAD AGENT STUDIO: LLM THẬT, SKILL ENGINE VÀ AUTOCAD THẬT

> Đặc tả giao việc cho AI lập trình có quyền đọc, sửa repository và chạy terminal.
> Dùng trong repository hiện tại; KHÔNG tạo lại dự án.
> Ngày soạn: 06/10/2026. UI, hướng dẫn và báo cáo bằng tiếng Việt; identifier trong code bằng tiếng Anh.
> Đây là yêu cầu triển khai và kiểm thử, không phải xác nhận rằng các tính năng bên dưới đã tồn tại.

## 0. MỤC TIÊU VÀ QUY TẮC ƯU TIÊN

Tiếp quản CAD Agent Studio hiện có và hoàn thiện một bản Windows desktop có đường chạy:

Người dùng kết nối LLM -> chọn project và bản vẽ AutoCAD -> agent chọn skill -> đọc ngữ cảnh thật -> lập phương án -> preview -> người dùng duyệt -> plugin ghi đối tượng thật -> đọc lại -> kiểm tra -> lưu lịch sử -> sửa tiếp hoặc hoàn tác có kiểm soát.

Đọc trước:
- CAD_AGENT_BASE_MASTER_PROMPT.md.
- CAD_AGENT_CONTINUE_01_PROMPT.md.
- README.md, IMPLEMENTATION_STATUS.md, HANDOFF.md và mã hiện có.

QUAN TRỌNG: CONTINUE_01 cố ý để OAuth và bridge ghi DWG ngoài phạm vi. Đợt này MỞ RỘNG PHẠM VI để triển khai các phần đó. Không viện dẫn giới hạn cũ để bỏ qua yêu cầu mới. Giữ nguyên các yêu cầu bảo mật, approval, migration, recovery của master. Nếu có mâu thuẫn về phạm vi, tài liệu này ưu tiên cho đợt triển khai hiện tại; không dùng điều đó để hạ mức bảo vệ.

Không chấp nhận hoàn thành bằng giao diện tĩnh, adapter luôn trả thành công, chức năng chỉ có TODO hoặc ảnh preview giống AutoCAD. Không đổi stack, không dựng lại scaffold, không chuyển sang Selenium/điều khiển chuột, không thêm backend cloud chỉ để có màn đăng nhập.

Không đòi người dùng gửi API key, mật khẩu hoặc token vào hội thoại lập trình. Đăng nhập và cấp quyền thực hiện trong ứng dụng hoặc trang chính thức của nhà cung cấp.

## 1. ĐIỂM XUẤT PHÁT CẦN KIỂM CHỨNG

Thông tin bàn giao của người dùng:
- Có React/Tauri, Rust, SQLite và demo tạo -> resize -> hoàn tác.
- Báo cáo mới cho biết ba kiểm thử tích hợp MCP đạt.
- Bản đang được xem là frontend trong trình duyệt Windows, không phải Tauri native.
- Ảnh ghi rõ LLM mock, CAD mock, AutoCAD chưa tích hợp; MockProvider chỉ xử lý fixture tấm đế.
- Windows được báo thiếu Node/pnpm, Rust, C++ Build Tools và Windows SDK.

Đây là báo cáo đầu vào, không phải kết quả bạn đã xác minh. Một màn hình không cho biết toàn bộ mã nguồn hoặc số test thực sự đang chạy. Mở repository, đọc code và chạy lại các kiểm tra khả thi trước khi kết luận.

Nếu repository không truy cập được, báo đúng điểm thiếu. Không tạo một repository khác rồi gọi là bản tiếp nối. Nếu mã hiện có đã triển khai một yêu cầu bên dưới, tái sử dụng và bổ sung test, không viết thêm một đường chạy trùng.

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

Chưa triển khai: cloud sync, marketplace công khai, arbitrary code execution, tự động điều khiển toàn bộ công cụ AutoCAD, BIM/MEP/3D đầy đủ, mô-đun tính toán kỹ thuật để tự chứng nhận thiết kế. Xây nền mở rộng nhưng không hứa các khả năng này đã có.

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

## 4. T0 — AUDIT REPOSITORY VÀ GỠ CHẶN WINDOWS NATIVE

### 4.1 Audit trước khi sửa

Xác định repository, branch, working tree, thay đổi chưa commit. Đọc package scripts, pnpm lockfile, Cargo workspace/lockfile, Tauri config/capabilities, migrations, contracts, providers, MCP tests và gateway implementations.

Tạo docs/CONTINUE_02_AUDIT.md với cột:
Capability | đường code thật | đường UI thật | test đang có | trạng thái | việc còn thiếu.

Chạy baseline và ghi cwd, lệnh, phiên bản toolchain, exit code. Không xóa test, sửa assertion cho qua hoặc tính test cũ là kết quả lần này.

### 4.2 Chẩn đoán đúng môi trường

Kiểm tra Windows host thực sự truy cập được từ terminal hay chỉ có WSL. Khi dùng powershell.exe từ WSL, kiểm tra nơi thực thi, path và môi trường Windows; không coi binary Linux là toolchain Windows.

Tạo hoặc sửa scripts/doctor.ps1, chỉ chẩn đoán theo nhóm:
- Node/pnpm đúng packageManager/engines của repository, không đoán phiên bản.
- Rust host/target MSVC, cargo, linker, C++ Build Tools và Windows SDK.
- WebView2 runtime và Tauri CLI theo dependency đã khóa.
- dotnet SDK dùng cho profile bridge.
- AutoCAD release, update/build, vị trí cài, runtime khi quan sát được; không đoán chỉ từ tên thư mục.
- Tauri sidecar binary, bundle plugin và cấu hình đường dẫn.

Tài liệu Tauri xác định Windows cần C++ Build Tools và WebView2, cùng Rust phù hợp; frontend JavaScript cần Node [S1]. Không đưa một lệnh cài phiên bản mới nhất mà không kiểm tra khả năng tương thích repository.

Tạo scripts/setup-windows.ps1 với chế độ mặc định chỉ in kế hoạch/kiểm tra. Chỉ cài prerequisite sau khi người dùng cho phép rõ; dùng nguồn chính thức, kiểm tra kết quả và khả năng cần khởi động lại terminal. Không tắt Defender/SmartScreen, không hạ PowerShell execution policy toàn máy, không tự thêm trusted path rộng hoặc tự nâng quyền cả ứng dụng.

Tạo scripts/start-desktop.ps1 và scripts/build-windows.ps1 phù hợp repository. Script phải phát hiện prerequisite thiếu và báo nguyên nhân cụ thể.

Chuẩn hóa script nếu chưa có:
- pnpm dev: Tauri desktop và backend thật.
- pnpm dev:web: frontend riêng.
- pnpm test:core, pnpm test:mcp, pnpm test:skills.
- pnpm package:windows: build và đóng gói Windows thực sự.

Không tạo vòng lặp giữa pnpm dev và Tauri beforeDevCommand. Không hardcode tên package/thư mục chưa tồn tại. Không spawn hai sidecar trùng nhau.

Nếu Windows chưa đủ điều kiện, vẫn xây/test core, database, provider fixtures, OAuth fixtures, skill engine và transport độc lập. Có thể chuẩn bị CI Windows nhưng chỉ ghi PASS khi job thực sự chạy. Không dừng toàn bộ công việc ở câu “thiếu toolchain”. Đồng thời không gọi việc build Linux hoặc tải trang Vite là Windows native pass.

## 5. T1 — FRONTEND CÓ KẾT NỐI LLM VÀ AUTOCAD DỄ TÌM

Giữ phong cách và thành phần tốt đang có, không thiết kế lại toàn bộ. Bổ sung navigation rõ ràng:
Workspace / Tài khoản & mô hình / Skills / Kết nối AutoCAD / Lịch sử / Chẩn đoán.

### 5.1 Onboarding

Trình tự: kiểm tra runtime -> kết nối LLM hoặc chọn demo -> kiểm tra AutoCAD hoặc chọn mock -> chọn project/document -> bật skill -> mở hội thoại.

Settings, trang provider, danh sách skill và diagnostics phải truy cập được khi CHƯA có project. Không tạo vòng khóa “cần project mới cấu hình LLM, nhưng cần LLM mới tạo project”.

Khi đang ở browser preview, hiển thị lý do thao tác native bị khóa và đường hướng dẫn mở desktop. Không chỉ để nút xám mà không giải thích. Không hardcode cổng dev trong mã production.

### 5.2 Trang Tài khoản & mô hình

Cung cấp các lựa chọn riêng:
- Demo offline — MockProvider, giới hạn fixture được ghi rõ.
- OpenAI API — nhập/thay/xóa API key, chọn model, kiểm tra kết nối.
- Continue with ChatGPT — OAuth chính thức theo mục 6, trạng thái eligibility/quyền rõ ràng.

Không làm form username/password tự chế để “đăng nhập ChatGPT”. Không gắn nút Continue with ChatGPT chỉ mở trang chủ rồi báo đã kết nối.

Provider/model được hiển thị ngay trong chat. Cho phép chọn lại giữa các run; không đổi tài khoản/model trong một run đang thực thi mà không hủy hoặc tạo run mới. Khi đổi tài khoản, không giữ catalog/credential/trạng thái lỗi của tài khoản cũ.

“Kiểm tra kết nối” có kết quả tách:
- Secret đã lưu an toàn?
- Xác thực/capability check thành công?
- Model đang chọn có dùng được?
- Một inference nhỏ có thành công?
- Function-calling probe có thành công? Probe chỉ dùng tool vô hại, không ghi CAD.

Báo trước việc live probe có thể dùng hạn mức/chi phí. Không hiện “đã có thể vẽ” chỉ vì nhận HTTP 200.

### 5.3 Chat và preview

Có streaming, lịch sử, tool activity, skill đang dùng, mode Ask/Plan/Execute có duyệt, gửi/dừng, lỗi dễ hiểu và retry phù hợp trạng thái. Không hiển thị private chain-of-thought.

Bản preview phải có document đích, đơn vị, gốc/hệ tọa độ, đối tượng thêm/sửa/xóa, phạm vi layer, bản diff tham số và nút Duyệt/Từ chối. Render từ change-set bất biến mà backend sẽ thực hiện, không từ một bản dữ liệu khác.

Sau thực hiện hiển thị receipt và kết quả validator. “Đã áp dụng vào bản vẽ đang mở” khác “Đã lưu DWG ra đĩa”. Không tự Save/Save As ngoài quyền đã duyệt.

## 6. T2 — AUTH, PROVIDER THẬT VÀ SECRET VAULT

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

## 7. T3 — SKILL ENGINE THỰC SỰ ĐƯỢC AGENT SỬ DỤNG

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

### 7.3 Ba skill mẫu có chức năng thật

**mounting-plate-2d**: tạo và resize tấm đế bốn lỗ; giữ quan hệ offset; dùng semantic object đã có; cập nhật dimension thật; đầu vào không cố định 200 x 120.

**primitives-2d**: tạo line, circle, rectangle và polyline 2D trong WCS XY qua danh sách thao tác có schema. Rectangle được chuyển thành polyline kín. Planner giới hạn số đỉnh/phần tử, finite numbers, bán kính dương, đoạn không suy biến. Chỉ tạo đối tượng trong phạm vi duyệt; chưa sửa/xóa đối tượng bên ngoài nhóm được quản lý. Không cho gửi raw AutoLISP, command string hoặc code vào schema.

**drawing-audit-2d**: đọc document/selection/layer và kiểm tra nhóm đối tượng được hỗ trợ; báo đơn vị, layer, hình học và các sai lệch xác định được. Có entity type chưa hỗ trợ phải báo giới hạn, không suy diễn là đúng/sai. Skill này read-only, không sửa bản vẽ hoặc “tự khắc phục” lỗi.

Tái sử dụng planner/validator chung cho mock và live, không tạo hai thuật toán có quy tắc khác nhau. Các test variable-parameter không được dùng output cố định.

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

## 9. T4 — MCP HIỆN CÓ VÀ AUTOCAD BRIDGE READ-ONLY

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

Tool domain chỉ đề xuất: design.propose_plate, design.propose_plate_resize, design.propose_primitives.

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

## 11. T5 — GHI HÌNH HỌC THẬT, SỬA TIẾP VÀ XÁC MINH

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

Bật/tắt, thiếu tool, input invalid, tampered hash, reference path traversal, skill update tăng quyền và instruction injection. Chạy được cả ba skill. Chứng minh skill disable không còn được router chọn và một run đã pin không đọc hướng dẫn bị đổi giữa chừng.

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

## 16. T6 — ĐÓNG GÓI VÀ TRẢI NGHIỆM NGƯỜI DÙNG CUỐI

Ưu tiên Windows native build; NSIS installer .exe có thể là lựa chọn đầu nếu hợp repository [S20]. Phân biệt installer .exe, app executable, MCP sidecar executable và AutoCAD plugin DLL/.bundle.

Đóng gói sidecar theo target, skill resources và bridge profile đã build/kiểm thử. Không bắt người dùng cuối cài Node, pnpm, Cargo hoặc source checkout để chạy một bộ cài đã phát hành. Prerequisite phát triển và runtime phải có hướng dẫn riêng; AutoCAD phù hợp vẫn cần được cài/cấp phép cho chế độ live.

Thử app từ thư mục cài thực tế, path có khoảng trắng/Unicode, restart, uninstall giữ project/DWG/credential theo lựa chọn rõ ràng. Không auto-update khi chưa có cơ chế ký/xác minh. Unsigned dev build ghi rõ, không yêu cầu tắt Windows security để dùng.

Chỉ báo đã có .exe khi file thực tồn tại, có size/hash và log build. Không đổi đuôi file khác thành .exe. Có CI config không có nghĩa đã có artifact từ CI.

## 17. MỐC BÀN GIAO VÀ TIÊU CHÍ THOÁT MỖI MỐC

| Mốc | Đầu ra có thể quan sát | Điều kiện thoát |
|---|---|---|
| T0 | Audit + Windows doctor/setup/start scripts | Baseline rõ, đường native được chạy hoặc blocker cụ thể có log |
| T1 | Onboarding, Providers, Skills, AutoCAD, Diagnostics UI | Mỗi control nối backend thật hoặc disabled với lý do chính xác |
| T2 | API-key provider + OAuth adapter/vault | Fixture pass, live checks phân biệt; không báo OAuth ready giả |
| T3 | Skill runtime + ba skill | Loader/router/executor/provenance và parameter tests chạy |
| T4 | Plugin/pipe/read-only | Native handshake và readback thật khi môi trường có; simulator tách nhãn |
| T5 | Create/resize/revert/primitives live | Approval, readback, conflict/recovery tests đạt trên AutoCAD mục tiêu |
| T6 | Live LLM + live CAD + installer | Luồng tích hợp và artifact được kiểm chứng, phần thiếu ghi rõ |

Đây là một backlog nhiều mốc, không phải yêu cầu nhồi mọi thứ vào một lần sửa không kiểm thử. Triển khai theo lát cắt chạy được, giữ repository nhất quán sau mỗi mốc. Có thể làm phần độc lập khi mốc trước bị blocker môi trường, nhưng không gắn nhãn đã nghiệm thu vượt gate.

Nếu bị giới hạn phiên, cập nhật HANDOFF với file/module đã sửa, test đã chạy, lỗi còn lại và lệnh tiếp theo. Không dùng “đã hoàn thiện hệ thống” khi mới đạt T1/T2 hoặc chỉ còn mock.

## 18. TÀI LIỆU VÀ BÁO CÁO PHẢI CÓ

Cập nhật README.md, IMPLEMENTATION_STATUS.md, HANDOFF.md, SECURITY.md và tài liệu tương ứng:
- docs/CONTINUE_02_AUDIT.md
- docs/WINDOWS_SETUP.md
- docs/AUTH_COMPATIBILITY.md
- docs/SKILLS.md
- docs/AUTOCAD_BRIDGE.md
- docs/BRIDGE_PROTOCOL.md
- docs/DATABASE.md
- docs/TESTING.md

Tên file có thể map sang tài liệu hiện có, không tạo bản trùng. Tài liệu phải khớp scripts/code thực tế, không quảng cáo tính năng chưa có.

IMPLEMENTATION_STATUS dùng các cột:
Capability | Implementation status | Test type | Test result | Environment | Evidence | Remaining blocker.

Trạng thái tối thiểu phân biệt: NOT_IMPLEMENTED, IMPLEMENTED_NOT_TESTED, FIXTURE_PASS, NATIVE_PASS, LIVE_PASS, BLOCKED_EXTERNAL, FAILED. Không dùng một ô DONE để che các mức chứng minh khác nhau.

Báo cáo cuối mỗi mốc nêu:
- Người dùng hiện làm được gì và bằng mode nào.
- Những file/module đã thay đổi.
- Lệnh, cwd, exit code, kết quả và evidence path.
- Tính năng chưa triển khai so với chỉ chưa test vì môi trường.
- Việc người dùng thực sự phải làm bằng tương tác trực tiếp: cài prerequisite được phê duyệt, đăng nhập, mở AutoCAD, chọn drawing test, chấp nhận pairing/approval.

Không hứa làm nền hoặc ước lượng thời gian vô căn cứ. Không hỏi lại stack/phạm vi đã chốt. Không dừng ở một kế hoạch mới khi có quyền sửa repository.

## 19. NGUỒN CHÍNH THỨC CẦN ĐỐI CHIẾU

Nguồn được xem khi soạn ngày 06/10/2026. AI triển khai phải kiểm tra lại API, eligibility và dependency đúng phiên bản thực tế; nếu thay đổi, ghi rõ khác biệt và điều chỉnh code/test, không bịa API. Các thiết kế contract, database, skill và gate trong tài liệu này là yêu cầu của sản phẩm, không phải tính năng có sẵn của nhà cung cấp.

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

## 20. BẮT ĐẦU THỰC HIỆN

Kiểm tra repository và chạy baseline T0 trước. Sau đó triển khai T1 -> T2 -> T3 -> T4 -> T5 -> T6 theo dependency thực tế; kiểm thử sau mỗi lát cắt và cập nhật trạng thái.

Không chỉ trả về kế hoạch. Không lấy thành công của demo cũ làm bằng chứng kết nối LLM/AutoCAD thật. Đích nghiệm thu là một người dùng có thể mở Windows desktop, kết nối provider hợp lệ, bật skill, chọn drawing test trong AutoCAD, gửi prompt, xem trước, duyệt, thấy entity thật và đọc lại được kết quả đúng.
