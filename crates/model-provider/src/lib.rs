#![forbid(unsafe_code)]
//! Backend-only model adapters. No application database or frontend secret API.
mod mock;
pub mod oauth;
mod openai;
mod secret;

pub use mock::MockProvider;
pub use openai::{ModelEntry, OpenAiProvider, ProviderLimits, OPENAI_RESPONSES_ENDPOINT};
#[cfg(any(test, feature = "test-support"))]
pub use secret::MemorySecretStore;
pub use secret::{
    SecretError, SecretStore, SecretValue, SessionSecretStore, WindowsCredentialStore,
};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ModelRequest {
    pub model: String,
    pub prompt: String,
    pub context: Value,
    pub tools: Vec<ModelTool>,
    /// Complete opaque history returned by the preceding response. Never publish
    /// this field as UI events or reasoning text.
    pub continuation: Vec<Value>,
    pub tool_outputs: Vec<ToolOutput>,
}

#[derive(Clone)]
pub struct ModelTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone)]
pub struct ToolOutput {
    pub call_id: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Default)]
pub struct ModelResponse {
    pub text: String,
    pub tool_calls: Vec<ProviderToolCall>,
    pub continuation: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelDelta {
    Text(String),
}

/// Error strings are constant, bounded, and never include HTTP bodies or keys.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ProviderError {
    pub code: &'static str,
    pub message: &'static str,
}

impl ProviderError {
    pub(crate) const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
    pub(crate) fn cancelled() -> Self {
        Self::new("provider_cancelled", "Đã dừng yêu cầu mô hình.")
    }
    pub(crate) fn timeout() -> Self {
        Self::new("provider_timeout", "Nhà cung cấp phản hồi quá thời hạn.")
    }
    pub(crate) fn invalid() -> Self {
        Self::new(
            "provider_invalid_response",
            "Phản hồi nhà cung cấp không hợp lệ hoặc chưa hoàn tất.",
        )
    }
    pub(crate) fn limit() -> Self {
        Self::new(
            "provider_limit",
            "Yêu cầu hoặc phản hồi vượt giới hạn cho phép.",
        )
    }
}

#[async_trait]
// async-trait emits must_use on its boxed Future (Rust 1.99 Clippy).
#[allow(clippy::double_must_use)]
pub trait ModelProvider: Send + Sync {
    async fn respond(
        &self,
        request: ModelRequest,
        events: mpsc::Sender<ModelDelta>,
        cancel: CancellationToken,
    ) -> Result<ModelResponse, ProviderError>;

    fn redact(&self, text: &str) -> String {
        redact_sensitive_text(text)
    }
}

/// Conservative token scrubber for accidental pasted API keys. Applications
/// should also avoid persisting untrusted prompts containing credentials.
pub fn redact_sensitive_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find("sk-") {
        result.push_str(&remaining[..start]);
        let token = &remaining[start..];
        let end = token
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(token.len());
        result.push_str("[ĐÃ ẨN KHÓA]");
        remaining = &token[end..];
    }
    result.push_str(remaining);
    result
}

pub(crate) async fn emit(
    events: &mpsc::Sender<ModelDelta>,
    text: String,
    cancel: &CancellationToken,
) -> Result<(), ProviderError> {
    if text.is_empty() {
        return Ok(());
    }
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ProviderError::cancelled()),
        result = events.send(ModelDelta::Text(text)) => result.map_err(|_| ProviderError::cancelled()),
    }
}
