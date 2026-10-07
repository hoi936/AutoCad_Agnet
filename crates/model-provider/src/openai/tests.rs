use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

const KEY: &str = "sk-fixture-never-a-real-key-credential-sentinel";
fn request() -> ModelRequest {
    ModelRequest {
        model: "configured-model-fixture".into(),
        prompt: "Vẽ tấm đế".into(),
        context: json!({"revision":0,"entity_count":0}),
        tools: vec![
            ModelTool {
                name: "cad.get_status".into(),
                description: "Đọc trạng thái".into(),
                input_schema: json!({"type":"object","properties":{},"additionalProperties":false}),
            },
            ModelTool {
                name: "design.propose_plate_resize".into(),
                description: "Đề xuất resize".into(),
                input_schema: json!({"type":"object","properties":{"width":{"type":"number"}},"required":["width"],"additionalProperties":false}),
            },
        ],
        continuation: vec![],
        tool_outputs: vec![],
    }
}
fn event(value: Value) -> String {
    format!("data: {value}\r\n\r\n")
}
fn completed(output: Vec<Value>) -> String {
    event(json!({"type":"response.completed","response":{"status":"completed","output":output}}))
}
fn call(id: &str, name: &str, args: &str) -> Value {
    json!({"type":"function_call","id":format!("fc_{id}"),"call_id":id,"name":name,"arguments":args,"status":"completed"})
}
fn message(text: &str) -> Value {
    json!({"type":"message","id":"msg_1","role":"assistant","status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]})
}

async fn fixture(
    status: u16,
    body: String,
    delay: Duration,
    extra_headers: &str,
) -> (String, oneshot::Receiver<Value>) {
    fixture_media_type(
        status,
        body,
        delay,
        extra_headers,
        "Content-Type: text/event-stream\r\n",
    )
    .await
}
async fn fixture_media_type(
    status: u16,
    body: String,
    delay: Duration,
    extra_headers: &str,
    content_type: &str,
) -> (String, oneshot::Receiver<Value>) {
    let content_type = content_type.to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    let extra_headers = extra_headers.to_owned();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut chunk = [0; 4096];
        let body_start = loop {
            let count = socket.read(&mut chunk).await.unwrap();
            if count == 0 {
                return;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                break end + 4;
            }
        };
        let headers = String::from_utf8_lossy(&bytes[..body_start]);
        assert!(headers.starts_with("POST /v1/responses HTTP/1.1"));
        let length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|value| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        while bytes.len() < body_start + length {
            let count = socket.read(&mut chunk).await.unwrap();
            if count == 0 {
                return;
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
        let request: Value =
            serde_json::from_slice(&bytes[body_start..body_start + length]).unwrap();
        let _ = tx.send(request);
        tokio::time::sleep(delay).await;
        let response=format!("HTTP/1.1 {status} Fixture\r\n{content_type}Content-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n{body}",body.len());
        let _ = socket.write_all(response.as_bytes()).await;
    });
    (format!("http://{address}/v1/responses"), rx)
}
fn provider(endpoint: String) -> OpenAiProvider {
    let mut provider = OpenAiProvider::new(SecretValue::new(KEY.into()).unwrap()).unwrap();
    provider.client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .unwrap();
    provider.fixture_endpoint = Some(endpoint);
    provider
}
async fn run(
    provider: &OpenAiProvider,
    request: ModelRequest,
) -> (Result<ModelResponse, ProviderError>, Vec<String>) {
    let (tx, mut rx) = mpsc::channel(256);
    let result = provider
        .respond(request, tx, CancellationToken::new())
        .await;
    let mut text = vec![];
    while let Some(ModelDelta::Text(delta)) = rx.recv().await {
        text.push(delta);
    }
    (result, text)
}

#[tokio::test]
async fn http_stream_assembles_parallel_calls_and_replays_opaque_continuation_with_ids() {
    let reasoning = json!({"type":"reasoning","id":"rs_1","encrypted_content":"opaque-fixture-only","summary":[]});
    let a = call("one", "cad_get_status", "{}");
    let b = call("two", "design_propose_plate_resize", "{\"width\":240}");
    let mut body = event(json!({"type":"response.output_text.delta","delta":"Đề xuất."}));
    for (index, item) in [(1, a.clone()), (2, b.clone())] {
        let mut added = item;
        added["arguments"] = json!("");
        body +=
            &event(json!({"type":"response.output_item.added","output_index":index,"item":added}));
    }
    for (index, id, delta) in [
        (2, "fc_two", "{\"width\":"),
        (1, "fc_one", "{"),
        (2, "fc_two", "240}"),
        (1, "fc_one", "}"),
    ] {
        body += &event(
            json!({"type":"response.function_call_arguments.delta","output_index":index,"item_id":id,"delta":delta}),
        );
    }
    for (index, id, args) in [(1, "fc_one", "{}"), (2, "fc_two", "{\"width\":240}")] {
        body += &event(
            json!({"type":"response.function_call_arguments.done","output_index":index,"item_id":id,"arguments":args}),
        );
    }
    body += &completed(vec![reasoning.clone(), a, b, message("Đề xuất.")]);
    let (url, captured) = fixture(200, body, Duration::ZERO, "").await;
    let adapter = provider(url);
    let (result, deltas) = run(&adapter, request()).await;
    let result = result.unwrap();
    assert_eq!(deltas.concat(), "Đề xuất.");
    assert_eq!(result.tool_calls.len(), 2);
    assert_eq!(result.tool_calls[0].name, "cad.get_status");
    assert_eq!(result.tool_calls[1].call_id, "two");
    assert!(result.continuation.contains(&reasoning));
    assert!(!deltas.concat().contains("opaque"));
    let sent = captured.await.unwrap();
    assert_eq!(sent["store"], false);
    assert_eq!(sent["stream"], true);
    assert_eq!(sent["tools"][0]["name"], "cad_get_status");
    assert_eq!(sent["tools"][0]["strict"], true);
    assert!(!sent.to_string().contains(KEY));
    let mut next = request();
    next.continuation = result.continuation;
    next.tool_outputs = vec![
        ToolOutput {
            call_id: "one".into(),
            output: "{\"connected\":true}".into(),
        },
        ToolOutput {
            call_id: "two".into(),
            output: "{\"preview_id\":\"preview-1\"}".into(),
        },
    ];
    let (url, captured) = fixture(
        200,
        completed(vec![message("Chờ duyệt.")]),
        Duration::ZERO,
        "",
    )
    .await;
    let (next_result, _) = run(&provider(url), next).await;
    assert_eq!(next_result.unwrap().text, "Chờ duyệt.");
    let sent = captured.await.unwrap();
    let input = sent["input"].as_array().unwrap();
    assert!(input.contains(&reasoning));
    assert_eq!(input[input.len() - 2]["call_id"], "one");
    assert_eq!(input[input.len() - 1]["call_id"], "two");
}

#[tokio::test]
async fn http_errors_are_classified_without_response_bodies_or_redirects() {
    for (status, code) in [
        (401, "provider_auth"),
        (403, "provider_auth"),
        (429, "provider_rate_limit"),
        (503, "provider_unavailable"),
        (307, "provider_redirect_blocked"),
    ] {
        let (url, _) = fixture(
            status,
            format!("untrusted server body contains {KEY}"),
            Duration::ZERO,
            "Location: http://127.0.0.1:1/steal\r\n",
        )
        .await;
        let error = run(&provider(url), request()).await.0.err().unwrap();
        assert_eq!(error.code, code);
        assert!(!format!("{error:?}").contains(KEY));
    }
}

#[tokio::test]
async fn timeout_and_cancellation_interrupt_real_http_wait() {
    let (url, _) = fixture(200, completed(vec![]), Duration::from_secs(2), "").await;
    let adapter = provider(url).with_limits(ProviderLimits {
        timeout: Duration::from_millis(20),
        ..ProviderLimits::default()
    });
    assert_eq!(
        run(&adapter, request()).await.0.err().unwrap().code,
        "provider_timeout"
    );
    let (url, captured) = fixture(200, completed(vec![]), Duration::from_secs(2), "").await;
    let adapter = provider(url);
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let (tx, _rx) = mpsc::channel(1);
    let pending = adapter.respond(request(), tx, cancel);
    let (_, result) = tokio::join!(
        async move {
            captured.await.unwrap();
            trigger.cancel();
        },
        pending
    );
    assert_eq!(result.err().unwrap().code, "provider_cancelled");
}

#[tokio::test]
async fn invalid_arguments_forbidden_tool_and_truncated_stream_never_return_calls() {
    for (body, code) in [
        (
            completed(vec![call("one", "cad_get_status", "{\"incomplete\":")]),
            "provider_invalid_arguments",
        ),
        (
            completed(vec![call("one", "cad_apply_change_set", "{}")]),
            "provider_forbidden_tool",
        ),
        (
            event(json!({"type":"response.output_text.delta","delta":"Đang"})),
            "provider_stream_interrupted",
        ),
        ("data: {incomplete}\n\n".into(), "provider_sse_invalid"),
        (
            event(json!({"type":"response.incomplete"})),
            "provider_incomplete",
        ),
    ] {
        let (url, _) = fixture(200, body, Duration::ZERO, "").await;
        assert_eq!(
            run(&provider(url), request()).await.0.err().unwrap().code,
            code
        );
    }
}

#[tokio::test]
async fn reflected_key_is_scrubbed_across_fragments_and_never_continued() {
    let split = KEY.len() / 2;
    let body = event(
        json!({"type":"response.output_text.delta","delta":format!("Không tiết lộ {}",&KEY[..split])}),
    ) + &event(
        json!({"type":"response.output_text.delta","delta":format!("{}.",&KEY[split..])}),
    ) + &completed(vec![message(&format!("Không tiết lộ {KEY}."))]);
    let (url, _) = fixture(200, body, Duration::ZERO, "").await;
    let (result, deltas) = run(&provider(url), request()).await;
    assert_eq!(result.err().unwrap().code, "provider_secret_reflection");
    assert!(!deltas.concat().contains(KEY));
    assert!(deltas.concat().contains("[ĐÃ ẨN KHÓA]"));
}

#[tokio::test]
async fn hostile_request_and_size_caps_fail_closed() {
    let adapter = OpenAiProvider::new(SecretValue::new(KEY.into()).unwrap()).unwrap();
    let mut bad = request();
    bad.tools[0].name = "shell.exec".into();
    assert_eq!(
        adapter.request_body(&bad).err().unwrap().code,
        "provider_forbidden_tool"
    );
    let mut bad = request();
    bad.prompt = KEY.into();
    assert_eq!(
        adapter.request_body(&bad).err().unwrap().code,
        "provider_secret_in_input"
    );
    let mut bad = request();
    bad.tool_outputs.push(ToolOutput {
        call_id: "invented".into(),
        output: "{}".into(),
    });
    assert!(adapter.request_body(&bad).is_err());
    let body = completed(vec![message(&"a".repeat(500))]);
    let (url, _) = fixture(200, body, Duration::ZERO, "").await;
    let limited = provider(url).with_limits(ProviderLimits {
        max_event_bytes: 64,
        ..ProviderLimits::default()
    });
    assert_eq!(
        run(&limited, request()).await.0.err().unwrap().code,
        "provider_limit"
    );
}

#[test]
fn sse_decoder_preserves_utf8_and_multiline_data_at_every_byte_boundary() {
    let raw="event: ignored\r\ndata: {\"type\":\"response.output_text.delta\",\r\ndata: \"delta\":\"Tấm đế\"}\r\n\r\n";
    let mut decoder = SseDecoder::default();
    let mut values = vec![];
    for byte in raw.as_bytes() {
        values.extend(decoder.feed(&[*byte], &ProviderLimits::default()).unwrap());
    }
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["delta"], "Tấm đế");
    let mut decoder = SseDecoder::default();
    let limits = ProviderLimits {
        max_response_bytes: 3,
        ..ProviderLimits::default()
    };
    assert_eq!(
        decoder.feed(b"data", &limits).err().unwrap().code,
        "provider_limit"
    );
}

#[test]
fn tool_mapping_is_bijective_and_optional_schema_fields_become_nullable() {
    assert_eq!(
        TOOL_NAMES
            .iter()
            .map(|(name, _)| name)
            .collect::<HashSet<_>>()
            .len(),
        TOOL_NAMES.len()
    );
    assert_eq!(
        TOOL_NAMES
            .iter()
            .map(|(_, wire)| wire)
            .collect::<HashSet<_>>()
            .len(),
        TOOL_NAMES.len()
    );
    let mut schema =
        json!({"type":"object","properties":{"operation_id":{"type":"string"}},"required":[]});
    strict_schema(&mut schema).unwrap();
    assert_eq!(schema["required"], json!(["operation_id"]));
    assert_eq!(
        schema["properties"]["operation_id"]["anyOf"][1]["type"],
        "null"
    );
}

#[tokio::test]
async fn missing_content_type_requires_valid_completed_sse() {
    let (endpoint, _) = fixture_media_type(
        200,
        completed(vec![message("Hello")]),
        Duration::ZERO,
        "",
        "",
    )
    .await;
    let (result, _) = run(&provider(endpoint), request()).await;
    assert_eq!(result.unwrap().text, "Hello");
    for body in [
        "<html>not an event stream</html>".to_owned(),
        event(json!({"type":"response.output_text.delta","delta":"unfinished"})),
    ] {
        let (endpoint, _) = fixture_media_type(200, body, Duration::ZERO, "", "").await;
        assert!(run(&provider(endpoint), request()).await.0.is_err());
    }
    let (endpoint, _) = fixture_media_type(
        200,
        completed(vec![message("Hello")]),
        Duration::ZERO,
        "",
        "Content-Type: text/html\r\n",
    )
    .await;
    assert_eq!(
        run(&provider(endpoint), request())
            .await
            .0
            .err()
            .unwrap()
            .code,
        "provider_content_type_invalid"
    );
}

#[tokio::test]
async fn reconciles_call_by_item_id_when_final_output_indices_shift() {
    let item = call("one", "cad_get_status", "{}");
    let body =
        event(json!({"type":"response.output_item.added","output_index":1,"item":item.clone()}))
            + &completed(vec![item.clone()]);
    let (endpoint, _) = fixture(200, body, Duration::ZERO, "").await;
    let result = run(&provider(endpoint), request()).await.0.unwrap();
    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(result.tool_calls[0].call_id, "one");
    let body = event(json!({"type":"response.output_item.added","output_index":1,"item":item}))
        + &completed(vec![call("other", "cad_get_status", "{}")]);
    let (endpoint, _) = fixture(200, body, Duration::ZERO, "").await;
    assert_eq!(
        run(&provider(endpoint), request())
            .await
            .0
            .err()
            .unwrap()
            .code,
        "provider_partial_call_missing"
    );
}

#[tokio::test]
async fn compact_terminal_event_uses_completed_items_but_never_partial_or_interrupted_items() {
    let item = call("compact", "cad_get_status", "{}");
    let added =
        event(json!({"type":"response.output_item.added","output_index":0,"item":item.clone()}));
    let done =
        event(json!({"type":"response.output_item.done","output_index":0,"item":item.clone()}));
    let (endpoint, _) = fixture(
        200,
        added.clone() + &done + &completed(vec![]),
        Duration::ZERO,
        "",
    )
    .await;
    let result = run(&provider(endpoint), request()).await.0.unwrap();
    assert_eq!(result.tool_calls[0].call_id, "compact");
    assert!(result.continuation.contains(&item));
    for body in [
        added.clone() + &done,
        added + &completed(vec![]),
        done.clone() + &done + &completed(vec![]),
        done + &completed(vec![call("different", "cad_get_status", "{}")]),
    ] {
        let (endpoint, _) = fixture(200, body, Duration::ZERO, "").await;
        assert!(run(&provider(endpoint), request()).await.0.is_err());
    }
    let text = message("Finished");
    let body = event(json!({"type":"response.output_text.delta","delta":"Finished"}))
        + &event(json!({"type":"response.output_item.done","output_index":0,"item":text}))
        + &completed(vec![]);
    let (endpoint, _) = fixture(200, body, Duration::ZERO, "").await;
    assert_eq!(
        run(&provider(endpoint), request()).await.0.unwrap().text,
        "Finished"
    );
}
