# CAD AGENT STUDIO — CONTINUATION 04
# R2 — AUTOCAD LIVE READ-ONLY BRIDGE

Ngày: 07/10/2026

## 0. MỤC TIÊU

Tiếp tục repository CAD Agent Studio hiện tại.

KHÔNG tạo lại dự án.
KHÔNG đổi stack.
KHÔNG làm lại T0/R1/R1B.
KHÔNG mở rộng Developer Demo.
KHÔNG cho phép ghi/sửa DWG trong R2.

Trạng thái đã bàn giao:

- Windows native desktop đã chạy.
- Build R1B đã có.
- Continue with ChatGPT đã LIVE_PASS.
- Model catalog đã LIVE_PASS.
- Text inference đã LIVE_PASS.
- Function calling thật đã LIVE_PASS.
- Standalone live chat hoạt động khi CAD disconnected.
- Windows vault/persistence đã có kiểm chứng.
- MCP sidecar đã tồn tại.
- AutoCAD 2026.1.2 đã được phát hiện trên Windows.
- AutoCAD live bridge C# chưa được triển khai/nghiệm thu.

Mục tiêu duy nhất của R2:

Người dùng mở AutoCAD thật -> plugin C# được load -> CAD Agent Studio ghép cặp
với đúng AutoCAD process/session -> App Core gọi MCP -> cad-tool-host gọi bridge
-> plugin đọc document thật -> dữ liệu trả về cho LLM thật -> LLM trả lời người dùng.

R2 CHỈ READ-ONLY.

Không được tạo, sửa, xóa entity.
Không được đổi layer, INSUNITS, UCS, style hoặc metadata DWG.
Không được Save/SaveAs.
Không được chạy raw command string/AutoLISP để "làm cho nhanh".

Đích nghiệm thu tối thiểu:

Người dùng mở một DWG thử nghiệm rồi hỏi:

"Cho tôi biết bản vẽ đang mở có bao nhiêu layer, đơn vị gì và hiện đang chọn bao nhiêu đối tượng."

LLM thật phải gọi tool thật, dữ liệu phải được đọc từ AutoCAD thật qua bridge,
sau đó LLM trả lời dựa trên kết quả đọc đó.

---

## 1. KIẾN TRÚC PHẢI GIỮ

React/Tauri UI
-> Rust App Core
-> ModelProvider thật
-> Skill/tool router
-> MCP client
-> cad-tool-host
-> AutoCadGateway
-> Windows named pipe
-> C# AutoCAD plugin
-> AutoCAD Managed .NET API
-> DWG/document đang mở

LLM cloud KHÔNG được truy cập named pipe trực tiếp.
Frontend KHÔNG được gọi AutoCAD trực tiếp.
Plugin KHÔNG được nhận API key/OAuth token.
Plugin KHÔNG được tự mở SQLite của ứng dụng.

App Core vẫn quản lý account/model, conversation/run, policy, tool visibility, audit và persistence.
cad-tool-host là boundary tool/MCP.
C# plugin chỉ làm handshake, session/document discovery, read-only query và validation/readback metadata.

---

## 2. AUDIT NGẮN TRƯỚC KHI CODE

Đọc:
- HANDOFF.md mới nhất
- IMPLEMENTATION_STATUS.md
- docs/AUTOCAD_BRIDGE.md
- docs/BRIDGE_PROTOCOL.md nếu có
- docs/CONTINUE_03_DELTA.md
- docs/AUTH_COMPATIBILITY.md
- Cargo workspace
- crates/cad-tool-host
- Tauri commands/capabilities
- app-core gateway/tool contracts
- mọi source C# bridge hiện có nếu tồn tại.

Tạo docs/CONTINUE_04_R2_DELTA.md với:
Capability | Source hiện có | Test hiện có | Thiếu gì | Quyết định tái sử dụng

Không chỉ trả kế hoạch. Sau audit ngắn, bắt đầu code R2 ngay.

---

