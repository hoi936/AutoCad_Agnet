use crate::{bounded::BoundedRead, permit::PermitAuthority, state::Bootstrap};
use async_trait::async_trait;
use cad_contracts::*;
use rmcp::{model::CallToolRequestParams, service::RunningService, RoleClient, ServiceExt};
use serde_json::{json, Value};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

#[async_trait]
// async-trait emits must_use on its boxed Future (Rust 1.99 Clippy).
#[allow(clippy::double_must_use)]
pub trait CadGateway: Send + Sync {
    async fn context(&self, project: &str) -> Result<DocumentContext, AppError>;
    async fn ensure_document(&self, project: &str) -> Result<(), AppError>;
    async fn apply(&self, change: &ChangeSet) -> Result<ExecutionReceipt, AppError>;
    async fn operation(&self, id: &str) -> Result<Option<ExecutionReceipt>, AppError>;
    async fn read(&self, tool: &str, project: &str) -> Result<Value, AppError>;
    async fn shutdown(&self);
}
pub struct McpGateway {
    service: Mutex<Option<RunningService<RoleClient, ()>>>,
    child: Mutex<Child>,
    authority: PermitAuthority,
    session: String,
}
fn disconnected<E>(_error: E) -> AppError {
    AppError::new(
        "CAD_DISCONNECTED",
        "Mất kết nối MCP CAD. Không tự gửi lại thao tác ghi; khởi động lại ứng dụng để đối chiếu.",
    )
}
impl McpGateway {
    pub async fn spawn(
        binary: &Path,
        snapshot: &Path,
        seeds: Vec<DocumentContext>,
        session: String,
        allow_initial_import: bool,
    ) -> Result<Self, AppError> {
        if !binary.is_absolute()
            || binary.file_name().and_then(|s| s.to_str())
                != Some(if cfg!(windows) {
                    "cad-tool-host.exe"
                } else {
                    "cad-tool-host"
                })
        {
            return Err(AppError::new(
                "SIDECAR_PATH",
                "Thiếu sidecar cố định của ứng dụng.",
            ));
        }
        let mut command = Command::new(binary);
        command
            .arg("--paired-stdio")
            .env_clear()
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // No provider credential, user-controlled command or PATH is inherited.
        for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let mut child = command.spawn().map_err(disconnected)?;
        let input = child.stdin.take().ok_or_else(|| disconnected(()))?;
        let output = child.stdout.take().ok_or_else(|| disconnected(()))?;
        let service = tokio::time::timeout(
            Duration::from_secs(10),
            ().serve((BoundedRead::new(output), input)),
        )
        .await
        .map_err(disconnected)?
        .map_err(disconnected)?;
        let authority = PermitAuthority::random();
        let gateway = Self {
            service: Mutex::new(Some(service)),
            child: Mutex::new(child),
            authority,
            session: session.clone(),
        };
        let discovered = gateway.discover().await?;
        for required in [
            "cad.get_status",
            "cad.get_document_context",
            "cad.apply_change_set",
            "cad.get_operation_status",
        ] {
            if !discovered.iter().any(|n| n == required) {
                return Err(AppError::new(
                    "MCP_CAPABILITY",
                    "Sidecar thiếu công cụ bắt buộc.",
                ));
            }
        }
        gateway
            .call(
                "cad.bootstrap",
                serde_json::to_value(Bootstrap {
                    session_id: session,
                    key: gateway.authority.bootstrap_key(),
                    snapshot_path: snapshot.to_owned(),
                    seeds,
                    allow_initial_import,
                })
                .map_err(disconnected)?,
            )
            .await?;
        Ok(gateway)
    }
    pub async fn discover(&self) -> Result<Vec<String>, AppError> {
        let guard = self.service.lock().await;
        let service = guard.as_ref().ok_or_else(|| disconnected(()))?;
        let tools = tokio::time::timeout(Duration::from_secs(10), service.list_all_tools())
            .await
            .map_err(disconnected)?
            .map_err(disconnected)?;
        if tools.len() > 32 {
            return Err(AppError::new("MCP_LIMIT", "Quá nhiều công cụ."));
        }
        Ok(tools.into_iter().map(|t| t.name.into_owned()).collect())
    }
    async fn call(&self, name: &str, args: Value) -> Result<Value, AppError> {
        if serde_json::to_vec(&args).map_err(disconnected)?.len() > 512 * 1024 {
            return Err(AppError::new("MCP_LIMIT", "Payload quá lớn."));
        }
        if self
            .child
            .lock()
            .await
            .try_wait()
            .map_err(disconnected)?
            .is_some()
        {
            return Err(disconnected(()));
        }
        let guard = self.service.lock().await;
        let service = guard.as_ref().ok_or_else(|| disconnected(()))?;
        let request = CallToolRequestParams::new(name.to_owned())
            .with_arguments(args.as_object().cloned().ok_or_else(|| disconnected(()))?);
        let result = tokio::time::timeout(Duration::from_secs(15), service.call_tool(request))
            .await
            .map_err(disconnected)?
            .map_err(disconnected)?;
        let value = result.structured_content.ok_or_else(|| disconnected(()))?;
        if result.is_error == Some(true) {
            return Err(serde_json::from_value(value).unwrap_or_else(|_| disconnected(())));
        }
        Ok(value)
    }
    #[cfg(feature = "fault-injection")]
    pub async fn inject_disconnect_after_commit(&self) -> Result<(), AppError> {
        self.call("test.drop_after_commit", json!({}))
            .await
            .map(|_| ())
    }
}
#[async_trait]
impl CadGateway for McpGateway {
    async fn context(&self, project: &str) -> Result<DocumentContext, AppError> {
        serde_json::from_value(
            self.call("cad.get_document_context", json!({"project_id":project}))
                .await?,
        )
        .map_err(disconnected)
    }
    async fn ensure_document(&self, project: &str) -> Result<(), AppError> {
        self.call("cad.ensure_document",json!({"project_id":project,"proof":self.authority.document_proof(project,&self.session)})).await.map(|_|())
    }
    async fn apply(&self, change: &ChangeSet) -> Result<ExecutionReceipt, AppError> {
        let permit = self
            .authority
            .issue(&change.id, &change.payload, crate::now() + 60)?;
        let tool = if change.payload.reverts.is_some() {
            "cad.revert_change_set"
        } else {
            "cad.apply_change_set"
        };
        serde_json::from_value(
            self.call(tool, json!({"change":change,"permit":permit}))
                .await?,
        )
        .map_err(disconnected)
    }
    async fn operation(&self, id: &str) -> Result<Option<ExecutionReceipt>, AppError> {
        serde_json::from_value(
            self.call("cad.get_operation_status", json!({"operation_id":id}))
                .await?,
        )
        .map_err(disconnected)
    }
    async fn read(&self, tool: &str, project: &str) -> Result<Value, AppError> {
        match tool {
            "cad.get_status" => self.call(tool, json!({})).await,
            "cad.get_document_context"
            | "cad.get_selection"
            | "cad.query_entities"
            | "cad.validate_entities" => self.call(tool, json!({"project_id":project})).await,
            _ => Err(AppError::new(
                "TOOL_DENIED",
                "Công cụ không nằm trong allowlist đọc.",
            )),
        }
    }
    async fn shutdown(&self) {
        if let Some(service) = self.service.lock().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), service.cancel()).await;
        }
        let mut child = self.child.lock().await;
        if tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .is_err()
        {
            let _ = child.kill().await;
        }
    }
}
