# Auth compatibility — R1B — 07/10/2026

Code: global accounts, API-key route, OAuth client, catalog, standalone chat/probe đã
triển khai. Fixture/native vault PASS; browser login đã xác minh theo ảnh UI user gửi.
Catalog/inference/function probe LIVE_PASS qua native AppRuntime với gpt-5.6-luna.
Evidence: evidence/continue03-r1b/windows-live-runtime.json. Lỗi policy trước đó là lịch sử.
Không yêu cầu user gửi secret hoặc client secret trong chat lập trình.

Đã đối chiếu [flow local/OSS](https://developers.openai.com/siwc/token-sharing-open-source/sign-in):
bootstrap dynamic_agent_client chỉ cho registration đầu; exchange bằng issued client_id.
Returning account giữ registration/verified subject. PKCE/state/nonce riêng, callback
loopback một lần, JWT signature/JWKS/issuer/audience/expiry trước khi lưu profile.
Scope chatgpt.tokens.use.direct được kiểm riêng; identity không đủ cho inference.

[Accounts/sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions):
refresh single-flight dưới gate và process lock; token replacement lưu DPAPI nguyên tử.
Signout thử revoke endpoint discovery origin-checked rồi xóa local; thất bại remote được
báo rõ. Không lấy email làm identity hoặc giữ login hint là token trong URL.

[Models/inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference):
OAuth catalog dùng models/slug/visibility; API key dùng data/id. User chọn model thật
trong catalog; không hardcode model. Cùng public Responses endpoint, auth routes riêng.
[Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations):
ChatGPT serializer store=false/stream=true, namespace tools, input history; bỏ
max_output_tokens/previous_response_id. Local runtime vẫn giới hạn vòng/call/thời gian.

Windows API key: WinCred. OAuth bundle: DPAPI CurrentUser, protected owner/System ACL,
file replacement nguyên tử, 128 KiB cap. Không credential trong SQLite/IPC/log/env sidecar.
Crate native-vault chứa FFI nhỏ; core/provider không dùng unsafe. Key RSA trong fixtures
chỉ ký token test, không tham gia live verification. HTTP refresh/revoke end-to-end
còn cần kiểm chứng. Catalog/probe/chat live đã pass
với profile hiện tại; không suy rộng eligibility cho mọi workspace/tài khoản.

Dùng tài liệu OpenAI Docs cho route đã đối chiếu; không đổi license/public repository.
R1B không sử dụng Codex app-server hay runtime agent khác.
