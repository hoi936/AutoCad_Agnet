use crate::bounded::BoundedRead;
use cad_contracts::AppError;
use rmcp::{model::CallToolRequestParams, service::RunningService, RoleClient, ServiceExt};
use serde_json::{json, Value};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{process::{Child, Command}, sync::Mutex};

pub struct LiveMcpGateway { service: Mutex<Option<RunningService<RoleClient, ()>>>, child: Mutex<Child> }
fn disconnected<E>(_e: E) -> AppError { AppError::new("CAD_DISCONNECTED", "Mất kết nối MCP với AutoCAD.") }
impl LiveMcpGateway {
    pub async fn pair(binary: &Path, pid: u32, code: &str) -> Result<(Self, Value), AppError> {
        if !binary.is_absolute() || binary.file_name().and_then(|s| s.to_str()) != Some(if cfg!(windows) { "cad-tool-host.exe" } else { "cad-tool-host" }) {
            return Err(AppError::new("SIDECAR_PATH", "Thiếu sidecar của ứng dụng."));
        }
        let mut command = Command::new(binary);
        command.arg("--live-read-stdio").env_clear().kill_on_drop(true).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] { if let Some(v) = std::env::var_os(name) { command.env(name, v); } }
        let mut child = command.spawn().map_err(disconnected)?;
        let input = child.stdin.take().ok_or_else(|| disconnected(()))?;
        let output = child.stdout.take().ok_or_else(|| disconnected(()))?;
        let service = tokio::time::timeout(Duration::from_secs(10), ().serve((BoundedRead::new(output), input))).await.map_err(disconnected)?.map_err(disconnected)?;
        let gateway = Self { service: Mutex::new(Some(service)), child: Mutex::new(child) };
        let names = tokio::time::timeout(Duration::from_secs(10), async {
            let guard = gateway.service.lock().await;
            guard.as_ref().ok_or_else(|| disconnected(()))?.list_all_tools().await.map_err(disconnected)
        }).await.map_err(disconnected)??;
        for required in ["cad.pair","cad.get_document_context","cad.get_selection","cad.list_layers"] {
            if !names.iter().any(|n| n.name == required) { return Err(AppError::new("MCP_CAPABILITY", "Sidecar thiếu công cụ đọc bắt buộc.")); }
        }
        let result = gateway.call("cad.pair", json!({"pid":pid,"code":code})).await?;
        if result["connected"] != true || result["capabilities"]["write"] != false { return Err(AppError::new("PAIR_DENIED", "Bridge chưa xác minh chỉ đọc.")); }
        Ok((gateway, result))
    }
    pub async fn call(&self, name: &str, args: Value) -> Result<Value, AppError> {
        if !matches!(name,"cad.pair"|"cad.disconnect"|"cad.get_status"|"cad.list_documents"|"cad.get_document_context"|"cad.get_selection"|"cad.list_layers"|"cad.query_entities"|"cad.validate_entities"|"cad.get_operation_status") {
            return Err(AppError::new("READ_ONLY_MODE", "Công cụ không có trong R2."));
        }
        if self.child.lock().await.try_wait().map_err(disconnected)?.is_some() { return Err(disconnected(())); }
        let guard = self.service.lock().await;
        let service = guard.as_ref().ok_or_else(|| disconnected(()))?;
        let request = CallToolRequestParams::new(name.to_owned()).with_arguments(args.as_object().cloned().ok_or_else(|| AppError::new("INVALID_ARGUMENTS", "Arguments phải là object."))?);
        let result = tokio::time::timeout(Duration::from_secs(18), service.call_tool(request)).await.map_err(disconnected)?.map_err(disconnected)?;
        let value = result.structured_content.ok_or_else(|| disconnected(()))?;
        if result.is_error == Some(true) { return Err(serde_json::from_value(value).unwrap_or_else(disconnected)); }
        Ok(value)
    }
    pub async fn shutdown(&self) {
        let _ = self.call("cad.disconnect", json!({})).await;
        self.service.lock().await.take();
        let _ = self.child.lock().await.kill().await;
    }
}
