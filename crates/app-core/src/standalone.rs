//! App services: accounts and chat never construct a CadGateway or spawn a process.
use crate::{decode, encode, internal, Result};
use cad_contracts::AppError;
use cad_tool_host::live_client::LiveMcpGateway;
use cad_model_provider::{
    oauth, ModelDelta, ModelEntry, ModelProvider, ModelRequest, ModelTool, OpenAiProvider,
    SecretStore, SecretValue, ToolOutput, WindowsCredentialStore,
};
use cad_native_vault::TokenVault;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use std::{collections::HashSet, path::Path, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn failure(code: &str, message: &str) -> AppError {
    AppError::new(code, message)
}
fn provider_error(e: cad_model_provider::ProviderError) -> AppError {
    failure(e.code, e.message)
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account {
    pub id: String,
    pub auth_mode: String,
    pub label: String,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub client_id: Option<String>,
    pub scope: String,
    pub expires_at: Option<i64>,
    pub selected_model: Option<String>,
    pub status: String,
    pub models: Vec<ModelEntry>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatEvent {
    pub run_id: String,
    pub sequence: u64,
    pub kind: String,
    pub text: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatRun {
    pub id: String,
    pub conversation_id: String,
    pub account_id: String,
    pub model: String,
    pub status: String,
    pub text: String,
    pub error_code: Option<String>,
    pub trace: Vec<Value>,
    pub events: Vec<ChatEvent>,
}
struct Active {
    run: ChatRun,
    cancel: CancellationToken,
}
pub struct AppRuntime {
    pool: SqlitePool,
    vault: TokenVault,
    keys: Arc<dyn SecretStore>,
    gate: Mutex<()>,
    active: Mutex<Option<Active>>,
    login: Mutex<Option<CancellationToken>>,
    cad: Mutex<Option<Arc<LiveMcpGateway>>>,
    _lock: std::fs::File,
}
impl AppRuntime {
    pub async fn open(database: &Path, vault_dir: &Path) -> Result<Arc<Self>> {
        Self::open_with_keys(database, vault_dir, Arc::new(WindowsCredentialStore::new())).await
    }
    async fn open_with_keys(
        database: &Path,
        vault_dir: &Path,
        keys: Arc<dyn SecretStore>,
    ) -> Result<Arc<Self>> {
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(database.with_extension("accounts.lock"))
            .map_err(internal)?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            failure(
                "ACCOUNT_RUNTIME_IN_USE",
                "Một bản ứng dụng khác đang dùng tài khoản/chat. Đóng bản đó trước khi mở bản mới.",
            )
        })?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(database)
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .journal_mode(SqliteJournalMode::Delete)
                    .synchronous(SqliteSynchronous::Full)
                    .busy_timeout(Duration::from_secs(5)),
            )
            .await
            .map_err(internal)?;
        sqlx::migrate!("../../database/migrations")
            .run(&pool)
            .await
            .map_err(internal)?;
        sqlx::query(
            "UPDATE standalone_runs SET status='interrupted',finished_at=? WHERE status='running'",
        )
        .bind(oauth::now())
        .execute(&pool)
        .await
        .map_err(internal)?;
        // Restore a verified inference label for profiles tested by an older build.
        // The completed run must belong to the currently selected model.
        sqlx::query("UPDATE llm_accounts SET status='inference_verified' WHERE status IN ('verified','catalog_verified') AND EXISTS (SELECT 1 FROM standalone_runs r WHERE r.account_id=llm_accounts.id AND r.model=llm_accounts.selected_model AND r.status='completed')")
            .execute(&pool).await.map_err(internal)?;
        sqlx::query("INSERT OR IGNORE INTO app_settings VALUES('oauth_host_id',?)")
            .bind(format!("urn:uuid:{}", id()))
            .execute(&pool)
            .await
            .map_err(internal)?;
        Ok(Arc::new(Self {
            pool,
            vault: TokenVault::new(vault_dir.into()),
            keys,
            gate: Mutex::new(()),
            active: Mutex::new(None),
            login: Mutex::new(None),
            cad: Mutex::new(None),
            _lock: lock,
        }))
    }
    pub async fn accounts(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query("SELECT * FROM llm_accounts ORDER BY created_at,id")
            .fetch_all(&self.pool)
            .await
            .map_err(internal)?;
        rows.into_iter()
            .map(|r| {
                Ok(Account {
                    id: r.get("id"),
                    auth_mode: r.get("auth_mode"),
                    label: r.get("label"),
                    issuer: r.get("issuer"),
                    subject: r.get("subject"),
                    client_id: r.get("client_id"),
                    scope: r.get("scope"),
                    expires_at: r.get("expires_at"),
                    selected_model: r.get("selected_model"),
                    status: r.get("status"),
                    models: decode(r.get("catalog_json"))?,
                })
            })
            .collect()
    }
    async fn account(&self, account: &str) -> Result<Account> {
        self.accounts()
            .await?
            .into_iter()
            .find(|a| a.id == account)
            .ok_or_else(|| failure("ACCOUNT_MISSING", "Không tìm thấy profile."))
    }
    async fn secret_ref(&self, account: &str) -> Result<String> {
        sqlx::query_scalar("SELECT secret_ref FROM llm_accounts WHERE id=?")
            .bind(account)
            .fetch_one(&self.pool)
            .await
            .map_err(internal)
    }
    async fn idle(&self) -> Result<()> {
        if self.active.lock().await.is_some() {
            return Err(failure(
                "RUN_ACTIVE",
                "Hãy dừng hoặc chờ lượt chat hiện tại kết thúc.",
            ));
        }
        Ok(())
    }
    pub async fn add_api_key(&self, label: &str, secret: String, consent: bool) -> Result<Account> {
        let secret = SecretValue::new(secret)
            .map_err(|_| failure("INVALID_SECRET", "API key không hợp lệ."))?;
        if !consent {
            return Err(failure(
                "CONSENT_REQUIRED",
                "Cần đồng ý gửi chat/tool output tới OpenAI; API có billing riêng.",
            ));
        }
        let _gate = self.gate.lock().await;
        self.idle().await?;
        if label.trim().is_empty() || label.len() > 120 || label.contains("sk-") {
            return Err(failure(
                "INVALID_LABEL",
                "Nhãn profile phải có 1–120 ký tự, không chứa API key.",
            ));
        }
        let account = id();
        let reference = format!("live-{}", id());
        self.keys.set(&reference, secret).map_err(|_| {
            failure(
                "VAULT_ERROR",
                "Không lưu được API key trong Windows Credential Manager.",
            )
        })?;
        let result=sqlx::query("INSERT INTO llm_accounts(id,auth_mode,label,secret_ref,status,created_at) VALUES(?,'api_key',?,?,'stored_unverified',?)").bind(&account).bind(label.trim()).bind(&reference).bind(oauth::now()).execute(&self.pool).await;
        if result.is_err() {
            let _ = self.keys.delete(&reference);
            return Err(internal("account insert"));
        }
        self.account(&account).await
    }
    pub async fn begin_login(
        &self,
        account: Option<String>,
        consent: bool,
    ) -> Result<(oauth::Login, String, CancellationToken)> {
        if !consent {
            return Err(failure(
                "CONSENT_REQUIRED",
                "Cần đồng ý dùng gói ChatGPT và gửi chat/tool output tới OpenAI.",
            ));
        }
        let _gate = self.gate.lock().await;
        self.idle().await?;
        let mut login = self.login.lock().await;
        if login.is_some() {
            return Err(failure("LOGIN_ACTIVE", "Đang có một lần đăng nhập."));
        }
        let returning = match account {
            Some(ref value) => {
                let a = self.account(value).await?;
                if a.auth_mode != "chatgpt" {
                    return Err(failure("AUTH_ROUTE", "Profile không thuộc ChatGPT."));
                }
                Some((
                    a.client_id.ok_or_else(|| internal("client"))?,
                    a.subject.ok_or_else(|| internal("subject"))?,
                ))
            }
            None => None,
        };
        let host: String =
            sqlx::query_scalar("SELECT value FROM app_settings WHERE key='oauth_host_id'")
                .fetch_one(&self.pool)
                .await
                .map_err(internal)?;
        let (pending, url) = oauth::Login::begin(&host, returning)
            .await
            .map_err(provider_error)?;
        let cancel = CancellationToken::new();
        *login = Some(cancel.clone());
        Ok((pending, url, cancel))
    }
    pub async fn cancel_login(&self) {
        if let Some(cancel) = self.login.lock().await.as_ref() {
            cancel.cancel();
        }
    }
    pub async fn abandon_login(&self) {
        self.cancel_login().await;
        *self.login.lock().await = None;
    }
    pub async fn finish_login(
        &self,
        pending: oauth::Login,
        cancel: CancellationToken,
    ) -> Result<Account> {
        let result = pending.finish(cancel).await;
        *self.login.lock().await = None;
        let identity = result.map_err(provider_error)?;
        let _gate = self.gate.lock().await;
        self.idle().await?;
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM llm_accounts WHERE issuer=? AND subject=? AND client_id=?",
        )
        .bind(oauth::ISSUER)
        .bind(&identity.subject)
        .bind(&identity.client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;
        let account = existing.clone().unwrap_or_else(id);
        let reference = id();
        let bytes =
            zeroize::Zeroizing::new(serde_json::to_vec(&identity.bundle).map_err(internal)?);
        self.vault
            .put(&reference, &bytes)
            .map_err(|_| failure("VAULT_ERROR", "Không lưu được token bundle DPAPI."))?;
        let old_ref = if existing.is_some() {
            Some(self.secret_ref(&account).await?)
        } else {
            None
        };
        let status = if identity.bundle.permitted() {
            "verified"
        } else {
            "missing_permission"
        };
        let result=sqlx::query("INSERT INTO llm_accounts(id,auth_mode,issuer,subject,client_id,label,secret_ref,scope,expires_at,status,created_at) VALUES(?,'chatgpt',?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET secret_ref=excluded.secret_ref,scope=excluded.scope,expires_at=excluded.expires_at,status=excluded.status,selected_model=NULL,catalog_json='[]',catalog_at=NULL")
   .bind(&account).bind(oauth::ISSUER).bind(&identity.subject).bind(&identity.client_id).bind(format!("ChatGPT {}",&account[..8])).bind(&reference).bind(&identity.bundle.scope).bind(identity.bundle.expires_at).bind(status).bind(oauth::now()).execute(&self.pool).await;
        if result.is_err() {
            let _ = self.vault.delete(&reference);
            return Err(internal("account metadata"));
        }
        if let Some(old) = old_ref {
            let _ = self.vault.delete(&old);
        }
        self.account(&account).await
    }
    async fn provider(&self, account: &Account) -> Result<OpenAiProvider> {
        if account.status == "signed_out" {
            return Err(failure("SIGNED_OUT", "Profile đã đăng xuất."));
        }
        let reference = self.secret_ref(&account.id).await?;
        if account.auth_mode == "api_key" {
            let key = self
                .keys
                .get(&reference)
                .map_err(|_| failure("VAULT_ERROR", "Không đọc được khóa API."))?
                .ok_or_else(|| failure("CREDENTIAL_MISSING", "Chưa có khóa API."))?;
            return OpenAiProvider::new(key).map_err(provider_error);
        }
        let raw = self
            .vault
            .get(&reference)
            .map_err(|_| failure("VAULT_ERROR", "Không đọc được token DPAPI."))?;
        let mut bundle: oauth::TokenBundle = serde_json::from_slice(&raw)
            .map_err(|_| failure("VAULT_CORRUPT", "Token bundle không hợp lệ."))?;
        if bundle.expires_at <= oauth::now() + 60 {
            match oauth::refresh(
                account
                    .client_id
                    .as_deref()
                    .ok_or_else(|| internal("client"))?,
                &bundle,
            )
            .await
            {
                Ok(next) => {
                    let bytes =
                        zeroize::Zeroizing::new(serde_json::to_vec(&next).map_err(internal)?);
                    self.vault
                        .put(&reference, &bytes)
                        .map_err(|_| failure("VAULT_ERROR", "Không lưu được token đã refresh."))?;
                    bundle = next;
                    sqlx::query("UPDATE llm_accounts SET expires_at=?,scope=?,status=? WHERE id=?")
                        .bind(bundle.expires_at)
                        .bind(&bundle.scope)
                        .bind(if bundle.permitted() {
                            "verified"
                        } else {
                            "missing_permission"
                        })
                        .bind(&account.id)
                        .execute(&self.pool)
                        .await
                        .map_err(internal)?;
                }
                Err(e) => {
                    sqlx::query("UPDATE llm_accounts SET status='expired' WHERE id=?")
                        .bind(&account.id)
                        .execute(&self.pool)
                        .await
                        .map_err(internal)?;
                    return Err(provider_error(e));
                }
            }
        }
        Ok(
            OpenAiProvider::new(bundle.bearer().map_err(provider_error)?)
                .map_err(provider_error)?
                .with_chatgpt_plan(),
        )
    }
    pub async fn catalog(&self, account: &str) -> Result<Account> {
        let _gate = self.gate.lock().await;
        self.idle().await?;
        let a = self.account(account).await?;
        let result = self.provider(&a).await?.catalog().await;
        let models = match result {
            Ok(m) => m,
            Err(e) => {
                sqlx::query("UPDATE llm_accounts SET catalog_json='[]',selected_model=NULL,status='catalog_error' WHERE id=?").bind(account).execute(&self.pool).await.map_err(internal)?;
                return Err(provider_error(e));
            }
        };
        let selected = a
            .selected_model
            .filter(|id| models.iter().any(|m| &m.id == id));
        sqlx::query("UPDATE llm_accounts SET catalog_json=?,catalog_at=?,selected_model=?,status='catalog_verified' WHERE id=?").bind(encode(&models)?).bind(oauth::now()).bind(selected).bind(account).execute(&self.pool).await.map_err(internal)?;
        self.account(account).await
    }
    pub async fn select_model(&self, account: &str, model: &str) -> Result<Account> {
        let _gate = self.gate.lock().await;
        self.idle().await?;
        let a = self.account(account).await?;
        if !a.models.iter().any(|m| m.id == model) {
            return Err(failure(
                "MODEL_UNAVAILABLE",
                "Model không có trong catalog của profile này.",
            ));
        }
        sqlx::query("UPDATE llm_accounts SET selected_model=?,status=CASE WHEN selected_model=? THEN status ELSE 'catalog_verified' END WHERE id=?")
            .bind(model)
            .bind(model)
            .bind(account)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        self.account(account).await
    }
    pub async fn sign_out(&self, account: &str) -> Result<String> {
        let _gate = self.gate.lock().await;
        self.idle().await?;
        let a = self.account(account).await?;
        let reference = self.secret_ref(account).await?;
        let mut remote = false;
        if a.auth_mode == "chatgpt" {
            if let Ok(raw) = self.vault.get(&reference) {
                if let Ok(bundle) = serde_json::from_slice::<oauth::TokenBundle>(&raw) {
                    remote = oauth::revoke(a.client_id.as_deref().unwrap_or(""), &bundle)
                        .await
                        .is_ok();
                }
            }
        }
        sqlx::query("UPDATE llm_accounts SET status='signed_out',selected_model=NULL,catalog_json='[]' WHERE id=?").bind(account).execute(&self.pool).await.map_err(internal)?;
        if a.auth_mode == "api_key" {
            self.keys.delete(&reference).map_err(|_| {
                failure(
                    "VAULT_DELETE",
                    "Đã khóa profile nhưng chưa xóa được credential.",
                )
            })?;
        } else {
            self.vault.delete(&reference).map_err(|_| {
                failure("VAULT_DELETE", "Đã khóa profile nhưng chưa xóa được token.")
            })?;
        }
        Ok(if a.auth_mode == "chatgpt" && !remote {
            "Đã đăng xuất local; chưa xác nhận thu hồi phiên từ OpenAI."
        } else {
            "Đã đăng xuất và xóa credential local."
        }
        .into())
    }
    pub async fn conversations(&self, account: &str) -> Result<Vec<Value>> {
        let rows=sqlx::query("SELECT id,title FROM standalone_conversations WHERE account_id=? ORDER BY created_at DESC,id").bind(account).fetch_all(&self.pool).await.map_err(internal)?;
        Ok(rows
            .iter()
            .map(|r| json!({"id":r.get::<String,_>("id"),"title":r.get::<String,_>("title")}))
            .collect())
    }
    pub async fn create_conversation(&self, account: &str) -> Result<String> {
        let _gate = self.gate.lock().await;
        self.idle().await?;
        self.account(account).await?;
        let id = id();
        sqlx::query("INSERT INTO standalone_conversations VALUES(?,?,?,?)")
            .bind(&id)
            .bind(account)
            .bind("Hội thoại mới")
            .bind(oauth::now())
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        Ok(id)
    }
    pub async fn messages(&self, conversation: &str) -> Result<Vec<Value>> {
        let rows=sqlx::query("SELECT role,content,run_id FROM standalone_messages WHERE conversation_id=? ORDER BY created_at,rowid").bind(conversation).fetch_all(&self.pool).await.map_err(internal)?;
        Ok(rows.iter().map(|r|json!({"role":r.get::<String,_>("role"),"content":r.get::<String,_>("content"),"run_id":r.get::<String,_>("run_id")})).collect())
    }
    pub async fn start(
        self: &Arc<Self>,
        conversation: &str,
        content: String,
        probe: bool,
    ) -> Result<ChatRun> {
        let _gate = self.gate.lock().await;
        self.idle().await?;
        if self.login.lock().await.is_some() {
            return Err(failure(
                "LOGIN_ACTIVE",
                "Hãy hoàn tất đăng nhập trước khi gửi chat.",
            ));
        }
        let account_id: String =
            sqlx::query_scalar("SELECT account_id FROM standalone_conversations WHERE id=?")
                .bind(conversation)
                .fetch_one(&self.pool)
                .await
                .map_err(internal)?;
        let account = self.account(&account_id).await?;
        let model = account
            .selected_model
            .clone()
            .ok_or_else(|| failure("MODEL_REQUIRED", "Lấy catalog và chọn model trước."))?;
        if !account.models.iter().any(|m| m.id == model) {
            return Err(failure("MODEL_UNAVAILABLE", "Cần làm mới model catalog."));
        }
        let provider: Arc<dyn ModelProvider> = Arc::new(self.provider(&account).await?);
        let content = if probe {
            "Hãy dùng công cụ convert_length để đổi 1.25 m sang mm, rồi trả lời dựa trên kết quả công cụ.".into()
        } else {
            content
        };
        if content.trim().is_empty() || content.len() > 16000 {
            return Err(failure("MESSAGE_SIZE", "Yêu cầu phải có 1–16000 byte."));
        }
        let content = provider.redact(&content);
        let mut history = self.messages(conversation).await?;
        if history.len() > 40 {
            history.drain(..history.len() - 40);
        }
        let mut continuation: Vec<Value> = history
            .iter()
            .map(|m| json!({"role":m["role"],"content":m["content"]}))
            .collect();
        continuation.push(json!({"role":"user","content":content}));
        let run = ChatRun {
            id: id(),
            conversation_id: conversation.into(),
            account_id,
            model,
            status: "running".into(),
            text: String::new(),
            error_code: None,
            trace: vec![],
            events: vec![],
        };
        let mut tx = self.pool.begin().await.map_err(internal)?;
        sqlx::query("INSERT INTO standalone_runs(id,conversation_id,account_id,model,status,started_at) VALUES(?,?,?,?,'running',?)").bind(&run.id).bind(conversation).bind(&run.account_id).bind(&run.model).bind(oauth::now()).execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO standalone_messages VALUES(?,?,?,'user',?,?)")
            .bind(id())
            .bind(conversation)
            .bind(&run.id)
            .bind(&content)
            .bind(oauth::now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        let cancel = CancellationToken::new();
        *self.active.lock().await = Some(Active {
            run: run.clone(),
            cancel: cancel.clone(),
        });
        let this = self.clone();
        let task_run = run.clone();
        tokio::spawn(async move {
            let outcome = tokio::time::timeout(
                Duration::from_secs(180),
                this.execute(&task_run, provider, continuation, cancel.clone(), probe),
            )
            .await;
            let result =
                outcome.unwrap_or_else(|_| Err(failure("RUN_TIMEOUT", "Lượt chat vượt 180 giây.")));
            this.finish(&task_run, result, cancel.is_cancelled()).await;
        });
        Ok(run)
    }
    async fn event(&self, run: &str, kind: &str, text: String) {
        let mut active = self.active.lock().await;
        if let Some(a) = active.as_mut().filter(|a| a.run.id == run) {
            let sequence = a.run.events.last().map_or(1, |e| e.sequence + 1);
            if a.run.events.len() < 2048 {
                a.run.events.push(ChatEvent {
                    run_id: run.into(),
                    sequence,
                    kind: kind.into(),
                    text,
                });
            }
        }
    }
    async fn execute(
        &self,
        run: &ChatRun,
        provider: Arc<dyn ModelProvider>,
        mut history: Vec<Value>,
        cancel: CancellationToken,
        probe: bool,
    ) -> Result<(String, Vec<Value>)> {
        let mut outputs = Vec::new();
        let mut ids = HashSet::new();
        let mut trace = Vec::new();
        let mut conversion = false;
        let mut needs_cad = false;
        for _ in 0..6 {
            if cancel.is_cancelled() {
                return Err(failure("CANCELLED", "Đã dừng."));
            }
            let cad = self.cad.lock().await.clone();
            let request = ModelRequest {
                model: run.model.clone(),
                prompt: String::new(),
                context: json!({"cad":if cad.is_some(){"connected_read_only"}else{"disconnected"},"project":null}),
                tools: tools(cad.is_some()),
                continuation: history,
                tool_outputs: outputs,
            };
            let (sender, mut receiver) = mpsc::channel(64);
            let response = provider.respond(request, sender, cancel.clone());
            tokio::pin!(response);
            let result=loop {tokio::select!{r=&mut response=>break r,d=receiver.recv()=>{if let Some(ModelDelta::Text(text))=d{self.event(&run.id,"text.delta",text).await;}else{break response.await;}}}}.map_err(provider_error)?;
            while let Ok(ModelDelta::Text(text)) = receiver.try_recv() {
                self.event(&run.id, "text.delta", text).await;
            }
            history = result.continuation;
            outputs = vec![];
            if result.tool_calls.is_empty() {
                if probe && !conversion {
                    return Err(failure("PROBE_NO_TOOL","Text inference hoàn tất nhưng chưa có function call convert_length đúng; probe chưa đạt."));
                }
                if needs_cad {
                    self.event(
                        &run.id,
                        "cad.required",
                        "Cần kết nối và chọn bản vẽ AutoCAD trước khi vẽ.".into(),
                    )
                    .await;
                }
                return Ok((provider.redact(&result.text), trace));
            }
            for call in result.tool_calls {
                if cancel.is_cancelled() {
                    return Err(failure("CANCELLED", "Đã dừng trước khi chạy tool."));
                }
                if !ids.insert(call.call_id.clone()) || ids.len() > 12 {
                    return Err(failure(
                        "TOOL_LIMIT",
                        "Call ID trùng hoặc vượt ngân sách tool.",
                    ));
                }
                let output = if call.name.starts_with("cad.") {
                    let gateway = cad.as_ref().ok_or_else(|| failure("CAD_DISCONNECTED", "AutoCAD chưa kết nối."))?;
                    let arguments: Value = serde_json::from_str(&call.arguments).map_err(internal)?;
                    gateway.call(&call.name, arguments).await?
                } else { execute_tool(&call.name, &call.arguments)? };
                let args: Value = serde_json::from_str(&call.arguments).map_err(internal)?;
                if call.name == "convert_length"
                    && args == json!({"value":1.25,"from":"m","to":"mm"})
                    && output["value"] == 1250.0
                    && output["unit"] == "mm"
                {
                    conversion = true;
                }
                if call.name == "cad_connection_required" {
                    needs_cad = true;
                }
                let entry = json!({"call_id":call.call_id,"tool":call.name,"arguments":args,"output":output});
                self.event(&run.id, "tool.completed", entry.to_string())
                    .await;
                trace.push(entry);
                outputs.push(ToolOutput {
                    call_id: call.call_id,
                    output: output.to_string(),
                });
            }
        }
        Err(failure("TOOL_LOOP", "Vượt giới hạn 6 vòng tool."))
    }
    async fn finish(&self, run: &ChatRun, result: Result<(String, Vec<Value>)>, cancelled: bool) {
        let (status, text, trace, error) = match result {
            Ok((text, trace)) if !cancelled => ("completed", text, trace, None),
            Ok(_) => ("cancelled", String::new(), vec![], None),
            Err(e) => (
                if cancelled { "cancelled" } else { "failed" },
                e.message,
                vec![],
                Some(e.code),
            ),
        };
        let saved:Result<()>=async{let mut tx=self.pool.begin().await.map_err(internal)?;sqlx::query("UPDATE standalone_runs SET status=?,text=?,tool_trace_json=?,error_code=?,finished_at=? WHERE id=?").bind(status).bind(&text).bind(encode(&trace)?).bind(&error).bind(oauth::now()).bind(&run.id).execute(&mut *tx).await.map_err(internal)?;
   if status=="completed"{
    sqlx::query("INSERT INTO standalone_messages VALUES(?,?,?,'assistant',?,?)").bind(id()).bind(&run.conversation_id).bind(&run.id).bind(&text).bind(oauth::now()).execute(&mut *tx).await.map_err(internal)?;
    sqlx::query("UPDATE llm_accounts SET status='inference_verified' WHERE id=? AND selected_model=? AND status NOT IN ('signed_out','missing_permission','expired')").bind(&run.account_id).bind(&run.model).execute(&mut *tx).await.map_err(internal)?;
   }
   tx.commit().await.map_err(internal)?;Ok(())}.await;
        let mut active = self.active.lock().await;
        if saved.is_ok() {
            *active = None;
        } else if let Some(a) = active.as_mut() {
            a.run.status = "failed".into();
            a.run.error_code = Some("PERSIST_FAILED".into());
            a.run.text = "Không lưu được kết quả; không tự gửi lại.".into();
        }
    }
    pub async fn snapshot(&self, run: &str) -> Result<ChatRun> {
        if let Some(a) = self
            .active
            .lock()
            .await
            .as_ref()
            .filter(|a| a.run.id == run)
        {
            return Ok(a.run.clone());
        }
        let r = sqlx::query("SELECT * FROM standalone_runs WHERE id=?")
            .bind(run)
            .fetch_one(&self.pool)
            .await
            .map_err(internal)?;
        Ok(ChatRun {
            id: r.get("id"),
            conversation_id: r.get("conversation_id"),
            account_id: r.get("account_id"),
            model: r.get("model"),
            status: r.get("status"),
            text: r.get("text"),
            error_code: r.get("error_code"),
            trace: decode(r.get("tool_trace_json"))?,
            events: vec![],
        })
    }
    pub async fn cancel(&self, run: &str) -> Result<()> {
        if let Some(a) = self
            .active
            .lock()
            .await
            .as_ref()
            .filter(|a| a.run.id == run)
        {
            a.cancel.cancel();
        }
        Ok(())
    }
    pub async fn cad_pair(&self, binary: &Path, pid: u32, code: &str) -> Result<Value> {
        self.idle().await?;
        let (gateway, verified) = LiveMcpGateway::pair(binary, pid, code).await?;
        let mut current = self.cad.lock().await;
        if let Some(old) = current.take() { old.shutdown().await; }
        *current = Some(Arc::new(gateway));
        Ok(verified)
    }
    pub async fn cad_status(&self) -> Result<Value> {
        let current = self.cad.lock().await.clone();
        match current { Some(gateway) => gateway.call("cad.get_status", json!({})).await,
            None => Ok(json!({"connected":false,"mode":"unconnected","write":false})) }
    }
    pub async fn cad_disconnect(&self) {
        if let Some(gateway) = self.cad.lock().await.take() { gateway.shutdown().await; }
    }
    pub async fn shutdown(&self) {
        self.cancel_login().await;
        if let Some(a) = self.active.lock().await.as_ref() {
            a.cancel.cancel();
        }
        self.cad_disconnect().await;
    }
}
fn tools(connected: bool) -> Vec<ModelTool> {
    let mut result = vec![
 ModelTool{name:"convert_length".into(),description:"Convert a finite length between mm/cm/m locally; no CAD access.".into(),input_schema:json!({"type":"object","properties":{"value":{"type":"number"},"from":{"type":"string","enum":["mm","cm","m"]},"to":{"type":"string","enum":["mm","cm","m"]}},"required":["value","from","to"],"additionalProperties":false})},
 ];
    if !connected {
        result.push(ModelTool{name:"cad_connection_required".into(),description:"Call when user asks to draw, modify or inspect an AutoCAD document. Reports disconnected; never draws.".into(),input_schema:json!({"type":"object","properties":{},"additionalProperties":false})});
    } else {
        let empty = json!({"type":"object","properties":{},"additionalProperties":false});
        for (name, description) in [
            ("cad.get_status", "Read current AutoCAD connection and active document identity."),
            ("cad.list_documents", "List open AutoCAD documents and identify the active drawing."),
            ("cad.get_document_context", "Read current drawing units, space and revision."),
            ("cad.list_layers", "Read layer names and total layer count from current drawing."),
            ("cad.get_selection", "Read count and summaries of currently selected entities."),
        ] { result.push(ModelTool{name:name.into(),description:description.into(),input_schema:empty.clone()}); }
        result.push(ModelTool{name:"cad.query_entities".into(),description:"Read up to 100 entities from active space; optional layer and entity_type filters.".into(),input_schema:json!({"type":"object","properties":{"max_results":{"type":"integer","minimum":1,"maximum":100},"layer":{"type":"string"},"entity_type":{"type":"string"}},"additionalProperties":false})});
        result.push(ModelTool{name:"cad.validate_entities".into(),description:"Verify up to 100 entity handles in current drawing without editing.".into(),input_schema:json!({"type":"object","properties":{"handles":{"type":"array","items":{"type":"string"},"maxItems":100}},"required":["handles"],"additionalProperties":false})});
    }
    result
}
fn execute_tool(name: &str, args: &str) -> Result<Value> {
    if args.len() > 2048 {
        return Err(failure("TOOL_ARGUMENTS", "Tool input quá lớn."));
    }
    if name == "cad_connection_required" {
        let value: Value = serde_json::from_str(args).map_err(internal)?;
        if value != json!({}) {
            return Err(failure("TOOL_ARGUMENTS", "Tool không nhận arguments."));
        }
        return Ok(
            json!({"status":"cad_connection_required","cad":"disconnected","applied":false}),
        );
    }
    if name != "convert_length" {
        return Err(failure("TOOL_DENIED", "Tool không được cấp quyền."));
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Input {
        value: f64,
        from: String,
        to: String,
    }
    let input: Input = serde_json::from_str(args)
        .map_err(|_| failure("TOOL_ARGUMENTS", "Arguments không đúng schema."))?;
    let scale = |unit: &str| match unit {
        "mm" => Ok(1.0),
        "cm" => Ok(10.0),
        "m" => Ok(1000.0),
        _ => Err(failure("TOOL_UNITS", "Đơn vị không hợp lệ.")),
    };
    let value = input.value * scale(&input.from)? / scale(&input.to)?;
    if !value.is_finite() || value.abs() > 1e12 {
        return Err(failure("TOOL_VALUE", "Giá trị vượt giới hạn."));
    }
    Ok(json!({"value":value,"unit":input.to}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversion_validation() {
        assert_eq!(
            execute_tool("convert_length", r#"{"value":1.25,"from":"m","to":"mm"}"#).unwrap(),
            json!({"value":1250.0,"unit":"mm"})
        );
        for args in [
            r#"{"value":1,"from":"inch","to":"mm"}"#,
            r#"{"value":1,"from":"m","to":"mm","extra":true}"#,
            "{",
        ] {
            assert!(execute_tool("convert_length", args).is_err());
        }
        assert!(execute_tool("shell", "{}").is_err());
        assert_eq!(
            execute_tool("cad_connection_required", "{}").unwrap()["applied"],
            false
        );
    }
    #[tokio::test]
    async fn standalone_persists_without_project_or_gateway() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("app.sqlite");
        let keys = Arc::new(cad_model_provider::SessionSecretStore::new_explicit_session());
        let app = AppRuntime::open_with_keys(&db, &dir.path().join("vault"), keys.clone())
            .await
            .unwrap();
        let a = app
            .add_api_key("Test API", "fixture-key-no-network".into(), true)
            .await
            .unwrap();
        let b = app
            .add_api_key("Other", "fixture-key-other".into(), true)
            .await
            .unwrap();
        let c = app.create_conversation(&a.id).await.unwrap();
        assert!(app.conversations(&b.id).await.unwrap().is_empty());
        assert!(app.select_model(&a.id, "not-in-catalog").await.is_err());
        let projects: i64 = sqlx::query_scalar("SELECT count(*) FROM projects")
            .fetch_one(&app.pool)
            .await
            .unwrap();
        assert_eq!(projects, 0);
        sqlx::query("UPDATE llm_accounts SET catalog_json=? WHERE id=?")
            .bind(r#"[{"id":"model-a","label":"Model A"}]"#)
            .bind(&a.id)
            .execute(&app.pool)
            .await
            .unwrap();
        app.select_model(&a.id, "model-a").await.unwrap();
        assert!(app.select_model(&b.id, "model-a").await.is_err());
        sqlx::query("INSERT INTO standalone_runs(id,conversation_id,account_id,model,status,started_at) VALUES('persisted-run',?,?,'model-a','completed',1)")
            .bind(&c).bind(&a.id).execute(&app.pool).await.unwrap();
        sqlx::query("INSERT INTO standalone_messages VALUES('persisted-message',?,'persisted-run','assistant','Brief được giữ sau restart',1)")
            .bind(&c).execute(&app.pool).await.unwrap();
        let validation_run = ChatRun {
            id: "validation-run".into(),
            conversation_id: c.clone(),
            account_id: a.id.clone(),
            model: "model-a".into(),
            status: "running".into(),
            text: String::new(),
            error_code: None,
            trace: vec![],
            events: vec![],
        };
        sqlx::query("INSERT INTO standalone_runs(id,conversation_id,account_id,model,status,started_at) VALUES(?,?,?,'model-a','running',2)")
            .bind(&validation_run.id).bind(&c).bind(&a.id).execute(&app.pool).await.unwrap();
        app.finish(
            &validation_run,
            Ok(("Đã xác minh inference".into(), vec![])),
            false,
        )
        .await;
        assert_eq!(
            app.account(&a.id).await.unwrap().status,
            "inference_verified"
        );
        sqlx::query("UPDATE llm_accounts SET status='verified' WHERE id=?")
            .bind(&a.id)
            .execute(&app.pool)
            .await
            .unwrap();
        app.pool.close().await;
        drop(app);
        let reopened = AppRuntime::open_with_keys(&db, &dir.path().join("vault"), keys)
            .await
            .unwrap();
        assert_eq!(reopened.accounts().await.unwrap().len(), 2);
        assert_eq!(
            reopened.account(&a.id).await.unwrap().status,
            "inference_verified"
        );
        assert_eq!(reopened.conversations(&a.id).await.unwrap()[0]["id"], c);
        assert_eq!(
            reopened.messages(&c).await.unwrap()[0]["content"],
            "Brief được giữ sau restart"
        );
        assert_eq!(
            reopened
                .account(&a.id)
                .await
                .unwrap()
                .selected_model
                .as_deref(),
            Some("model-a")
        );
        assert!(reopened.account(&b.id).await.unwrap().models.is_empty());
        assert!(!std::fs::read(&db)
            .unwrap()
            .windows(b"fixture-key-no-network".len())
            .any(|x| x == b"fixture-key-no-network"));
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;
    use cad_model_provider::{ModelResponse, ProviderError, ProviderToolCall};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Fixture {
        kind: &'static str,
        calls: AtomicUsize,
    }
    #[async_trait::async_trait]
    impl ModelProvider for Fixture {
        async fn respond(
            &self,
            request: ModelRequest,
            _events: mpsc::Sender<ModelDelta>,
            cancel: CancellationToken,
        ) -> std::result::Result<ModelResponse, ProviderError> {
            let round = self.calls.fetch_add(1, Ordering::SeqCst);
            if self.kind == "cancel" {
                cancel.cancel();
                return Ok(ModelResponse::default());
            }
            if self.kind == "zero" {
                return Ok(ModelResponse {
                    text: "Hello".into(),
                    ..Default::default()
                });
            }
            if round > 0 && self.kind != "loop" && self.kind != "duplicate" {
                assert_eq!(
                    request.tool_outputs.len(),
                    if self.kind == "multiple" { 2 } else { 1 }
                );
                assert!(request.tool_outputs[0].output.contains("1250"));
                return Ok(ModelResponse {
                    text: "1250 mm".into(),
                    ..Default::default()
                });
            }
            let call = ProviderToolCall {
                call_id: if self.kind == "loop" {
                    format!("call-{round}")
                } else {
                    "call-1".into()
                },
                name: if self.kind == "unknown" {
                    "shell"
                } else {
                    "convert_length"
                }
                .into(),
                arguments: if self.kind == "malformed" {
                    "{"
                } else {
                    r#"{"value":1.25,"from":"m","to":"mm"}"#
                }
                .into(),
            };
            let mut calls = vec![call.clone()];
            if self.kind == "multiple" {
                let mut other = call;
                other.call_id = "call-2".into();
                calls.push(other);
            }
            Ok(ModelResponse {
                tool_calls: calls,
                ..Default::default()
            })
        }
    }
    #[tokio::test]
    async fn bounded_tool_loop_and_real_output_continuation() {
        let dir = tempfile::tempdir().unwrap();
        let app = AppRuntime::open_with_keys(
            &dir.path().join("db"),
            &dir.path().join("vault"),
            Arc::new(cad_model_provider::SessionSecretStore::new_explicit_session()),
        )
        .await
        .unwrap();
        let run = ChatRun {
            id: "run".into(),
            conversation_id: "conversation".into(),
            account_id: "account".into(),
            model: "fixture".into(),
            status: "running".into(),
            text: String::new(),
            error_code: None,
            trace: vec![],
            events: vec![],
        };
        for kind in [
            "one",
            "multiple",
            "zero",
            "malformed",
            "unknown",
            "duplicate",
            "loop",
            "cancel",
        ] {
            let result = app
                .execute(
                    &run,
                    Arc::new(Fixture {
                        kind,
                        calls: AtomicUsize::new(0),
                    }),
                    vec![],
                    CancellationToken::new(),
                    true,
                )
                .await;
            assert_eq!(result.is_ok(), matches!(kind, "one" | "multiple"), "{kind}");
        }
        assert!(app
            .execute(
                &run,
                Arc::new(Fixture {
                    kind: "zero",
                    calls: AtomicUsize::new(0)
                }),
                vec![],
                CancellationToken::new(),
                false
            )
            .await
            .is_ok());
    }
}