## 3. PROFILE AUTOCAD MỤC TIÊU

Máy đã phát hiện AutoCAD 2026.1.2.

Xác minh lại trên Windows:
- acad.exe path
- product/release/build
- process architecture
- runtime
- AutoCAD managed assemblies thực tế
- assembly version
- vị trí AcDbMgd.dll
- AcMgd.dll
- AcCoreMgd.dll nếu cần
- .NET runtime/TFM phù hợp
- SDK/reference thực tế.

Không chỉ suy luận từ tên "AutoCAD 2026".
Không copy Autodesk DLL vào repository nếu license không cho phép.
Không tạo fake Autodesk assemblies chỉ để compile.

Nếu references chỉ có trong installation directory:
- dùng conditional MSBuild properties/profile
- document rõ đường dẫn local
- host references không CopyLocal nếu không phù hợp.

Tạo/cập nhật docs/COMPATIBILITY.md.

---

## 4. CẤU TRÚC C# BRIDGE

Nếu chưa có, tạo cấu trúc tương đương:

bridges/
  autocad/
    CadAgent.Bridge.Protocol/
    CadAgent.Bridge.Transport/
    CadAgent.Bridge.AutoCAD/
    CadAgent.Bridge.Tests/
    CadAgent.AutoCAD.bundle/
      PackageContents.xml
      Contents/
        Windows/

Protocol không phụ thuộc Autodesk API.
Transport xử lý named pipe/framing/security.
AutoCAD là phần duy nhất tham chiếu Autodesk Managed .NET API.
Tests cho Protocol/Transport phải chạy không cần AutoCAD.

Không coi simulator test là AutoCAD LIVE_PASS.

---

## 5. PLUGIN AUTOCAD

Plugin phải thực sự được load trong AutoCAD 2026.1.2.

Tạo các command:
CADAGENTCONNECT
CADAGENTSTATUS
CADAGENTDISCONNECT

CADAGENTSTATUS tối thiểu hiển thị:
- plugin version
- protocol version
- AutoCAD release/build
- process id
- bridge state
- paired/unpaired
- active document
- read-only capability state
- last bridge error.

Không log pairing secret.

---

## 6. TRIỂN KHAI .BUNDLE

Tạo Autodesk ApplicationPlugins bundle thật.

Không yêu cầu:
- SECURELOAD=0
- tắt Defender
- whitelist cả ổ đĩa
- chạy AutoCAD admin mặc định.

Tạo:
scripts/autocad-bridge-build.ps1
scripts/autocad-bridge-install.ps1
scripts/autocad-bridge-uninstall.ps1
scripts/autocad-bridge-doctor.ps1

Yêu cầu:
- dry-run/WhatIf khi hợp lý
- chỉ thao tác bundle của dự án
- không xóa plugin khác
- báo user-scope/machine-scope
- kiểm tra target AutoCAD
- kiểm tra PackageContents.xml
- evidence log
- hỗ trợ path có khoảng trắng/Unicode.

Nếu unsigned development plugin cần trust, hướng dẫn đúng cách; không hạ security toàn máy.

---

## 7. PAIRING VÀ SESSION

UI Kết nối AutoCAD phải triển khai thật.

Luồng:
1. CAD Agent Studio phát hiện AutoCAD process phù hợp.
2. Plugin đã load.
3. User chủ động ghép cặp.
4. Hai phía tạo bridge session.
5. Handshake protocol/capability.
6. App xác minh đọc document thành công.
7. UI mới hiện AutoCAD đã kết nối.

Không coi acad.exe đang chạy, registry thấy AutoCAD, DLL tồn tại hoặc pipe tồn tại là connected.

Connected chỉ khi handshake pass, protocol compatible, đúng process/session và document readback thành công.

Pairing secret không command-line, không discovery file plaintext, không log, không SQLite plaintext, không gửi cho LLM.

AutoCAD restart tạo session mới. Session cũ không reuse.

---

