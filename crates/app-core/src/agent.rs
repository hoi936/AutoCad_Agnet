//! Bounded agent orchestration. Only this process writes application SQLite.
//! The model can read CAD context or create proposals; approval is a UI service.
use super::*;
use cad_model_provider::{
    ModelDelta, ModelProvider, ModelRequest, ModelResponse, ModelTool, ToolOutput,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_PROMPT: usize = 8_000;
const MAX_TEXT: usize = 65_536;
const MAX_ARGUMENTS: usize = 16_384;
const MAX_TOOLS: usize = 12;
const MAX_REPAIRS: usize = 2;
const MAX_EVENTS: usize = 256;
const RUN_TIMEOUT: Duration = Duration::from_secs(120);

fn conversation_row(row: sqlx::sqlite::SqliteRow) -> Conversation {
    Conversation {
        id: row.get("id"),
        project_id: row.get("project_id"),
        title: row.get("title"),
        created_at: row.get("created_at"),
    }
}
fn run_row(row: sqlx::sqlite::SqliteRow) -> Result<AgentRun> {
    let mode: String = row.get("mode");
    Ok(AgentRun {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        project_id: row.get("project_id"),
        mode: decode(&format!("\"{mode}\""))?,
        provider: row.get("provider"),
        model: row.get("model"),
        status: row.get("status"),
        error_code: row.get("error_code"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
    })
}

impl Studio {
    pub async fn conversations_create(
        &self,
        project_id: &str,
        title: String,
    ) -> Result<Conversation> {
        self.ensure_project(project_id).await?;
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 120 {
            return Err(AppError::new(
                "INVALID_TITLE",
                "Tên hội thoại phải có 1–120 ký tự.",
            ));
        }
        let conversation = Conversation {
            id: self.next_id(),
            project_id: project_id.into(),
            title: cad_model_provider::redact_sensitive_text(title),
            created_at: self.clock.now(),
        };
        let _guard = self.writer.lock().await;
        sqlx::query("INSERT INTO conversations(id,project_id,title,created_at) VALUES(?,?,?,?)")
            .bind(&conversation.id)
            .bind(project_id)
            .bind(&conversation.title)
            .bind(conversation.created_at)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        Ok(conversation)
    }

    async fn ensure_project(&self, project_id: &str) -> Result<()> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)")
            .bind(project_id)
            .fetch_one(&self.pool)
            .await
            .map_err(internal)?;
        if exists {
            Ok(())
        } else {
            Err(AppError::new("NOT_FOUND", "Không tìm thấy dự án."))
        }
    }

    pub async fn conversations_list(&self, project_id: &str) -> Result<Vec<Conversation>> {
        self.ensure_project(project_id).await?;
        Ok(sqlx::query("SELECT * FROM conversations WHERE project_id=? ORDER BY created_at DESC,rowid DESC LIMIT 100")
            .bind(project_id).fetch_all(&self.pool).await.map_err(internal)?.into_iter().map(conversation_row).collect())
    }

    async fn conversation(&self, conversation_id: &str) -> Result<Conversation> {
        sqlx::query("SELECT * FROM conversations WHERE id=?")
            .bind(conversation_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?
            .map(conversation_row)
            .ok_or_else(|| AppError::new("NOT_FOUND", "Không tìm thấy hội thoại."))
    }

    pub async fn conversation_snapshot(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationSnapshot> {
        let conversation = self.conversation(conversation_id).await?;
        let rows = sqlx::query("SELECT * FROM (SELECT rowid AS ordering,* FROM messages WHERE conversation_id=? ORDER BY rowid DESC LIMIT 200) ORDER BY ordering")
            .bind(conversation_id).fetch_all(&self.pool).await.map_err(internal)?;
        let messages = rows
            .into_iter()
            .map(|row| ChatMessage {
                id: row.get("id"),
                conversation_id: row.get("conversation_id"),
                run_id: row.get("run_id"),
                role: row.get("role"),
                content: row.get("content"),
                created_at: row.get("created_at"),
            })
            .collect();
        let runs = sqlx::query("SELECT r.*,c.project_id FROM agent_runs r JOIN conversations c ON c.id=r.conversation_id WHERE r.conversation_id=? ORDER BY r.rowid DESC LIMIT 100")
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(internal)?
        .into_iter()
        .map(run_row)
        .collect::<Result<Vec<_>>>()?;
        Ok(ConversationSnapshot {
            conversation,
            messages,
            runs,
        })
    }

    pub async fn provider_get_config(&self, project_id: &str) -> Result<ProviderConfig> {
        self.ensure_project(project_id).await?;
        let row = sqlx::query("SELECT * FROM provider_configs WHERE project_id=?")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?;
        Ok(match row {
            Some(row) => ProviderConfig {
                project_id: project_id.into(),
                provider: row.get("provider"),
                model: row.get("model"),
                secret_ref: row.get("secret_ref"),
                cloud_consent: row.get("cloud_consent"),
                configured: false,
                storage: row.get("storage"),
            },
            None => ProviderConfig {
                project_id: project_id.into(),
                provider: "mock".into(),
                model: "mock-plate-v1".into(),
                secret_ref: None,
                cloud_consent: false,
                configured: true,
                storage: "session".into(),
            },
        })
    }

    pub async fn provider_save_config(
        &self,
        project_id: &str,
        provider: &str,
        model: &str,
        cloud_consent: bool,
        storage: &str,
    ) -> Result<ProviderConfig> {
        self.ensure_project(project_id).await?;
        if !["mock", "openai"].contains(&provider) || !["windows", "session"].contains(&storage) {
            return Err(AppError::new(
                "INVALID_PROVIDER",
                "Provider hoặc nơi lưu credential không được hỗ trợ.",
            ));
        }
        let model = if provider == "mock" {
            "mock-plate-v1"
        } else {
            model.trim()
        };
        if model.is_empty()
            || model.len() > 120
            || model.contains("sk-")
            || !model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.:".contains(c))
        {
            return Err(AppError::new(
                "INVALID_MODEL",
                "Nhập model ID hợp lệ trong Settings; không nhập API key vào trường model.",
            ));
        }
        let secret_ref = (provider == "openai")
            .then(|| format!("cad-agent-studio:{project_id}:openai:{storage}"));
        let _guard = self.writer.lock().await;
        let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_runs r JOIN conversations c ON c.id=r.conversation_id WHERE c.project_id=? AND r.status='running')")
            .bind(project_id).fetch_one(&self.pool).await.map_err(internal)?;
        if busy {
            return Err(AppError::new(
                "RUN_ACTIVE",
                "Hãy dừng lượt chạy trước khi đổi provider.",
            ));
        }
        let mut tx = self.pool.begin().await.map_err(internal)?;
        sqlx::query("INSERT OR IGNORE INTO proposal_invalidations SELECT c.id,'provider_changed' FROM change_sets c JOIN documents d ON d.id=c.document_id WHERE d.project_id=? AND c.status IN ('pending_approval','planned') AND NOT EXISTS(SELECT 1 FROM gateway_operations g WHERE g.change_set_id=c.id AND g.state!='verified')")
            .bind(project_id).execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("UPDATE change_sets SET status='rejected' WHERE id IN (SELECT change_set_id FROM proposal_invalidations) AND status IN ('pending_approval','planned')")
            .execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO provider_configs(project_id,provider,model,secret_ref,cloud_consent,storage,updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(project_id) DO UPDATE SET provider=excluded.provider,model=excluded.model,secret_ref=excluded.secret_ref,cloud_consent=excluded.cloud_consent,storage=excluded.storage,updated_at=excluded.updated_at")
            .bind(project_id).bind(provider).bind(model).bind(secret_ref).bind(provider == "openai" && cloud_consent).bind(storage).bind(self.clock.now())
            .execute(&mut *tx).await.map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        self.provider_get_config(project_id).await
    }

    /// Call once when opening a database, before admitting new runs. Never replay work.
    pub async fn agent_recover_interrupted(&self) -> Result<()> {
        let _guard = self.writer.lock().await;
        let ids: Vec<(String, String)> =
            sqlx::query_as("SELECT id,conversation_id FROM agent_runs WHERE status='running'")
                .fetch_all(&self.pool)
                .await
                .map_err(internal)?;
        let mut tx = self.pool.begin().await.map_err(internal)?;
        for (run_id, conversation_id) in ids {
            sqlx::query("UPDATE agent_runs SET status='interrupted',error_code='APP_RESTARTED',finished_at=? WHERE id=?")
                .bind(self.clock.now()).bind(&run_id).execute(&mut *tx).await.map_err(internal)?;
            sqlx::query("INSERT INTO messages(id,conversation_id,run_id,role,content,created_at) VALUES(?,?,?,'system',?,?)")
                .bind(self.next_id()).bind(conversation_id).bind(run_id)
                .bind("Lượt trước bị gián đoạn khi đóng ứng dụng. Không tự chạy lại; hãy kiểm tra các phương án đã lưu.")
                .bind(self.clock.now()).execute(&mut *tx).await.map_err(internal)?;
        }
        tx.commit().await.map_err(internal)
    }

    async fn agent_run(&self, run_id: &str) -> Result<AgentRun> {
        run_row(
            sqlx::query("SELECT r.*,c.project_id FROM agent_runs r JOIN conversations c ON c.id=r.conversation_id WHERE r.id=?")
                .bind(run_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(internal)?
                .ok_or_else(|| AppError::new("NOT_FOUND", "Không tìm thấy lượt chạy."))?,
        )
    }
}

struct ActiveRun {
    cancel: CancellationToken,
    sequence: i64,
    events: Vec<RunEvent>,
}

pub struct AgentService {
    studio: Arc<Studio>,
    active: Mutex<HashMap<String, ActiveRun>>,
}

impl AgentService {
    pub fn new(studio: Arc<Studio>) -> Self {
        Self {
            studio,
            active: Mutex::new(HashMap::new()),
        }
    }

    /// The desktop chooses this provider from the persisted config and credential store.
    /// Injection is backend-only; the UI cannot supply an arbitrary provider or tool list.
    pub async fn start_run(
        self: &Arc<Self>,
        conversation_id: &str,
        content: String,
        mode: RunMode,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<AgentRun> {
        let content = content.trim();
        if content.is_empty() || content.len() > MAX_PROMPT {
            return Err(AppError::new(
                "INVALID_MESSAGE",
                "Nội dung phải có 1–8.000 byte.",
            ));
        }
        let content = provider.redact(content);
        let conversation = self.studio.conversation(conversation_id).await?;
        let config = self
            .studio
            .provider_get_config(&conversation.project_id)
            .await?;
        if config.provider == "openai" && !config.cloud_consent {
            return Err(AppError::new(
                "CLOUD_CONSENT_REQUIRED",
                "Cần đồng ý gửi nội dung và ngữ cảnh CAD tối thiểu tới provider cho dự án này.",
            ));
        }
        let document_id: String = sqlx::query_scalar("SELECT id FROM documents WHERE project_id=?")
            .bind(&conversation.project_id)
            .fetch_one(&self.studio.pool)
            .await
            .map_err(internal)?;
        let run = AgentRun {
            id: self.studio.next_id(),
            conversation_id: conversation_id.into(),
            project_id: conversation.project_id,
            mode,
            provider: config.provider,
            model: config.model,
            status: "running".into(),
            error_code: None,
            started_at: self.studio.clock.now(),
            finished_at: None,
        };
        let cancel = CancellationToken::new();
        {
            let _guard = self.studio.writer.lock().await;
            let busy: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM agent_runs WHERE document_id=? AND status='running')",
            )
            .bind(&document_id)
            .fetch_one(&self.studio.pool)
            .await
            .map_err(internal)?;
            if busy {
                return Err(AppError::new(
                    "RUN_ACTIVE",
                    "Hội thoại đang có một lượt chạy; hãy dừng hoặc chờ lượt đó kết thúc.",
                ));
            }
            let mut tx = self.studio.pool.begin().await.map_err(internal)?;
            sqlx::query("INSERT OR IGNORE INTO proposal_invalidations SELECT id,'new_request' FROM change_sets WHERE document_id=? AND status IN ('pending_approval','planned') AND NOT EXISTS(SELECT 1 FROM gateway_operations g WHERE g.change_set_id=change_sets.id AND g.state!='verified')")
                .bind(&document_id).execute(&mut *tx).await.map_err(internal)?;
            sqlx::query("UPDATE change_sets SET status='rejected' WHERE document_id=? AND status IN ('pending_approval','planned') AND id IN (SELECT change_set_id FROM proposal_invalidations)")
                .bind(&document_id).execute(&mut *tx).await.map_err(internal)?;
            sqlx::query("INSERT INTO agent_runs(id,conversation_id,document_id,provider_config_id,mode,provider,model,status,started_at,limits_json) VALUES(?,?,?,(SELECT project_id FROM provider_configs WHERE project_id=?),?,?,?,'running',?,?)")
                .bind(&run.id).bind(conversation_id).bind(document_id).bind(&run.project_id)
                .bind(encode(&run.mode)?.trim_matches('"')).bind(&run.provider).bind(&run.model).bind(run.started_at)
                .bind(r#"{"tool_calls":12,"argument_repairs":2,"timeout_seconds":120,"max_text_bytes":65536}"#)
                .execute(&mut *tx).await.map_err(internal)?;
            sqlx::query("INSERT INTO messages(id,conversation_id,run_id,role,content,created_at) VALUES(?,?,?,'user',?,?)")
                .bind(self.studio.next_id()).bind(conversation_id).bind(&run.id).bind(&content).bind(run.started_at).execute(&mut *tx).await.map_err(internal)?;
            tx.commit().await.map_err(internal)?;
        }
        self.active.lock().await.insert(
            run.id.clone(),
            ActiveRun {
                cancel: cancel.clone(),
                sequence: 0,
                events: Vec::new(),
            },
        );
        self.event(
            &run.id,
            "run.started",
            Some("Đang lấy ngữ cảnh CAD và xử lý yêu cầu.".into()),
            None,
            None,
        )
        .await?;
        let service = self.clone();
        let running = run.clone();
        tokio::spawn(async move {
            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(AppError::new("RUN_CANCELLED", "Đã dừng lượt chạy. Không có thao tác ghi CAD nào được agent gửi.")),
                result = tokio::time::timeout(RUN_TIMEOUT, service.run_loop(&running, content, provider, cancel.clone())) => result.unwrap_or_else(|_| Err(AppError::new("RUN_TIMEOUT", "Lượt chạy đã vượt giới hạn 120 giây."))),
            };
            // Persist a terminal state before dropping the in-memory stream buffer.
            let (status, text, error) = match result {
                Ok((status, text)) => (status, text, None),
                Err(error) => (
                    if error.code == "RUN_CANCELLED" || error.code == "provider_cancelled" {
                        "cancelled"
                    } else {
                        "error"
                    }
                    .into(),
                    error.message,
                    Some(error.code),
                ),
            };
            if service
                .finish(&running, &status, &text, error.as_deref())
                .await
                .is_ok()
            {
                service.active.lock().await.remove(&running.id);
            }
        });
        Ok(run)
    }

    pub async fn cancel(&self, run_id: &str) -> Result<()> {
        self.studio.agent_run(run_id).await?;
        if let Some(active) = self.active.lock().await.get(run_id) {
            active.cancel.cancel();
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        for active in self.active.lock().await.values() {
            active.cancel.cancel();
        }
    }

    pub async fn snapshot(&self, run_id: &str) -> Result<RunSnapshot> {
        let run = self.studio.agent_run(run_id).await?;
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT safe_payload_json FROM run_events WHERE run_id=? ORDER BY sequence LIMIT 256",
        )
        .bind(run_id)
        .fetch_all(&self.studio.pool)
        .await
        .map_err(internal)?;
        let mut events = BTreeMap::new();
        for row in rows {
            let event: RunEvent = decode(&row)?;
            events.insert(event.sequence, event);
        }
        if let Some(active) = self.active.lock().await.get(run_id) {
            for event in &active.events {
                events.insert(event.sequence, event.clone());
            }
        }
        let ids: Vec<String> =
            sqlx::query_scalar("SELECT change_set_id FROM run_change_sets WHERE run_id=?")
                .bind(run_id)
                .fetch_all(&self.studio.pool)
                .await
                .map_err(internal)?;
        let mut change_sets = Vec::new();
        for id in ids {
            change_sets.push(self.studio.change(&id).await?);
        }
        Ok(RunSnapshot {
            run,
            events: events.into_values().collect(),
            change_sets,
        })
    }

    async fn event(
        &self,
        run_id: &str,
        kind: &str,
        text: Option<String>,
        tool_name: Option<String>,
        change_set_id: Option<String>,
    ) -> Result<()> {
        let event = {
            let mut active = self.active.lock().await;
            let control = active
                .get_mut(run_id)
                .ok_or_else(|| AppError::new("RUN_NOT_ACTIVE", "Lượt chạy không còn hoạt động."))?;
            // Streaming checkpoints live only in memory. Milestones and final message survive restart.
            if kind == "text.delta" && control.events.len() >= MAX_EVENTS - 64 {
                return Ok(());
            }
            control.sequence += 1;
            let event = RunEvent {
                run_id: run_id.into(),
                sequence: control.sequence,
                kind: kind.into(),
                text,
                tool_name,
                change_set_id,
                created_at: self.studio.clock.now(),
            };
            control.events.push(event.clone());
            event
        };
        if kind != "text.delta" {
            sqlx::query("INSERT INTO run_events(run_id,sequence,kind,safe_payload_json,created_at) VALUES(?,?,?,?,?)")
                .bind(run_id).bind(event.sequence).bind(kind).bind(encode(&event)?).bind(event.created_at)
                .execute(&self.studio.pool).await.map_err(internal)?;
        }
        Ok(())
    }

    async fn finish(
        &self,
        run: &AgentRun,
        status: &str,
        text: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let _guard = self.studio.writer.lock().await;
        if self.studio.agent_run(&run.id).await?.status != "running" {
            // A user may approve a persisted preview while the final provider turn ends.
            // Preserve the observed CAD result instead of regressing to awaiting approval.
            return Ok(());
        }
        let kind = if status == "cancelled" {
            "run.cancelled"
        } else if status == "error" {
            "run.error"
        } else {
            "run.completed"
        };
        self.event(&run.id, kind, Some(text.into()), None, None)
            .await?;
        let mut tx = self.studio.pool.begin().await.map_err(internal)?;
        sqlx::query("UPDATE agent_runs SET status=?,error_code=?,finished_at=? WHERE id=? AND status='running'")
            .bind(status).bind(error).bind(self.studio.clock.now()).bind(&run.id).execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("UPDATE tool_calls SET status='failed',finished_at=? WHERE run_id=? AND status='started'")
            .bind(self.studio.clock.now()).bind(&run.id).execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO messages(id,conversation_id,run_id,role,content,created_at) VALUES(?,?,?,'assistant',?,?)")
            .bind(self.studio.next_id()).bind(&run.conversation_id).bind(&run.id).bind(text).bind(self.studio.clock.now()).execute(&mut *tx).await.map_err(internal)?;
        tx.commit().await.map_err(internal)
    }

    /// Called by the existing approval/rejection service after the actual result is known.
    /// This method never approves or applies a change.
    pub async fn change_finished(&self, change_id: &str, outcome: &str) -> Result<()> {
        let status = match outcome {
            "verified" | "rejected" | "outcome_unknown" => outcome,
            _ => {
                return Err(AppError::new(
                    "INVALID_OUTCOME",
                    "Kết quả thay đổi không hợp lệ.",
                ))
            }
        };
        let link: Option<(String, String)> = sqlx::query_as("SELECT r.id,r.conversation_id FROM run_change_sets l JOIN agent_runs r ON r.id=l.run_id WHERE l.change_set_id=?")
            .bind(change_id).fetch_optional(&self.studio.pool).await.map_err(internal)?;
        let Some((run_id, conversation_id)) = link else {
            return Ok(());
        };
        let actual = self.studio.change(change_id).await?;
        if status != "outcome_unknown" && actual.status != status {
            return Err(AppError::new(
                "OUTCOME_MISMATCH",
                "Kết quả hội thoại không khớp trạng thái CAD đã lưu.",
            ));
        }
        let text = match status { "verified" => "Thay đổi đã được người dùng duyệt, áp dụng và đọc lại/xác minh. CAD đang ở chế độ mô phỏng; chưa ghi DWG.", "rejected" => "Người dùng đã từ chối phương án. Không có thay đổi CAD từ phương án này.", _ => "Chưa xác định kết quả thao tác. Cần đối chiếu operation status/readback; không tự chạy lại." };
        let _guard = self.studio.writer.lock().await;
        let mut tx = self.studio.pool.begin().await.map_err(internal)?;
        let current: String = sqlx::query_scalar("SELECT status FROM agent_runs WHERE id=?")
            .bind(&run_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;
        if current == status {
            return Ok(());
        }
        let mut sequence: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0)+1 FROM run_events WHERE run_id=?")
                .bind(&run_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(internal)?;
        if let Some(active) = self.active.lock().await.get_mut(&run_id) {
            sequence = sequence.max(active.sequence + 1);
            active.sequence = sequence;
        }
        let event = RunEvent {
            run_id: run_id.clone(),
            sequence,
            kind: "change.finished".into(),
            text: Some(text.into()),
            tool_name: None,
            change_set_id: Some(change_id.into()),
            created_at: self.studio.clock.now(),
        };
        sqlx::query("UPDATE agent_runs SET status=?,finished_at=? WHERE id=?")
            .bind(status)
            .bind(self.studio.clock.now())
            .bind(&run_id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("INSERT INTO run_events(run_id,sequence,kind,safe_payload_json,created_at) VALUES(?,?,?,?,?)").bind(&run_id).bind(sequence).bind(&event.kind).bind(encode(&event)?).bind(event.created_at).execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO messages(id,conversation_id,run_id,role,content,created_at) VALUES(?,?,?,'system',?,?)").bind(self.studio.next_id()).bind(conversation_id).bind(&run_id).bind(text).bind(self.studio.clock.now()).execute(&mut *tx).await.map_err(internal)?;
        tx.commit().await.map_err(internal)
    }

    async fn provider_turn(
        &self,
        run: &AgentRun,
        provider: &dyn ModelProvider,
        request: ModelRequest,
        cancel: CancellationToken,
    ) -> Result<ModelResponse> {
        let (sender, mut receiver) = mpsc::channel(32);
        let future = provider.respond(request, sender, cancel.clone());
        tokio::pin!(future);
        let mut pending = String::new();
        let mut total = 0usize;
        let mut closed = false;
        let mut ticker = tokio::time::interval(Duration::from_millis(250));
        let response = loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(AppError::new("RUN_CANCELLED", "Đã dừng lượt chạy; agent không gửi thao tác ghi CAD.")),
                result = &mut future => break result.map_err(|e| AppError::new(e.code, e.message))?,
                delta = receiver.recv(), if !closed => match delta {
                    Some(ModelDelta::Text(text)) => { total += text.len(); if total > MAX_TEXT { return Err(AppError::new("PROVIDER_LIMIT", "Phản hồi vượt giới hạn 64 KiB.")); } pending.push_str(&text); },
                    None => closed = true,
                },
                _ = ticker.tick(), if !pending.is_empty() => {
                    // Hold the final word so a credential split across deltas is never emitted.
                    if let Some(end) = pending.rfind(char::is_whitespace) {
                        let chunk = pending[..end].to_string();
                        pending = pending[end..].to_string();
                        if !chunk.is_empty() { self.event(&run.id, "text.delta", Some(provider.redact(&chunk)), None, None).await?; }
                    }
                }
            }
        };
        while let Ok(ModelDelta::Text(text)) = receiver.try_recv() {
            total += text.len();
            pending.push_str(&text);
        }
        if total > MAX_TEXT || response.text.len() > MAX_TEXT || response.continuation.len() > 128 {
            return Err(AppError::new(
                "PROVIDER_LIMIT",
                "Phản hồi vượt giới hạn cho phép.",
            ));
        }
        if !pending.is_empty() {
            self.event(
                &run.id,
                "text.delta",
                Some(provider.redact(&pending)),
                None,
                None,
            )
            .await?;
        }
        Ok(response)
    }

    async fn run_loop(
        &self,
        run: &AgentRun,
        prompt: String,
        provider: Arc<dyn ModelProvider>,
        cancel: CancellationToken,
    ) -> Result<(String, String)> {
        let context = self.studio.context(&run.project_id).await?;
        let mut context = minimal_context(&context);
        // A bounded recent history helps follow-up questions while avoiding the full project.
        // Opaque provider continuation never becomes public chat or persisted reasoning.
        let history = self
            .studio
            .conversation_snapshot(&run.conversation_id)
            .await?;
        context["recent_messages"] = json!(history.messages.iter().rev().skip(1).take(6).rev().map(|m| {
            json!({"role":m.role,"content":provider.redact(&m.content.chars().take(2000).collect::<String>())})
        }).collect::<Vec<_>>());
        let mut continuation = Vec::new();
        let mut outputs = Vec::new();
        let mut calls = 0usize;
        let mut repairs = 0usize;
        let mut proposed: Option<ChangeSet> = None;
        for response_index in 0..=MAX_TOOLS {
            let finalizing = proposed.is_some();
            let request = ModelRequest {
                model: run.model.clone(),
                prompt: prompt.clone(),
                context: context.clone(),
                tools: if finalizing {
                    Vec::new()
                } else {
                    allowed_tools(&run.mode)
                },
                continuation,
                tool_outputs: outputs,
            };
            let response = self
                .provider_turn(run, provider.as_ref(), request, cancel.clone())
                .await?;
            if let Some(change) = proposed {
                // Tool output was returned under the original call ID. Any further calls are ignored;
                // the run ends at the approval boundary regardless of model wording.
                let status = if run.mode == RunMode::Plan {
                    "planned"
                } else {
                    "awaiting_approval"
                };
                let text = if run.mode == RunMode::Plan {
                    "Đã lập phương án Plan. Chưa ghi CAD; chuyển Execute để tạo phương án có thể duyệt."
                } else {
                    "Đã tạo preview. Chưa ghi CAD. Hãy kiểm tra kích thước và duyệt hoặc từ chối phương án."
                };
                self.event(
                    &run.id,
                    "change.proposed",
                    Some(text.into()),
                    None,
                    Some(change.id),
                )
                .await?;
                return Ok((status.into(), text.into()));
            }
            if response.tool_calls.is_empty() {
                return Ok(("completed".into(), provider.redact(&response.text)));
            }
            if calls + response.tool_calls.len() > MAX_TOOLS {
                return Err(AppError::new(
                    "TOOL_LIMIT",
                    "Đã đạt giới hạn 12 lần gọi công cụ.",
                ));
            }
            let allowed: HashSet<String> = allowed_tools(&run.mode)
                .into_iter()
                .map(|t| t.name)
                .collect();
            let mut seen = HashSet::new();
            outputs = Vec::new();
            continuation = response.continuation;
            for call in response.tool_calls {
                if cancel.is_cancelled() {
                    return Err(AppError::new(
                        "RUN_CANCELLED",
                        "Đã dừng trước khi thực thi công cụ.",
                    ));
                }
                calls += 1;
                if !allowed.contains(&call.name) {
                    return Err(AppError::new(
                        "TOOL_DENIED",
                        "Mô hình yêu cầu công cụ ngoài quyền đọc/đề xuất; yêu cầu đã bị chặn.",
                    ));
                }
                if call.call_id.is_empty()
                    || call.call_id.len() > 256
                    || !seen.insert(call.call_id.clone())
                {
                    return Err(AppError::new(
                        "INVALID_CALL_ID",
                        "Call ID của nhà cung cấp không hợp lệ hoặc bị lặp.",
                    ));
                }
                if call.arguments.len() > MAX_ARGUMENTS {
                    return Err(AppError::new(
                        "ARGUMENT_LIMIT",
                        "Arguments công cụ vượt giới hạn 16 KiB.",
                    ));
                }
                if proposed.is_some() {
                    return Err(AppError::new(
                        "PROPOSAL_LIMIT",
                        "Một lượt chỉ lập một phương án trước khi chờ người dùng.",
                    ));
                }
                let tool_id = self.studio.next_id();
                let input_hash = format!("{:x}", Sha256::digest(call.arguments.as_bytes()));
                let call_hash = format!("{:x}", Sha256::digest(call.call_id.as_bytes()));
                // Never persist raw model arguments. Schema validation happens before dispatch.
                sqlx::query("INSERT INTO tool_calls(id,run_id,response_index,provider_call_id,tool_name,input_hash,safe_input_json,status,started_at) VALUES(?,?,?,?,?,?,'{}','started',?)")
                    .bind(&tool_id).bind(&run.id).bind(response_index as i64).bind(call_hash).bind(&call.name).bind(input_hash).bind(self.studio.clock.now())
                    .execute(&self.studio.pool).await.map_err(internal)?;
                self.event(&run.id, "tool.started", None, Some(call.name.clone()), None)
                    .await?;
                let result = tokio::time::timeout(
                    Duration::from_secs(15),
                    self.dispatch(run, &call.name, &call.arguments),
                )
                .await
                .unwrap_or_else(|_| {
                    Err(AppError::new(
                        "TOOL_TIMEOUT",
                        "Công cụ không phản hồi trong 15 giây.",
                    ))
                });
                let (output, failed) = match result {
                    Ok((value, change)) => {
                        if let Some(change) = change {
                            sqlx::query(
                                "INSERT INTO run_change_sets(run_id,change_set_id) VALUES(?,?)",
                            )
                            .bind(&run.id)
                            .bind(&change.id)
                            .execute(&self.studio.pool)
                            .await
                            .map_err(internal)?;
                            proposed = Some(change);
                        }
                        (value, false)
                    }
                    Err(error)
                        if error.code == "INVALID_ARGUMENTS"
                            || error.code == "INVALID_PARAMETERS" =>
                    {
                        repairs += 1;
                        if repairs > MAX_REPAIRS {
                            return Err(AppError::new(
                                "ARGUMENT_REPAIR_LIMIT",
                                "Không sửa được arguments sau hai lần; chưa ghi CAD.",
                            ));
                        }
                        (json!({"error": error.code, "message": error.message}), true)
                    }
                    Err(error) => return Err(error),
                };
                let output = encode(&output)?;
                if output.len() > MAX_TEXT {
                    return Err(AppError::new(
                        "TOOL_OUTPUT_LIMIT",
                        "Kết quả công cụ vượt giới hạn.",
                    ));
                }
                let safe_input = if failed {
                    "{}".into()
                } else {
                    // Successful dispatch accepted a strict numeric/empty schema, so canonicalize it.
                    encode(&serde_json::from_str::<Value>(&call.arguments).map_err(internal)?)?
                };
                sqlx::query(
                    "UPDATE tool_calls SET safe_output_json=?,status=?,finished_at=?,safe_input_json=? WHERE id=?",
                )
                .bind(&output)
                .bind(if failed { "failed" } else { "completed" })
                .bind(self.studio.clock.now())
                .bind(safe_input)
                .bind(tool_id)
                .execute(&self.studio.pool)
                .await
                .map_err(internal)?;
                self.event(
                    &run.id,
                    if failed {
                        "tool.failed"
                    } else {
                        "tool.completed"
                    },
                    None,
                    Some(call.name),
                    proposed.as_ref().map(|c| c.id.clone()),
                )
                .await?;
                outputs.push(ToolOutput {
                    call_id: call.call_id,
                    output,
                });
            }
        }
        Err(AppError::new(
            "TOOL_LIMIT",
            "Đã đạt giới hạn công cụ của lượt chạy.",
        ))
    }

    async fn dispatch(
        &self,
        run: &AgentRun,
        name: &str,
        arguments: &str,
    ) -> Result<(Value, Option<ChangeSet>)> {
        fn arguments_error() -> AppError {
            AppError::new(
                "INVALID_ARGUMENTS",
                "Arguments chưa đầy đủ hoặc không đúng schema; hãy gửi lại đúng schema công cụ.",
            )
        }
        match name {
            "design.propose_plate" => {
                if run.mode == RunMode::Ask {
                    return Err(AppError::new("TOOL_DENIED", "Ask chỉ được đọc CAD."));
                }
                let parameters: PlateParameters =
                    serde_json::from_str(arguments).map_err(|_| arguments_error())?;
                let change = self
                    .studio
                    .propose(&run.project_id, parameters, run.mode.clone())
                    .await?;
                Ok((
                    json!({"change_set_id":change.id,"status":change.status,"preview":change.payload.after}),
                    Some(change),
                ))
            }
            "design.propose_plate_resize" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Resize {
                    width: f64,
                }
                if run.mode == RunMode::Ask {
                    return Err(AppError::new("TOOL_DENIED", "Ask chỉ được đọc CAD."));
                }
                let input: Resize =
                    serde_json::from_str(arguments).map_err(|_| arguments_error())?;
                let context = self.studio.context(&run.project_id).await?;
                let mut parameters = context
                    .geometry
                    .ok_or_else(|| {
                        AppError::new(
                            "NO_PLATE",
                            "Chưa có tấm đế để resize. Hãy tạo và duyệt tấm đế trước.",
                        )
                    })?
                    .parameters;
                parameters.width = input.width;
                let change = self
                    .studio
                    .propose(&run.project_id, parameters, run.mode.clone())
                    .await?;
                Ok((
                    json!({"change_set_id":change.id,"status":change.status,"preview":change.payload.after}),
                    Some(change),
                ))
            }
            "cad.get_status"
            | "cad.get_document_context"
            | "cad.get_selection"
            | "cad.query_entities"
            | "cad.validate_entities" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Empty {}
                let _: Empty = serde_json::from_str(arguments).map_err(|_| arguments_error())?;
                let output = self.studio.cad_read(name, &run.project_id).await?;
                Ok((output, None))
            }
            _ => Err(AppError::new("TOOL_DENIED", "Công cụ không được phép.")),
        }
    }
}

