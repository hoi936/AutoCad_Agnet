use cad_app_core::Studio;
use cad_contracts::{PlateParameters, RunMode};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let app = Studio::open(&dir.path().join("demo.sqlite")).await?;
    let project = app.projects_create("Demo tấm đế".into()).await?;
    let create = app
        .propose(&project.id, PlateParameters::default(), RunMode::Execute)
        .await?;
    assert!(app.context(&project.id).await?.geometry.is_none());
    println!("MÔ PHỎNG — KHÔNG PHẢI AUTOCAD\nPreview: 200 × 120, 4 lỗ Ø10; chưa ghi.");
    // Explicit test harness approval, never a model/provider action.
    app.approve(&create.id, &create.payload_hash).await?;
    let resize = app
        .propose(
            &project.id,
            PlateParameters {
                width: 240.,
                ..PlateParameters::default()
            },
            RunMode::Execute,
        )
        .await?;
    app.approve(&resize.id, &resize.payload_hash).await?;
    println!(
        "Đã duyệt resize 240; revision {}",
        app.context(&project.id).await?.revision
    );
    let inverse = app.revert(&resize.id).await?;
    app.approve(&inverse.id, &inverse.payload_hash).await?;
    println!(
        "Đã duyệt hoàn tác: {} mm; SQLite {}. Không tạo DWG.",
        app.context(&project.id)
            .await?
            .geometry
            .unwrap()
            .parameters
            .width,
        app.sqlite_version().await?
    );
    Ok(())
}
