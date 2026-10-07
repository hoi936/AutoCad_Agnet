use super::*;
use cad_tool_host::{CadGateway, McpGateway};
use serde_json::{json, Value};

#[async_trait::async_trait]
// async-trait emits must_use on its boxed Future (Rust 1.99 Clippy).
#[allow(clippy::double_must_use)]
pub trait ApprovalService {
    async fn approve_change(&self, id: &str, hash: &str) -> Result<ExecutionReceipt>;
    async fn reject_change(&self, id: &str) -> Result<()>;
    async fn propose_revert(&self, id: &str) -> Result<ChangeSet>;
}
#[async_trait::async_trait]
impl ApprovalService for Studio {
    async fn approve_change(&self, id: &str, hash: &str) -> Result<ExecutionReceipt> {
        self.approve(id, hash).await
    }
    async fn reject_change(&self, id: &str) -> Result<()> {
        self.reject(id).await
    }
    async fn propose_revert(&self, id: &str) -> Result<ChangeSet> {
        self.revert(id).await
    }
}
impl Studio {
    /// Production desktop/demo constructor. `open` remains an explicit in-process
    /// development/test harness, never selected as a fallback after sidecar failure.
    pub async fn open_mcp(database: &Path, binary: &Path, snapshot: &Path) -> Result<Self> {
        let mut app = Self::open(database).await?;
        let known: Option<String> =
            sqlx::query_scalar("SELECT value FROM gateway_state WHERE key='snapshot_initialized'")
                .fetch_optional(&app.pool)
                .await
                .map_err(internal)?;
        if known.is_some() && !snapshot.exists() {
            return Err(AppError::new("RECOVERY_REQUIRED","Snapshot CAD bị thiếu. Không tạo lại bằng cache SQLite; khôi phục snapshot trước khi tiếp tục."));
        }
        let parent = snapshot.parent().ok_or_else(|| internal("snapshot"))?;
        std::fs::create_dir_all(parent).map_err(internal)?;
        let mut seeds = vec![];
        for p in app.projects_list().await? {
            seeds.push(app.cached_context(&p.id).await?);
        }
        let gateway = Arc::new(
            McpGateway::spawn(
                binary,
                snapshot,
                seeds,
                app.session.clone(),
                known.is_none(),
            )
            .await?,
        );
        sqlx::query(
            "INSERT OR IGNORE INTO gateway_state(key,value) VALUES('snapshot_initialized','1')",
        )
        .execute(&app.pool)
        .await
        .map_err(internal)?;
        sqlx::query("UPDATE gateway_operations SET state='outcome_unknown' WHERE state='sending'")
            .execute(&app.pool)
            .await
            .map_err(internal)?;
        app.gateway = Some(gateway);
        Ok(app)
    }
    pub async fn shutdown(&self) {
        if let Some(gateway) = &self.gateway {
            gateway.shutdown().await;
        }
        self.pool.close().await;
    }
    pub async fn context(&self, project_id: &str) -> Result<DocumentContext> {
        let local = self.cached_context(project_id).await?;
        match &self.gateway {
            Some(gateway) => gateway.context(project_id).await,
            None => Ok(local),
        }
    }
    pub async fn cad_read(&self, tool: &str, project: &str) -> Result<Value> {
        self.cached_context(project).await?;
        if let Some(gateway) = &self.gateway {
            return gateway.read(tool, project).await;
        }
        let context = self.context(project).await?;
        match tool {
            "cad.get_status" => {
                Ok(json!({"mode":"mock","connected":true,"transport":"in_process_test"}))
            }
            "cad.get_document_context" => serde_json::to_value(context).map_err(internal),
            "cad.query_entities" => {
                Ok(json!({"entities":context.geometry.map(|g|g.entities).unwrap_or_default()}))
            }
            "cad.get_selection" => Ok(json!({"entities":[]})),
            "cad.validate_entities" => {
                if let Some(g) = &context.geometry {
                    geometry::verify(g, &context.document_id)?;
                }
                Ok(json!({"valid":true,"revision":context.revision}))
            }
            _ => Err(AppError::new(
                "TOOL_DENIED",
                "Công cụ không nằm trong allowlist đọc.",
            )),
        }
    }
    pub async fn approve(&self, change_id: &str, displayed_hash: &str) -> Result<ExecutionReceipt> {
        let Some(gateway) = &self.gateway else {
            return self.approve_legacy(change_id, displayed_hash).await;
        };
        let _guard = self.writer.lock().await;
        let change = self.change(change_id).await?;
        if change.payload_hash != displayed_hash || hash(&change.payload)? != displayed_hash {
            return Err(AppError::new(
                "HASH_MISMATCH",
                "Nội dung đang duyệt không khớp phương án.",
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
                "Thao tác đã được gửi trước đó. Hãy đối chiếu trạng thái; không gửi lại mutation.",
            ));
        }
        let p = &change.payload;
        let unresolved:i64=sqlx::query_scalar("SELECT COUNT(*) FROM gateway_operations g JOIN change_sets c ON c.id=g.change_set_id JOIN documents d ON d.id=c.document_id WHERE d.project_id=? AND g.state IN ('sending','outcome_unknown','recovery_required')").bind(&p.project_id).fetch_one(&self.pool).await.map_err(internal)?;
        if unresolved > 0 {
            return Err(AppError::new(
                "RECOVERY_REQUIRED",
                "Dự án còn thao tác chưa xác minh; đối chiếu trước khi ghi tiếp.",
            ));
        }
        if change.status == "superseded" {
            return Err(AppError::new(
                "CONFLICT",
                "Phương án đã bị thay thế bởi yêu cầu hoặc cấu hình mới. Hãy lập lại.",
            ));
        }
        if change.status != "pending_approval" || p.mode != RunMode::Execute {
            return Err(AppError::new(
                "APPROVAL_DENIED",
                "Phương án không có quyền ghi.",
            ));
        }
        if self.clock.now() >= p.expires_at {
            return Err(AppError::new(
                "EXPIRED",
                "Phương án đã hết hạn; lập lại preview.",
            ));
        }
        let actual = gateway.context(&p.project_id).await?;
        if actual.session_id != p.session_id {
            return Err(AppError::new(
                "SESSION_CHANGED",
                "Phiên CAD thay đổi; lập lại preview.",
            ));
        }
        if actual.document_id != p.document_id
            || actual.revision != p.base_revision
            || actual.geometry != p.before
        {
            return Err(AppError::new(
                "CONFLICT",
                "CAD thay đổi sau preview; không áp dụng.",
            ));
        }
        if let Some(g) = &p.after {
            geometry::verify(g, &p.document_id)?;
        }
        let mut tx = self.pool.begin().await.map_err(internal)?;
        sqlx::query("INSERT INTO approvals VALUES(?,?,?,?,?,'approved')")
            .bind(self.next_id())
            .bind(change_id)
            .bind(displayed_hash)
            .bind(&p.session_id)
            .bind(self.clock.now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("INSERT INTO gateway_operations VALUES(?,?,?,'sending',?)")
            .bind(change_id)
            .bind(change_id)
            .bind(displayed_hash)
            .bind(self.clock.now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        match gateway.apply(&change).await {
            Ok(receipt) => match self
                .finish_external(&change, &receipt, gateway.as_ref())
                .await
            {
                Ok(()) => Ok(receipt),
                Err(_) => {
                    self.mark_unknown(change_id).await?;
                    Err(AppError::new(
                        "OUTCOME_UNKNOWN",
                        "CAD có thể đã ghi; cần đối chiếu receipt/readback, không gửi lại.",
                    ))
                }
            },
            Err(error) => {
                self.mark_unknown(change_id).await?;
                Err(AppError::new(
                    "OUTCOME_UNKNOWN",
                    if error.code == "CAD_DISCONNECTED" {
                        "Mất kết nối sau khi gửi; cần đối chiếu, không tự vẽ lại."
                    } else {
                        "Không xác minh được kết quả mutation; cần đối chiếu trạng thái."
                    },
                ))
            }
        }
    }
    async fn mark_unknown(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE gateway_operations SET state='outcome_unknown',updated_at=? WHERE change_set_id=?").bind(self.clock.now()).bind(id).execute(&self.pool).await.map_err(internal)?;
        Ok(())
    }
    async fn finish_external(
        &self,
        change: &ChangeSet,
        receipt: &ExecutionReceipt,
        gateway: &dyn CadGateway,
    ) -> Result<()> {
        let p = &change.payload;
        let actual = gateway.context(&p.project_id).await?;
        if receipt.change_set_id != change.id
            || receipt.status != "verified"
            || receipt.saved_to_dwg
            || actual.document_id != p.document_id
            || receipt.operation_id != change.id
            || receipt.payload_hash != change.payload_hash
            || receipt.observed_revision != p.base_revision + 1
            || actual.revision != receipt.observed_revision
            || actual.geometry != p.after
        {
            return Err(AppError::new(
                "RECOVERY_REQUIRED",
                "Receipt và hình học hiện tại không khớp; cần kiểm tra thủ công.",
            ));
        }
        if let Some(g) = &actual.geometry {
            geometry::verify(g, &p.document_id)?;
        }
        let report = gateway.read("cad.validate_entities", &p.project_id).await?;
        if report["valid"] != true {
            return Err(AppError::new(
                "VALIDATION_FAILED",
                "CAD validator không xác nhận.",
            ));
        }
        let mut tx = self.pool.begin().await.map_err(internal)?;
        sqlx::query("UPDATE documents SET geometry_json=?,revision=? WHERE id=?")
            .bind(actual.geometry.as_ref().map(encode).transpose()?)
            .bind(actual.revision)
            .bind(&actual.document_id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("INSERT OR IGNORE INTO execution_receipts VALUES(?,?,?)")
            .bind(&receipt.operation_id)
            .bind(&change.id)
            .bind(encode(receipt)?)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("UPDATE change_sets SET status='verified' WHERE id=?")
            .bind(&change.id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query(
            "UPDATE gateway_operations SET state='verified',updated_at=? WHERE change_set_id=?",
        )
        .bind(self.clock.now())
        .bind(&change.id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
        sqlx::query("INSERT INTO audit_events VALUES(?,?,?,?)")
            .bind(self.next_id())
            .bind(&change.id)
            .bind("mcp.mock.verified")
            .bind(self.clock.now())
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        Ok(())
    }
    pub async fn recover_change(&self, id: &str) -> Result<ExecutionReceipt> {
        let _guard = self.writer.lock().await;
        let gateway = self.gateway.as_ref().ok_or_else(|| {
            AppError::new(
                "NOT_MCP",
                "Đối chiếu ngoài process chỉ dùng trong MCP mode.",
            )
        })?;
        let change = self.change(id).await?;
        let state: Option<String> =
            sqlx::query_scalar("SELECT state FROM gateway_operations WHERE change_set_id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(internal)?;
        if state.is_none() {
            return Err(AppError::new("NOT_SENT", "Phương án chưa được gửi."));
        }
        if let Some(receipt) = gateway.operation(id).await? {
            self.finish_external(&change, &receipt, gateway.as_ref())
                .await?;
            return Ok(receipt);
        }
        let actual = gateway.context(&change.payload.project_id).await?;
        if actual.revision == change.payload.base_revision
            && actual.geometry == change.payload.before
        {
            sqlx::query("UPDATE gateway_operations SET state='not_applied' WHERE change_set_id=?")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(internal)?;
            return Err(AppError::new(
                "NOT_APPLIED",
                "Snapshot xác nhận chưa áp dụng. Tạo phương án mới; không retry operation cũ.",
            ));
        }
        sqlx::query(
            "UPDATE gateway_operations SET state='recovery_required' WHERE change_set_id=?",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(internal)?;
        Err(AppError::new(
            "RECOVERY_REQUIRED",
            "Không đủ bằng chứng applied/not-applied; cần kiểm tra snapshot.",
        ))
    }
}
