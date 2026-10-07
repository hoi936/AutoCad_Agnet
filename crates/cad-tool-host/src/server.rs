use crate::state::{Bootstrap, MockCadGateway, MutationRequest};
use cad_contracts::AppError;
use rmcp::{model::*, service::RequestContext, RoleServer, ServerHandler};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

pub const TOOLS: &[&str] = &[
    "cad.get_status",
    "cad.get_document_context",
    "cad.get_selection",
    "cad.query_entities",
    "cad.validate_entities",
    "cad.get_operation_status",
    "cad.apply_change_set",
    "cad.revert_change_set",
    "cad.bootstrap",
    "cad.ensure_document",
];
pub struct CadServer {
    paired: bool,
    state: Mutex<Option<MockCadGateway>>,
    #[cfg(feature = "fault-injection")]
    drop_after_commit: std::sync::atomic::AtomicBool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectArg {
    project_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationArg {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnsureArg {
    project_id: String,
    proof: Vec<u8>,
}
impl CadServer {
    pub fn new(paired: bool) -> Self {
        Self {
            paired,
            state: Mutex::new(None),
            #[cfg(feature = "fault-injection")]
            drop_after_commit: std::sync::atomic::AtomicBool::new(false),
        }
    }
    async fn dispatch(&self, name: &str, args: Value) -> Result<Value, AppError> {
        if serde_json::to_vec(&args).map_err(invalid)?.len() > 512 * 1024 {
            return Err(AppError::new("PAYLOAD_TOO_LARGE", "Payload vượt giới hạn."));
        }
        let mut state = self.state.lock().await;
        if name == "cad.get_status" {
            return Ok(json!({"mode":"mock","connected":state.is_some(),"schema_version":1}));
        }
        if name == "cad.bootstrap" {
            if !self.paired || state.is_some() {
                return Err(AppError::new(
                    "BOOTSTRAP_DENIED",
                    "Process không được ghép cặp hoặc phiên đã được thiết lập.",
                ));
            }
            *state = Some(MockCadGateway::bootstrap(
                serde_json::from_value::<Bootstrap>(args).map_err(invalid)?,
            )?);
            return Ok(json!({"connected":true}));
        }
        let gateway = state
            .as_mut()
            .ok_or_else(|| AppError::new("NOT_PAIRED", "Chưa thiết lập phiên CAD tin cậy."))?;
        match name {
            "cad.ensure_document" => {
                let a: EnsureArg = serde_json::from_value(args).map_err(invalid)?;
                gateway.ensure_document(&a.project_id, &a.proof)?;
                Ok(json!({"ready":true}))
            }
            "cad.get_operation_status" => {
                let a: OperationArg = serde_json::from_value(args).map_err(invalid)?;
                serde_json::to_value(gateway.operation(&a.operation_id)).map_err(invalid)
            }
            "cad.apply_change_set" | "cad.revert_change_set" => {
                let request: MutationRequest = serde_json::from_value(args).map_err(invalid)?;
                if (name == "cad.revert_change_set") != request.change.payload.reverts.is_some() {
                    return Err(AppError::new(
                        "INVALID_SCOPE",
                        "Tool và phạm vi hoàn tác không khớp.",
                    ));
                }
                let receipt = gateway.apply(request)?;
                #[cfg(feature = "fault-injection")]
                if self
                    .drop_after_commit
                    .swap(false, std::sync::atomic::Ordering::SeqCst)
                {
                    std::process::exit(77);
                }
                serde_json::to_value(receipt).map_err(invalid)
            }
            #[cfg(feature = "fault-injection")]
            "test.drop_after_commit" => {
                self.drop_after_commit
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(json!({"armed":true}))
            }
            "cad.get_document_context"
            | "cad.get_selection"
            | "cad.query_entities"
            | "cad.validate_entities" => {
                let a: ProjectArg = serde_json::from_value(args).map_err(invalid)?;
                let context = gateway.context(&a.project_id)?;
                match name {
                    "cad.get_document_context" => serde_json::to_value(context).map_err(invalid),
                    "cad.get_selection" => Ok(json!({"entities":[],"mode":"mock"})),
                    "cad.query_entities" => Ok(
                        json!({"entities":context.geometry.map(|g|g.entities).unwrap_or_default(),"revision":context.revision}),
                    ),
                    _ => {
                        if let Some(g) = &context.geometry {
                            cad_contracts::geometry::verify(g, &context.document_id)?;
                        }
                        Ok(json!({"valid":true,"revision":context.revision}))
                    }
                }
            }
            _ => Err(AppError::new("UNKNOWN_TOOL", "Công cụ không được hỗ trợ.")),
        }
    }
}
fn invalid<E>(_error: E) -> AppError {
    AppError::new("INVALID_ARGUMENTS", "Arguments không đúng contract.")
}
fn definition(name: &str) -> Tool {
    let schema = match name {
        "cad.get_status" => json!({"type":"object","properties":{},"additionalProperties":false}),
        "cad.get_operation_status" => {
            json!({"type":"object","properties":{"operation_id":{"type":"string","maxLength":100}},"required":["operation_id"],"additionalProperties":false})
        }
        "cad.apply_change_set" | "cad.revert_change_set" => {
            json!({"type":"object","properties":{"change":{"type":"object"},"permit":{"type":"object"}},"required":["change","permit"],"additionalProperties":false})
        }
        "cad.bootstrap" | "cad.ensure_document" => {
            json!({"type":"object","description":"Private trusted-parent setup; not a model tool"})
        }
        _ => {
            json!({"type":"object","properties":{"project_id":{"type":"string","maxLength":100}},"required":["project_id"],"additionalProperties":false})
        }
    };
    Tool::new(
        name.to_owned(),
        "CAD mock domain tool",
        schema.as_object().expect("schema object").clone(),
    )
}
impl ServerHandler for CadServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: TOOLS.iter().map(|n| definition(n)).collect(),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let result = self
            .dispatch(
                &request.name,
                Value::Object(request.arguments.unwrap_or_default()),
            )
            .await;
        Ok(match result {
            Ok(value) => CallToolResult::structured(value),
            Err(error) => {
                CallToolResult::structured_error(serde_json::to_value(error).expect("error DTO"))
            }
        }
        .into())
    }
}