## 8. NAMED PIPE SECURITY

Dùng Windows named pipe local.

Yêu cầu:
- local-only
- DACL giới hạn user/logon session phù hợp
- không Everyone write
- không Anonymous
- giới hạn payload
- versioned framing
- reject oversized frame
- timeout/cancellation
- reject protocol mismatch
- reject stale session
- request_id/operation_id
- duplicate-safe cho read.

Pipe name/PID không là bằng chứng xác thực duy nhất.
Không mở HTTP/WebSocket public chỉ để tránh named pipe.

---

## 9. THREADING / AUTOCAD CONTEXT

Listener pipe KHÔNG trực tiếp gọi AutoCAD API từ thread nền.

Mọi AutoCAD database/document API phải chạy trong context/main thread phù hợp AutoCAD Managed .NET.

Không giữ document lock/transaction/command context khi chờ LLM, mạng, user approval hoặc pipe I/O lâu.

Read-only query dùng transaction ngắn khi cần.

Nếu document busy/closing/unavailable/command state không cho phép:
trả typed error/state.
Không tự gửi ESC.

---

## 10. DOCUMENT IDENTITY VÀ REVISION

Document DTO tối thiểu:
- document_id nội bộ bridge/session
- file name
- safe path metadata nếu policy cho phép
- saved/unsaved
- active
- model/paper space
- units
- UCS/WCS context
- dirty state
- revision token
- process/session id
- lifecycle state.

Không dùng filename làm unique id.
Hai drawing cùng tên phải phân biệt.
Unsaved drawing có session identity.

Switch document/open/close/undo/redo/user edit/save as phải invalidation/update revision đủ để R3 không dùng stale state.

---

## 11. READ-ONLY TOOL CATALOG

Map sang contract hiện có nếu tương đương.

Tối thiểu:
- cad.get_status
- cad.list_documents
- cad.get_document_context
- cad.get_selection
- cad.list_layers
- cad.query_entities
- cad.validate_entities
- cad.get_operation_status

Mutation tools trong R2 phải disabled/not exposed/READ_ONLY_MODE.

---

## 12. ENTITY TYPES ĐỌC BAN ĐẦU

Tối thiểu:
- Line
- Polyline/LWPolyline
- Circle
- Arc
- DBText
- MText
- BlockReference
- Dimension cơ bản

Entity chưa support phải trả unsupported=true, không bỏ lặng.

Query hỗ trợ:
- selection hiện tại
- layer filter
- entity type filter
- bounded max results.

Không dump toàn bộ DWG mặc định vào LLM.

---

## 13. UNITS VÀ COORDINATES

Không tự sửa INSUNITS.

Read-only trả observed units, unitless/unknown khi không chắc, WCS/UCS metadata, model/paper space.

Không giả định mm chỉ vì CAD Agent Studio dùng domain mm.
Không âm thầm coi UCS = WCS nếu chưa transform đúng.

---

## 14. APP CORE / MCP INTEGRATION

Giữ R1B live LLM hiện có.

CAD disconnected: chat thường vẫn hoạt động.
Nếu user hỏi về bản vẽ đang mở khi chưa connected: yêu cầu kết nối CAD, không fallback Demo.

Khi connected:
LLM chỉ thấy read-only CAD tools phù hợp.

Flow:
User -> LLM -> cad.list_layers -> App Core validate -> MCP -> cad-tool-host -> AutoCadGateway -> pipe -> plugin -> AutoCAD -> result -> LLM -> final answer.

Không cho LLM chọn pipe path, executable, secret, shell hay raw AutoCAD command.

---

## 15. SKILL R2 TỐI THIỂU

Không xây full floorplan trong R2.

Tạo/hoàn thiện operator tối thiểu "autocad-operator":
- đọc context trước khi trả lời về drawing
- không đoán units/layer/selection
- query bounded
- thiếu CAD connection thì yêu cầu kết nối
- unsupported entity phải nói rõ
- không claim đã sửa drawing
- không expose mutation trong R2.

