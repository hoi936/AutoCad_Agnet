#![forbid(unsafe_code)]
mod agent;
pub mod geometry;
pub use agent::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const SCHEMA_VERSION: u32 = 1;
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlateParameters {
    pub schema_version: u32,
    pub width: f64,
    pub height: f64,
    pub hole_diameter: f64,
    pub edge_offset: f64,
    pub origin: [f64; 2],
    pub units: Units,
    pub add_dimensions: bool,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Units {
    #[serde(rename = "mm")]
    Millimeters,
}
impl Default for PlateParameters {
    fn default() -> Self {
        Self {
            schema_version: 1,
            width: 200.,
            height: 120.,
            hole_diameter: 10.,
            edge_offset: 20.,
            origin: [0., 0.],
            units: Units::Millimeters,
            add_dimensions: true,
        }
    }
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entity {
    Polyline {
        id: String,
        layer: String,
        points: [[f64; 2]; 4],
        closed: bool,
    },
    Circle {
        id: String,
        layer: String,
        center: [f64; 2],
        radius: f64,
    },
    Dimension {
        id: String,
        layer: String,
        measurement: f64,
        role: String,
    },
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Geometry {
    pub parameters: PlateParameters,
    pub entities: Vec<Entity>,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Ask,
    Plan,
    Execute,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct DocumentContext {
    pub document_id: String,
    pub project_id: String,
    pub session_id: String,
    #[ts(type = "number")]
    pub revision: i64,
    pub geometry: Option<Geometry>,
    pub mode: String,
    pub units: Units,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ChangePayload {
    pub schema_version: u32,
    pub project_id: String,
    pub document_id: String,
    pub session_id: String,
    #[ts(type = "number")]
    pub base_revision: i64,
    pub mode: RunMode,
    pub before: Option<Geometry>,
    pub after: Option<Geometry>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub expires_at: i64,
    pub reverts: Option<String>,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSet {
    pub id: String,
    pub payload_hash: String,
    pub status: String,
    pub payload: ChangePayload,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub operation_id: String,
    pub change_set_id: String,
    pub payload_hash: String,
    #[ts(type = "number")]
    pub observed_revision: i64,
    pub status: String,
    pub saved_to_dwg: bool,
}
#[derive(TS, JsonSchema, Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub correlation_id: String,
    pub safe_details: Option<String>,
}
impl AppError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            correlation_id: String::new(),
            safe_details: None,
        }
    }
}

/// Contract generation used by the export tool and drift test.
pub fn typescript_contracts() -> String {
    let cfg = ts_rs::Config::default();
    let mut types = vec![
        PlateParameters::decl(&cfg),
        Units::decl(&cfg),
        Entity::decl(&cfg),
        Geometry::decl(&cfg),
        Project::decl(&cfg),
        RunMode::decl(&cfg),
        DocumentContext::decl(&cfg),
        ChangePayload::decl(&cfg),
        ChangeSet::decl(&cfg),
        ExecutionReceipt::decl(&cfg),
        AppError::decl(&cfg),
    ];
    types.extend(agent_typescript_declarations(&cfg));
    let body = types
        .into_iter()
        .map(|s| format!("export {s}\n"))
        .collect::<String>();
    format!("// Generated from cad-contracts. Do not edit.\n{body}")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frontend_types_match_rust() {
        assert_eq!(
            typescript_contracts(),
            include_str!("../../../apps/desktop/src/generated.ts")
        );
    }
    #[test]
    fn schema_and_fixture_match_rust() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/schemas/plate-parameters.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema,
            serde_json::to_value(schemars::schema_for!(PlateParameters)).unwrap()
        );
        let fixture: PlateParameters = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/plate-default.json"
        ))
        .unwrap();
        assert_eq!(fixture, PlateParameters::default());
    }
}
