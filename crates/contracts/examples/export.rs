use cad_contracts::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("apps/desktop/src/generated.ts"),
        typescript_contracts(),
    )?;
    let schema = schemars::schema_for!(PlateParameters);
    std::fs::write(
        root.join("contracts/schemas/plate-parameters.schema.json"),
        serde_json::to_string_pretty(&schema)?,
    )?;
    std::fs::write(
        root.join("contracts/fixtures/plate-default.json"),
        serde_json::to_string_pretty(&PlateParameters::default())?,
    )?;
    Ok(())
}
