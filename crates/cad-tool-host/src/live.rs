//! R2 read-only MCP server. It has no mutation tools and never receives provider secrets.
use cad_contracts::AppError;
use rmcp::{model::*, service::RequestContext, RoleServer, ServerHandler};
use serde_json::{json, Value};
use tokio::sync::Mutex;
#[cfg(windows)]
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{ClientOptions, NamedPipeClient}};

const READ_TOOLS: &[(&str, &str)] = &[
    ("cad.get_status", "get_status"),
    ("cad.list_documents", "list_documents"),
    ("cad.get_document_context", "get_document_context"),
    ("cad.get_selection", "get_selection"),
    ("cad.list_layers", "list_layers"),
    ("cad.query_entities", "query_entities"),
    ("cad.validate_entities", "validate_entities"),
    ("cad.get_operation_status", "get_operation_status"),
];
const MAX_FRAME: usize = 65536;

#[cfg(windows)]
struct Session { pipe: NamedPipeClient, id: String, document_id: String }
#[cfg(not(windows))]
struct Session;
pub struct LiveServer { session: Mutex<Option<Session>> }
impl LiveServer {
    pub fn new() -> Self { Self { session: Mutex::new(None) } }
    async fn dispatch(&self, name: &str, args: Value) -> Result<Value, AppError> {
        if serde_json::to_vec(&args).map_err(invalid)?.len() > 8192 { return Err(error("PAYLOAD_TOO_LARGE", "Arguments too large.")); }
        #[cfg(not(windows))]
        { let _ = (name, args); Err(error("WINDOWS_REQUIRED", "Live bridge requires Windows.")) }
        #[cfg(windows)]
        { self.dispatch_windows(name, args).await }
    }
    #[cfg(windows)]
    async fn dispatch_windows(&self, name: &str, args: Value) -> Result<Value, AppError> {
        if name == "cad.pair" {
            let pid = args.get("pid").and_then(Value::as_u64).filter(|v| *v > 0 && *v <= u32::MAX as u64).ok_or_else(|| error("INVALID_ARGUMENTS", "PID required."))? as u32;
            let code = args.get("code").and_then(Value::as_str).filter(|v| v.len() == 32 && v.bytes().all(|b| b.is_ascii_hexdigit())).ok_or_else(|| error("INVALID_ARGUMENTS", "Pairing code invalid."))?;
            let path = format!(r"\\.\pipe\cadagent-r2-{pid}");
            let pipe = ClientOptions::new().open(&path).map_err(|_| error("PLUGIN_NOT_LOADED", "AutoCAD bridge pipe unavailable."))?;
            if !crate::pipe_identity::server_is_pid(&pipe, pid) { return Err(error("PROCESS_MISMATCH", "Bridge pipe does not belong to selected AutoCAD process.")); }
            let mut candidate = Session { pipe, id: String::new(), document_id: String::new() };
            let response = candidate.call("pair", None, json!({"code":code,"expected_pid":pid})).await?;
            candidate.id = response.get("session_id").and_then(Value::as_str).ok_or_else(|| error("PAIR_DENIED", "Session missing."))?.to_owned();
            if response["result"]["process_id"] != pid || response["result"]["read_only"] != true || response["result"]["write"] != false {
                return Err(error("PROTOCOL_MISMATCH", "Bridge capabilities mismatch."));
            }
            let docs = candidate.call("list_documents", None, json!({})).await?;
            let doc = docs["result"]["documents"].as_array().and_then(|docs| docs.iter().find(|d| d["active"] == true)).ok_or_else(|| error("NO_DOCUMENT", "Open a drawing in AutoCAD."))?;
            let document_id = doc["document_id"].as_str().ok_or_else(|| error("PROTOCOL_MISMATCH", "Document ID missing."))?.to_owned();
            let context = candidate.call("get_document_context", Some(&document_id), json!({})).await?;
            if context["result"]["document_id"] != document_id { return Err(error("DOCUMENT_CHANGED", "Document readback mismatch.")); }
            candidate.document_id = document_id;
            let mut guard = self.session.lock().await;
            *guard = Some(candidate);
            return Ok(json!({"connected":true,"pid":pid,"document":context["result"],"capabilities":{"read":true,"write":false}}));
        }
        let mut guard = self.session.lock().await;
        let session = guard.as_mut().ok_or_else(|| error("CAD_DISCONNECTED", "AutoCAD not paired."))?;
        if name == "cad.disconnect" {
            let _ = session.call("disconnect", None, json!({})).await;
            *guard = None;
            return Ok(json!({"connected":false}));
        }
        let operation = READ_TOOLS.iter().find(|(tool, _)| *tool == name).map(|(_, op)| *op).ok_or_else(|| error("READ_ONLY_MODE", "Tool unavailable in R2."))?;
        let supplied_doc = args.get("document_id").and_then(Value::as_str);
        if let Some(doc) = supplied_doc { if doc != session.document_id { return Err(error("DOCUMENT_CHANGED", "Re-select active drawing.")); } }
        let doc_id = if matches!(operation, "get_status" | "list_documents" | "get_operation_status") { None } else { Some(session.document_id.clone()) };
        let response = session.call(operation, doc_id.as_deref(), args).await?;
        if operation == "get_status" && response["result"]["active_document_id"] != session.document_id { return Err(error("DOCUMENT_CHANGED", "Active drawing changed.")); }
        Ok(response["result"].clone())
    }
}
impl Default for LiveServer { fn default() -> Self { Self::new() } }
fn error(code: &'static str, message: &'static str) -> AppError { AppError::new(code, message) }
fn invalid<E>(_e: E) -> AppError { error("INVALID_ARGUMENTS", "Invalid JSON arguments.") }