Nếu full skill engine chưa xong, ghi đúng trạng thái; không giả T3 đã hoàn thành.

---

## 16. UI KẾT NỐI AUTOCAD

Hiển thị:

Installation:
- AutoCAD product
- release/build
- path
- bridge compatibility

Plugin:
- installed
- loaded
- plugin version
- protocol version

Runtime:
- PID
- session state
- paired
- heartbeat

Document:
- active document
- saved/unsaved
- units
- model/paper
- revision
- dirty

Capabilities:
- read_document
- read_selection
- read_layers
- query_entities
- validate_entities
- write=false

Buttons:
- Refresh detection
- Install bridge
- Pair
- Disconnect
- Diagnostics

Không có write/apply trong R2 live CAD.

---

## 17. LIVE READ-ONLY ACCEPTANCE

Dùng DWG TEST.

Drawing test nên có:
- >=3 layer
- 2 line
- 1 rectangle/polyline
- 2 circle
- 1 text
- 1 dimension nếu tiện
- selection chủ động.

Test 1 handshake:
pair đúng process, plugin handshake, document context read, drawing không dirty thêm.

Test 2 layers:
User: "Hãy cho tôi biết bản vẽ đang mở có bao nhiêu layer và tên các layer."
LLM thật phải gọi cad.list_layers.

Test 3 selection:
User chọn 2 entity rồi hỏi: "Tôi đang chọn những gì?"
LLM gọi cad.get_selection và phản ánh selection thật.

Test 4 units/context:
"Bản vẽ này đang dùng đơn vị gì, đang ở model space hay paper space?"

Test 5 bounded entity query:
"Có bao nhiêu đường tròn trên layer X?"
Không dump toàn DWG.

Test 6 document switch:
Mở 2 drawing, switch active document, app phải biết target đổi.

Test 7 disconnect:
Disconnect bridge; CAD query trả typed CONNECTION_REQUIRED, chat thường vẫn hoạt động.

---

## 18. ZERO-MUTATION PROOF

R2 chỉ PASS khi read-only tests không làm thay đổi DWG.

Kiểm tra trước/sau:
- entity count/identity subset
- layer metadata relevant
- dirty state
- revision.

Sau tests:
- không entity mới
- không entity sửa
- không layer mới
- không INSUNITS change
- không save
- không metadata marker làm dirty drawing.

Nếu session tracking đang cần metadata write, thiết kế lại để R2 giữ session identity ngoài DWG.

---

## 19. TEST MATRIX

A. .NET protocol unit tests:
serialization, unknown fields, version mismatch, malformed request, payload bound, Unicode/path, cancellation.

B. Pipe integration:
start/stop, fragmented frame, oversized frame, timeout, duplicate request, stale session, invalid pairing, second client.

C. Rust gateway:
tool mapping, typed errors, disconnected, wrong document, query bounds.

D. Simulator:
ghi SIMULATOR_PASS, không dùng thay LIVE_PASS.

E. AutoCAD live:
plugin load, handshake, document, selection, layers, entity query, zero-mutation.

F. Live LLM + Live CAD:
ít nhất list layers, selection, units/context.

---

## 20. EVIDENCE

Tạo docs/evidence/continue04-r2/ gồm tối thiểu:

autocad-detection.json
bridge-build.txt
bridge-install.txt
plugin-load.json
pairing-handshake.json
document-context.json
layer-query.json
selection-query.json
entity-query.json
zero-mutation-check.json
live-llm-live-cad.json
windows-native-build.txt
windows-startup.json

Không log OAuth token, refresh token, API key, pairing secret.

Evidence có:
- app build id
- plugin version
- protocol version
- AutoCAD release/build
- process/session
- safe document id
- request id
- status.

---

## 21. STATUS TAXONOMY