fn minimal_context(context: &DocumentContext) -> Value {
    json!({"mode":context.mode,"units":"mm","revision":context.revision,"parameters":context.geometry.as_ref().map(|g| &g.parameters),"entity_count":context.geometry.as_ref().map_or(0,|g|g.entities.len())})
}

fn allowed_tools(mode: &RunMode) -> Vec<ModelTool> {
    let mut tools = [
        ("cad.get_status", "Đọc trạng thái kết nối CAD mô phỏng."),
        ("cad.get_document_context", "Đọc đơn vị, revision và kích thước chi tiết hiện tại."),
        ("cad.get_selection", "Đọc lựa chọn hiện tại trong CAD mô phỏng."),
        ("cad.query_entities", "Đọc entities của chi tiết mô phỏng hiện tại."),
        ("cad.validate_entities", "Kiểm tra hình học hiện tại, không ghi CAD."),
    ].into_iter().map(|(name,description)| ModelTool { name: name.into(), description: description.into(), input_schema: json!({"type":"object","properties":{},"required":[],"additionalProperties":false}) }).collect::<Vec<_>>();
    if *mode != RunMode::Ask {
        tools.push(ModelTool {
            name: "design.propose_plate".into(),
            description:
                "Tạo preview tấm đế chữ nhật bốn lỗ, đơn vị mm. Chưa ghi CAD và chưa được duyệt."
                    .into(),
            input_schema: serde_json::from_str(include_str!(
                "../../../contracts/schemas/plate-parameters.schema.json"
            ))
            .expect("checked in schema"),
        });
        tools.push(ModelTool { name: "design.propose_plate_resize".into(), description: "Tạo preview đổi chiều dài tấm đế hiện tại, giữ chiều cao, đường kính lỗ, khoảng cách mép. Chưa ghi CAD.".into(), input_schema: json!({"type":"object","properties":{"width":{"type":"number"}},"required":["width"],"additionalProperties":false}) });
    }
    tools
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
