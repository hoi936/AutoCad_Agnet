#![forbid(unsafe_code)]
pub mod agent;
pub mod geometry;
pub mod standalone;
pub use agent::AgentService;
pub mod provider_service;
pub use provider_service::ProviderService;
mod gateway_integration;
use cad_contracts::*;
pub use gateway_integration::ApprovalService;
use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use uuid::Uuid;

type Result<T> = std::result::Result<T, AppError>;
pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}
pub trait IdGenerator: Send + Sync {
    fn next_id(&self) -> String;
}
pub struct UuidGenerator;
impl IdGenerator for UuidGenerator {
    fn next_id(&self) -> String {
        id()
    }
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn internal<E: std::fmt::Display>(_e: E) -> AppError {
    let mut e = AppError::new(
        "STORAGE_ERROR",
        "Không thể xử lý dữ liệu cục bộ; không xóa database để khôi phục.",
    );
    e.correlation_id = id();
    e
}
fn encode<T: serde::Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(internal)
}
fn decode<T: serde::de::DeserializeOwned>(v: &str) -> Result<T> {
    serde_json::from_str(v).map_err(internal)
}
fn hash(p: &ChangePayload) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(encode(p)?.as_bytes())))
}

pub struct Studio {
    pool: SqlitePool,
    session: String,
    clock: Arc<dyn Clock>,
    writer: Mutex<()>,
    ids: Arc<dyn IdGenerator>,
    gateway: Option<Arc<dyn cad_tool_host::CadGateway>>,
}
impl Studio {
    pub async fn open(path: &Path) -> Result<Self> {
        Self::open_with_clock(path, Arc::new(SystemClock)).await
    }
    pub async fn open_with_clock(path: &Path, clock: Arc<dyn Clock>) -> Result<Self> {
        Self::open_with_services(path, clock, Arc::new(UuidGenerator)).await
    }
    pub async fn open_with_services(
        path: &Path,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Result<Self> {
        // DELETE journal avoids relying on an unpatched bundled SQLite WAL implementation.
        // Switch to WAL only after runtime version and backup/recovery tests are pinned.
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Delete)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(internal)?;
        let enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .map_err(internal)?;
        if enabled != 1 {
            return Err(AppError::new(
                "FOREIGN_KEYS_DISABLED",
                "Database không bật foreign keys.",
            ));
        }
        sqlx::migrate!("../../database/migrations")
            .run(&pool)
            .await
            .map_err(internal)?;
        let app = Self {
            pool,
            session: id(),
            clock,
            writer: Mutex::new(()),
            ids,
            gateway: None,
        };
        app.agent_recover_interrupted().await?;
        Ok(app)
    }
    pub fn next_id(&self) -> String {
        self.ids.next_id()
    }
    pub async fn sqlite_version(&self) -> Result<String> {
        sqlx::query_scalar("SELECT sqlite_version()")
            .fetch_one(&self.pool)
            .await
            .map_err(internal)
    }
    pub async fn projects_list(&self) -> Result<Vec<Project>> {
        let rows = sqlx::query("SELECT id,name FROM projects ORDER BY created_at,id")
            .fetch_all(&self.pool)
            .await
            .map_err(internal)?;
        Ok(rows
            .into_iter()
            .map(|r| Project {
                id: r.get("id"),
                name: r.get("name"),
            })
            .collect())
    }
    pub async fn projects_create(&self, name: String) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 {
            return Err(AppError::new(
                "INVALID_NAME",
                "Tên dự án phải có 1–120 ký tự.",
            ));
        }
        let _guard = self.writer.lock().await;
        let p = Project {
            id: self.next_id(),
            name: name.into(),
        };
        let mut tx = self.pool.begin().await.map_err(internal)?;
        sqlx::query("INSERT INTO projects VALUES(?,?,?)")
            .bind(&p.id)
            .bind(&p.name)
            .bind(self.clock.now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("INSERT INTO documents(id,project_id,mode) VALUES(?,?,'mock')")
            .bind(format!("mock:{}", p.id))
            .bind(&p.id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        if let Some(gateway) = &self.gateway {
            gateway.ensure_document(&p.id).await?;
        }
        Ok(p)
    }
    async fn cached_context(&self, project_id: &str) -> Result<DocumentContext> {
        let r = sqlx::query("SELECT * FROM documents WHERE project_id=?")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "Không tìm thấy document thuộc dự án."))?;
        let json: Option<String> = r.get("geometry_json");
        Ok(DocumentContext {
            document_id: r.get("id"),
            project_id: project_id.into(),
            session_id: self.session.clone(),
            revision: r.get("revision"),
            geometry: json.map(|s| decode(&s)).transpose()?,
            mode: "mock".into(),
            units: Units::Millimeters,
        })
    }
    pub async fn propose(
        &self,
        project_id: &str,
        parameters: PlateParameters,
        mode: RunMode,
    ) -> Result<ChangeSet> {
        if mode == RunMode::Ask {
            return Err(AppError::new(
                "READ_ONLY",
                "Ask chỉ đọc; chọn Plan hoặc Execute để lập phương án.",
            ));
        }
        let _guard = self.writer.lock().await;
        let c = self.context(project_id).await?;
        let geometry = geometry::plan(parameters, &c.document_id)?;
        self.insert_change(&c, Some(geometry), mode, None).await
    }
    async fn insert_change(
        &self,
        c: &DocumentContext,
        after: Option<Geometry>,
        mode: RunMode,
        reverts: Option<String>,
    ) -> Result<ChangeSet> {
        let now = self.clock.now();
        let payload = ChangePayload {
            schema_version: 1,
            project_id: c.project_id.clone(),
            document_id: c.document_id.clone(),
            session_id: c.session_id.clone(),
            base_revision: c.revision,
            mode: mode.clone(),
            before: c.geometry.clone(),
            after,
            created_at: now,
            expires_at: now + 600,
            reverts,
        };
        let change = ChangeSet {
            id: self.next_id(),
            payload_hash: hash(&payload)?,
            status: if mode == RunMode::Plan {
                "planned"
            } else {
                "pending_approval"
            }
            .into(),
            payload,
        };
        sqlx::query("INSERT INTO change_sets VALUES(?,?,?,?,?,?)")
            .bind(&change.id)
            .bind(&c.document_id)
            .bind(encode(&change.payload)?)
            .bind(&change.payload_hash)
            .bind(&change.status)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        Ok(change)
    }
    async fn change(&self, change_id: &str) -> Result<ChangeSet> {
        let r = sqlx::query("SELECT * FROM change_sets WHERE id=?")
            .bind(change_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "Không tìm thấy phương án."))?;
        let external: Option<String> = sqlx::query_scalar(
            "SELECT state FROM gateway_operations WHERE change_set_id=? AND state != 'verified'",
        )
        .bind(change_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;
        let invalidated: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM proposal_invalidations WHERE change_set_id=?)",
        )
        .bind(change_id)
        .fetch_one(&self.pool)
        .await
        .map_err(internal)?;
        Ok(ChangeSet {
            id: r.get("id"),
            payload_hash: r.get("payload_hash"),
            status: if invalidated {
                Some("superseded".to_string())
            } else {
                external
            }
            .map(|state| {
                if state == "sending" {
                    "outcome_unknown".into()
                } else {
                    state
                }
            })
            .unwrap_or_else(|| r.get("status")),
            payload: decode(r.get("payload_json"))?,
        })
    }
    pub async fn changes_list(&self, project_id: &str) -> Result<Vec<ChangeSet>> {
        // History remains available even while the external CAD process is offline.
        let c = self.cached_context(project_id).await?;
        let ids:Vec<String>=sqlx::query_scalar("SELECT id FROM change_sets WHERE document_id=? ORDER BY created_at DESC,rowid DESC LIMIT 100").bind(c.document_id).fetch_all(&self.pool).await.map_err(internal)?;
        let mut out = Vec::new();
        for change_id in ids {
            out.push(self.change(&change_id).await?);
        }
        Ok(out)
    }
    pub async fn reject(&self, change_id: &str) -> Result<()> {
        let _guard = self.writer.lock().await;
        if sqlx::query_scalar::<_, String>(
            "SELECT state FROM gateway_operations WHERE change_set_id=?",
        )
        .bind(change_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?
        .is_some()
        {
            return Err(AppError::new(
                "RECOVERY_REQUIRED",
                "Thao tác đã gửi; cần đối chiếu kết quả trước.",
            ));
        }
        let r=sqlx::query("UPDATE change_sets SET status='rejected' WHERE id=? AND status IN ('pending_approval','planned')").bind(change_id).execute(&self.pool).await.map_err(internal)?;
        if r.rows_affected() != 1 {
            return Err(AppError::new(
                "INVALID_STATE",
                "Phương án không còn chờ xử lý.",
            ));
        }
        Ok(())
    }
    async fn approve_legacy(
        &self,
        change_id: &str,
        displayed_hash: &str,
    ) -> Result<ExecutionReceipt> {
        let _guard = self.writer.lock().await;
        let change = self.change(change_id).await?;
        if displayed_hash != change.payload_hash || hash(&change.payload)? != change.payload_hash {
            return Err(AppError::new(
                "HASH_MISMATCH",
                "Nội dung duyệt không khớp phương án.",
            ));
        }
        if let Some(receipt) = sqlx::query_scalar::<_, String>(
            "SELECT receipt_json FROM execution_receipts WHERE change_set_id=?",
        )
        .bind(change_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?
        {
            return decode(&receipt);
        }
        let p = &change.payload;
        if change.status == "superseded" {
            return Err(AppError::new(
                "CONFLICT",
                "Phương án đã bị thay thế bởi yêu cầu hoặc cấu hình mới. Hãy lập lại.",
            ));
        }
        if change.status != "pending_approval" || p.mode != RunMode::Execute {
            return Err(AppError::new(
                "APPROVAL_DENIED",
                "Chỉ phương án Execute đang chờ duyệt mới được áp dụng.",
            ));
        }
        if p.session_id != self.session {
            return Err(AppError::new(
                "SESSION_CHANGED",
                "Phiên đã thay đổi. Hãy lập phương án mới.",
            ));
        }
        if self.clock.now() >= p.expires_at {
            sqlx::query("UPDATE change_sets SET status='expired' WHERE id=?")
                .bind(change_id)
                .execute(&self.pool)
                .await
                .map_err(internal)?;
            return Err(AppError::new(
                "EXPIRED",
                "Phương án đã hết hạn sau 10 phút.",
            ));
        }
        let context = self.context(&p.project_id).await?;
        if context.document_id != p.document_id
            || context.revision != p.base_revision
            || context.geometry != p.before
        {
            sqlx::query("UPDATE change_sets SET status='conflict' WHERE id=?")
                .bind(change_id)
                .execute(&self.pool)
                .await
                .map_err(internal)?;
            return Err(AppError::new(
                "CONFLICT",
                "Document đã thay đổi. Cần lập và duyệt lại phương án.",
            ));
        }
        if let Some(g) = &p.after {
            geometry::verify(g, &p.document_id)?;
        }
        let mut tx = self.pool.begin().await.map_err(internal)?;
        let geometry_json = p.after.as_ref().map(encode).transpose()?;
        let updated = sqlx::query(
            "UPDATE documents SET geometry_json=?, revision=revision+1 WHERE id=? AND revision=?",
        )
        .bind(geometry_json)
        .bind(&p.document_id)
        .bind(p.base_revision)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
        if updated.rows_affected() != 1 {
            return Err(AppError::new("CONFLICT", "Revision thay đổi tại điểm ghi."));
        }
        let observed: Option<String> =
            sqlx::query_scalar("SELECT geometry_json FROM documents WHERE id=?")
                .bind(&p.document_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(internal)?;
        let observed: Option<Geometry> = observed.map(|s| decode(&s)).transpose()?;
        if observed != p.after {
            return Err(AppError::new(
                "READBACK_MISMATCH",
                "Dữ liệu lưu không khớp; transaction sẽ rollback.",
            ));
        }
        if let Some(g) = &observed {
            geometry::verify(g, &p.document_id)?;
        }
        let receipt = ExecutionReceipt {
            operation_id: change.id.clone(),
            change_set_id: change.id.clone(),
            payload_hash: change.payload_hash.clone(),
            observed_revision: p.base_revision + 1,
            status: "verified".into(),
            saved_to_dwg: false,
        };
        sqlx::query("INSERT INTO approvals VALUES(?,?,?,?,?,'approved')")
            .bind(id())
            .bind(change_id)
            .bind(displayed_hash)
            .bind(&self.session)
            .bind(self.clock.now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("INSERT INTO execution_receipts VALUES(?,?,?)")
            .bind(&receipt.operation_id)
            .bind(change_id)
            .bind(encode(&receipt)?)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query(
            "UPDATE change_sets SET status='verified' WHERE id=? AND status='pending_approval'",
        )
        .bind(change_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
        sqlx::query("INSERT INTO audit_events VALUES(?,?,?,?)")
            .bind(id())
            .bind(change_id)
            .bind("mock.change.verified")
            .bind(self.clock.now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        Ok(receipt)
    }
    pub async fn revert(&self, change_id: &str) -> Result<ChangeSet> {
        let _guard = self.writer.lock().await;
        let change = self.change(change_id).await?;
        if change.status != "verified" {
            return Err(AppError::new(
                "INVALID_STATE",
                "Chỉ hoàn tác thao tác đã xác minh.",
            ));
        }
        let c = self.context(&change.payload.project_id).await?;
        if c.revision != change.payload.base_revision + 1 || c.geometry != change.payload.after {
            return Err(AppError::new(
                "CONFLICT",
                "Đã có thay đổi tiếp theo; không thể hoàn tác mù.",
            ));
        }
        self.insert_change(&c, change.payload.before, RunMode::Execute, Some(change.id))
            .await
    }
}
#[cfg(test)]
mod tests;
