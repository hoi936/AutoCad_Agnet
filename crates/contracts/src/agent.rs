use crate::{ChangeSet, RunMode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub title: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub run_id: Option<String>,
    pub role: String,
    pub content: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: String,
    pub conversation_id: String,
    pub project_id: String,
    pub mode: RunMode,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub error_code: Option<String>,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub finished_at: Option<i64>,
}

#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub run_id: String,
    #[ts(type = "number")]
    pub sequence: i64,
    pub kind: String,
    pub text: Option<String>,
    pub tool_name: Option<String>,
    pub change_set_id: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    pub conversation: Conversation,
    pub messages: Vec<ChatMessage>,
    pub runs: Vec<AgentRun>,
}

#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct RunSnapshot {
    pub run: AgentRun,
    pub events: Vec<RunEvent>,
    pub change_sets: Vec<ChangeSet>,
}

/// This is metadata only. `configured` is filled by the desktop credential service.
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub project_id: String,
    pub provider: String,
    pub model: String,
    pub secret_ref: Option<String>,
    pub cloud_consent: bool,
    pub configured: bool,
    pub storage: String,
}

pub fn agent_typescript_declarations(config: &ts_rs::Config) -> Vec<String> {
    vec![
        Conversation::decl(config),
        ChatMessage::decl(config),
        AgentRun::decl(config),
        RunEvent::decl(config),
        ConversationSnapshot::decl(config),
        RunSnapshot::decl(config),
        ProviderConfig::decl(config),
    ]
}