#[cfg(windows)]
impl Session {
    async fn call(&mut self, operation: &str, document_id: Option<&str>, args: Value) -> Result<Value, AppError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = json!({"version":1,"request_id":request_id,"session_id":if self.id.is_empty() {None} else {Some(self.id.as_str())},"operation":operation,"document_id":document_id,"args":args});
        let bytes = serde_json::to_vec(&request).map_err(invalid)?;
        if bytes.len() > MAX_FRAME { return Err(error("PAYLOAD_TOO_LARGE", "Bridge frame too large.")); }
        let mut header = (bytes.len() as u32).to_le_bytes();
        let io = async {
            self.pipe.write_all(&header).await.map_err(disconnected)?;
            self.pipe.write_all(&bytes).await.map_err(disconnected)?;
            self.pipe.flush().await.map_err(disconnected)?;
            self.pipe.read_exact(&mut header).await.map_err(disconnected)?;
            let len = u32::from_le_bytes(header) as usize;
            if len < 2 || len > MAX_FRAME { return Err(error("FRAME_SIZE", "Bridge frame invalid.")); }
            let mut output = vec![0u8; len];
            self.pipe.read_exact(&mut output).await.map_err(disconnected)?;
            let value: Value = serde_json::from_slice(&output).map_err(invalid)?;
            if value["version"] != 1 || value["request_id"] != request_id { return Err(error("PROTOCOL_MISMATCH", "Bridge response mismatch.")); }
            if value["ok"] != true {
                let code = value["error"]["code"].as_str().unwrap_or("BRIDGE_ERROR");
                return Err(AppError::new(code, "AutoCAD read failed."));
            }
            Ok(value)
        };
        tokio::time::timeout(std::time::Duration::from_secs(15), io).await.map_err(|_| error("CAD_BUSY", "AutoCAD read timed out."))?
    }
}
#[cfg(windows)]
fn disconnected<E>(_e: E) -> AppError { error("CAD_DISCONNECTED", "AutoCAD bridge disconnected.") }
fn tool(name: &str) -> Tool {
    let schema = if name == "cad.pair" { json!({"type":"object","properties":{"pid":{"type":"integer"},"code":{"type":"string"}},"required":["pid","code"],"additionalProperties":false}) }
    else if name == "cad.query_entities" { json!({"type":"object","properties":{"document_id":{"type":"string"},"max_results":{"type":"integer","minimum":1,"maximum":100},"layer":{"type":"string"},"entity_type":{"type":"string"}},"additionalProperties":false}) }
    else if name == "cad.validate_entities" { json!({"type":"object","properties":{"document_id":{"type":"string"},"handles":{"type":"array","items":{"type":"string"},"maxItems":100}},"required":["handles"],"additionalProperties":false}) }
    else { json!({"type":"object","properties":{"document_id":{"type":"string"}},"additionalProperties":false}) };
    Tool::new(name.to_owned(), "Read-only AutoCAD live tool", schema.as_object().expect("object").clone())
}
impl ServerHandler for LiveServer {
    fn get_info(&self) -> ServerConfig { ServerConfig::new(ServerCapabilities::builder().enable_tools().build()) }
    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult { tools: READ_TOOLS.iter().map(|(name,_)|tool(name)).chain([tool("cad.pair"),tool("cad.disconnect")]).collect(), ..Default::default() })
    }
    async fn call_tool(&self, request: CallToolRequestParams, _context: RequestContext<RoleServer>) -> Result<CallToolResponse, rmcp::ErrorData> {
        let result = self.dispatch(&request.name, Value::Object(request.arguments.unwrap_or_default())).await;
        Ok(match result { Ok(value) => CallToolResult::structured(value), Err(e) => CallToolResult::structured_error(serde_json::to_value(e).expect("error DTO")) }.into())
    }
}
