use crate::*;
use serde_json::json;

/// Deterministic documented fixtures; not a general language model.
#[derive(Default)]
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ModelProvider for MockProvider {
    async fn respond(
        &self,
        request: ModelRequest,
        events: mpsc::Sender<ModelDelta>,
        cancel: CancellationToken,
    ) -> Result<ModelResponse, ProviderError> {
        if cancel.is_cancelled() {
            return Err(ProviderError::cancelled());
        }
        let normalized = request
            .prompt
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let prompt = normalized.trim_end_matches(['.', '!', '?']);
        if prompt == "[fixture:timeout]" {
            return Err(ProviderError::timeout());
        }
        if prompt == "[fixture:429]" {
            return Err(ProviderError::new(
                "provider_rate_limit",
                "Nhà cung cấp đang giới hạn yêu cầu. Hãy thử lại sau.",
            ));
        }
        if prompt == "[fixture:401]" {
            return Err(ProviderError::new(
                "provider_auth",
                "Khóa API không hợp lệ hoặc không có quyền truy cập.",
            ));
        }
        if prompt == "[fixture:cancel]" {
            cancel.cancelled().await;
            return Err(ProviderError::cancelled());
        }
        let mut response = ModelResponse::default();
        let call = if prompt == "[fixture:invalid_json]" {
            Some(("design.propose_plate", "{\"width\":".to_string()))
        } else if prompt == "[fixture:forbidden_tool]" {
            Some(("system.read_secret", "{}".into()))
        } else if prompt == "[fixture:loop]" {
            Some(("cad.get_status", "{}".into()))
        } else if !request.tool_outputs.is_empty() {
            None
        } else if prompt == "vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm"
        {
            Some(("design.propose_plate", json!({"schema_version":1,"width":200.0,"height":120.0,"hole_diameter":10.0,"edge_offset":20.0,"origin":[0.0,0.0],"units":"mm","add_dimensions":true}).to_string()))
        } else if prompt == "tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép" {
            Some((
                "design.propose_plate_resize",
                json!({"width":240.0}).to_string(),
            ))
        } else {
            None
        };

        response.text = if let Some((name, arguments)) = call {
            // Error fixtures deliberately bypass this check so the runtime's
            // allowlist and argument validation are tested, rather than hidden.
            if request.tools.iter().any(|tool| tool.name == name) || prompt.starts_with("[fixture:")
            {
                response.tool_calls.push(ProviderToolCall {
                    call_id: format!("mock-call-{}", request.continuation.len()),
                    name: name.into(),
                    arguments,
                });
                "MÔ PHỎNG: đang đề xuất phương án để xem trước; chưa ghi CAD.".into()
            } else {
                "MÔ PHỎNG: chế độ hiện tại không cho phép đề xuất thay đổi CAD.".into()
            }
        } else if !request.tool_outputs.is_empty() {
            "MÔ PHỎNG: đã nhận kết quả công cụ. Mọi thay đổi cần được xem trước và duyệt.".into()
        } else {
            "MÔ PHỎNG: chỉ hỗ trợ hai câu mẫu tạo tấm đế 200 x 120 mm và tăng chiều dài lên 240 mm; đây không phải LLM tổng quát.".into()
        };
        for sentence in response.text.split_inclusive(' ') {
            emit(&events, sentence.to_owned(), &cancel).await?;
            tokio::task::yield_now().await;
        }
        response.continuation = request.continuation;
        response
            .continuation
            .push(json!({"type":"mock_checkpoint","call_count":response.tool_calls.len()}));
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(prompt: &str) -> ModelRequest {
        ModelRequest {
            model: "mock".into(),
            prompt: prompt.into(),
            context: json!({}),
            tools: vec![
                ModelTool {
                    name: "design.propose_plate".into(),
                    description: String::new(),
                    input_schema: json!({}),
                },
                ModelTool {
                    name: "design.propose_plate_resize".into(),
                    description: String::new(),
                    input_schema: json!({}),
                },
            ],
            continuation: vec![],
            tool_outputs: vec![],
        }
    }
    async fn respond(prompt: &str) -> Result<ModelResponse, ProviderError> {
        let (tx, _rx) = mpsc::channel(128);
        MockProvider::new()
            .respond(request(prompt), tx, CancellationToken::new())
            .await
    }
    #[tokio::test]
    async fn create_and_resize_are_deterministic_and_unknown_prompt_is_not_success() {
        let create =
            respond("Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm.")
                .await
                .unwrap();
        let args: Value = serde_json::from_str(&create.tool_calls[0].arguments).unwrap();
        assert_eq!(args["width"], 200.0);
        let resize = respond("Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép.")
            .await
            .unwrap();
        assert_eq!(resize.tool_calls[0].name, "design.propose_plate_resize");
        assert_eq!(respond("Vẽ tòa nhà").await.unwrap().tool_calls.len(), 0);
    }
    #[tokio::test]
    async fn hostile_fixtures_and_error_codes_are_real() {
        assert!(serde_json::from_str::<Value>(
            &respond("[fixture:invalid_json]").await.unwrap().tool_calls[0].arguments
        )
        .is_err());
        assert_eq!(
            respond("[fixture:forbidden_tool]")
                .await
                .unwrap()
                .tool_calls[0]
                .name,
            "system.read_secret"
        );
        assert_eq!(
            respond("[fixture:timeout]").await.err().unwrap().code,
            "provider_timeout"
        );
        assert_eq!(
            respond("[fixture:429]").await.err().unwrap().code,
            "provider_rate_limit"
        );
        let first = respond("[fixture:loop]").await.unwrap();
        let mut next = request("[fixture:loop]");
        next.continuation = first.continuation;
        next.tool_outputs.push(ToolOutput {
            call_id: first.tool_calls[0].call_id.clone(),
            output: "{}".into(),
        });
        let (tx, _rx) = mpsc::channel(128);
        assert_eq!(
            MockProvider::new()
                .respond(next, tx, CancellationToken::new())
                .await
                .unwrap()
                .tool_calls[0]
                .name,
            "cad.get_status"
        );
    }
    #[tokio::test]
    async fn cancellation_interrupts_wait_and_blocked_events() {
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        let (tx, _rx) = mpsc::channel(1);
        let pending = MockProvider::new();
        let future = pending.respond(request("[fixture:cancel]"), tx, cancel);
        let (_, result) = tokio::join!(
            async move {
                tokio::task::yield_now().await;
                trigger.cancel();
            },
            future
        );
        assert_eq!(result.err().unwrap().code, "provider_cancelled");
    }
}