Dùng:
NOT_IMPLEMENTED
IMPLEMENTED_NOT_TESTED
FIXTURE_PASS
SIMULATOR_PASS
NATIVE_PASS
AUTOCAD_LIVE_READ_PASS
LIVE_LLM_LIVE_CAD_PASS
BLOCKED_EXTERNAL
FAILED

Không dùng DONE chung chung.

---

## 22. EXIT CRITERIA R2

R2 PASS khi:

1. Plugin C# build cho AutoCAD profile mục tiêu.
2. Bundle install/load trong AutoCAD thật.
3. CADAGENTSTATUS chạy.
4. App pair đúng AutoCAD process.
5. Handshake thật pass.
6. Đọc active document thật.
7. Đọc units/space/revision.
8. Đọc layers thật.
9. Đọc current selection thật.
10. Query Line/Polyline/Circle/Text thật.
11. Read-only không làm dirty/thay DWG.
12. LLM thật gọi ít nhất một CAD read tool thật.
13. Tool result quay lại LLM và final answer đúng.
14. Disconnect/reconnect hoạt động.
15. Không mock fallback âm thầm.
16. Evidence không lộ credential.
17. R1B standalone chat regression PASS.

Dotnet build hoặc simulator pass một mình KHÔNG phải R2 PASS.

---

## 23. KHÔNG LÀM TRONG R2

Không live mutation:
- create line
- create circle
- move
- delete
- resize
- add dimension
- create layer
- modify entity
- save DWG.

Không full floorplan, 3D, Civil 3D, Revit, cloud sync, marketplace, arbitrary code execution.

---

## 24. SAU KHI R2 PASS

DỪNG và bàn giao.
Không tự mở full R3 mutation nếu user chưa yêu cầu tiếp.

R3 sau này:
LLM thật -> skill -> immutable ChangeSet -> preview -> user approval -> permit -> MCP -> plugin -> AutoCAD transaction -> readback -> validation.

---

## 25. BÀN GIAO

Cập nhật:
- HANDOFF.md
- IMPLEMENTATION_STATUS.md
- README.md
- docs/AUTOCAD_BRIDGE.md
- docs/BRIDGE_PROTOCOL.md
- docs/COMPATIBILITY.md
- docs/TESTING.md
- docs/CONTINUE_04_R2_DELTA.md

Bàn giao:
- app executable mới
- build ID
- exact path
- bytes
- SHA256
- bundle path
- plugin DLL path
- plugin SHA256
- plugin version
- protocol version
- AutoCAD profile tested.

Nêu đúng thao tác user cần:
- mở AutoCAD
- mở DWG test
- load/allow plugin khi flow thật sẵn sàng
- chạy CADAGENTCONNECT nếu cần
- chấp nhận pairing trong app.

---

## 26. LỆNH GIAO VIỆC

BẮT ĐẦU THỰC HIỆN R2 NGAY.

Không chỉ viết kế hoạch.

Ưu tiên:
1. Audit bridge/contracts hiện tại.
2. Xác minh AutoCAD 2026.1.2 references/runtime.
3. Build Protocol + Transport + plugin.
4. Build/install .bundle.
5. Load plugin live.
6. Implement pairing + handshake.
7. Implement document context.
8. Implement layers/selection/entity read tools.
9. Wire AutoCadGateway vào cad-tool-host.
10. Wire MCP/App Core.
11. Wire LLM read-only CAD tool calls.
12. Chạy live read-only acceptance.
13. Zero-mutation proof.
14. Regression R1B.
15. Update evidence/handoff.

Nếu gặp blocker:
- tiếp tục phần độc lập
- ghi chính xác blocker
- không thay live bằng mock rồi đánh dấu pass
- không hạ security để ép chạy.

Đích cuối R2:

Người dùng hỏi bằng tiếng Việt trong CAD Agent Studio,
LLM thật gọi MCP,
plugin C# đọc AutoCAD 2026.1.2 thật,
kết quả DWG thật quay lại LLM,
LLM trả lời đúng,
và bản vẽ không bị thay đổi.
