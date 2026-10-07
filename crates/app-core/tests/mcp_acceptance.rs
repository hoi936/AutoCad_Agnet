use cad_app_core::{AgentService, Studio};
use cad_contracts::{ChangeSet, RunMode};
use cad_model_provider::MockProvider;
use std::{sync::Arc, time::Duration};
async fn proposal(service: &Arc<AgentService>, conversation: &str, prompt: &str) -> ChangeSet {
    let run = service
        .start_run(
            conversation,
            prompt.into(),
            RunMode::Execute,
            Arc::new(MockProvider::new()),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = service.snapshot(&run.id).await.unwrap();
            if snapshot.run.status != "running" {
                assert_eq!(snapshot.run.status, "awaiting_approval");
                return snapshot.change_sets[0].clone();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn chat_through_real_mcp_approval_resize_revert_restart() {
    let dir = tempfile::tempdir().unwrap();
    let binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(if cfg!(windows) {
            "cad-tool-host.exe"
        } else {
            "cad-tool-host"
        });
    assert!(
        binary.exists(),
        "Run cargo build -p cad-tool-host before core tests"
    );
    let db = dir.path().join("app.sqlite");
    let state = dir.path().join("mock.json");
    let studio = Arc::new(Studio::open_mcp(&db, &binary, &state).await.unwrap());
    let project = studio
        .projects_create("MCP acceptance".into())
        .await
        .unwrap();
    let conversation = studio
        .conversations_create(&project.id, "Chat".into())
        .await
        .unwrap();
    let service = Arc::new(AgentService::new(studio.clone()));
    let first = proposal(
        &service,
        &conversation.id,
        "Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm.",
    )
    .await;
    assert_eq!(studio.context(&project.id).await.unwrap().revision, 0);
    studio
        .approve(&first.id, &first.payload_hash)
        .await
        .unwrap();
    service
        .change_finished(&first.id, "verified")
        .await
        .unwrap();
    studio
        .approve(&first.id, &first.payload_hash)
        .await
        .unwrap();
    assert_eq!(studio.context(&project.id).await.unwrap().revision, 1);
    let resize = proposal(
        &service,
        &conversation.id,
        "Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép.",
    )
    .await;
    studio
        .approve(&resize.id, &resize.payload_hash)
        .await
        .unwrap();
    service
        .change_finished(&resize.id, "verified")
        .await
        .unwrap();
    assert_eq!(
        studio
            .context(&project.id)
            .await
            .unwrap()
            .geometry
            .unwrap()
            .parameters
            .width,
        240.
    );
    let inverse = studio.revert(&resize.id).await.unwrap();
    studio
        .approve(&inverse.id, &inverse.payload_hash)
        .await
        .unwrap();
    service.shutdown().await;
    studio.shutdown().await;
    drop(service);
    drop(studio);
    let reopened = Studio::open_mcp(&db, &binary, &state).await.unwrap();
    let context = reopened.context(&project.id).await.unwrap();
    assert_eq!(context.revision, 3);
    assert_eq!(context.geometry.unwrap().parameters.width, 200.);
    assert_eq!(
        reopened
            .conversation_snapshot(&conversation.id)
            .await
            .unwrap()
            .runs
            .len(),
        2
    );
    reopened.shutdown().await;
    std::fs::remove_file(&state).unwrap();
    assert!(
        matches!(Studio::open_mcp(&db,&binary,&state).await,Err(e) if e.code=="RECOVERY_REQUIRED")
    );
}

#[tokio::test]
async fn committed_cad_with_failed_local_receipt_recovers_without_replay() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("app.sqlite");
    let state = dir.path().join("mock.json");
    let binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(if cfg!(windows) {
            "cad-tool-host.exe"
        } else {
            "cad-tool-host"
        });
    let studio = Studio::open_mcp(&db, &binary, &state).await.unwrap();
    let project = studio.projects_create("Recovery".into()).await.unwrap();
    let change = studio
        .propose(&project.id, Default::default(), RunMode::Execute)
        .await
        .unwrap();
    let pool =
        sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&db))
            .await
            .unwrap();
    sqlx::query("CREATE TRIGGER reject_receipt BEFORE INSERT ON execution_receipts BEGIN SELECT RAISE(ABORT,'fixture'); END").execute(&pool).await.unwrap();
    assert_eq!(
        studio
            .approve(&change.id, &change.payload_hash)
            .await
            .unwrap_err()
            .code,
        "OUTCOME_UNKNOWN"
    );
    assert_eq!(studio.context(&project.id).await.unwrap().revision, 1);
    assert_eq!(
        studio
            .changes_list(&project.id)
            .await
            .unwrap()
            .into_iter()
            .find(|c| c.id == change.id)
            .unwrap()
            .status,
        "outcome_unknown"
    );
    assert_eq!(
        studio.reject(&change.id).await.unwrap_err().code,
        "RECOVERY_REQUIRED"
    );
    let next = studio
        .propose(
            &project.id,
            cad_contracts::PlateParameters {
                width: 240.,
                ..Default::default()
            },
            RunMode::Execute,
        )
        .await
        .unwrap();
    assert_eq!(
        studio
            .approve(&next.id, &next.payload_hash)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_REQUIRED"
    );
    studio.shutdown().await;
    sqlx::query("DROP TRIGGER reject_receipt")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let reopened = Studio::open_mcp(&db, &binary, &state).await.unwrap();
    assert_eq!(
        reopened
            .approve(&change.id, &change.payload_hash)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_REQUIRED"
    );
    let receipt = reopened.recover_change(&change.id).await.unwrap();
    assert_eq!(receipt.observed_revision, 1);
    assert_eq!(reopened.context(&project.id).await.unwrap().revision, 1);
    assert_eq!(
        reopened
            .changes_list(&project.id)
            .await
            .unwrap()
            .into_iter()
            .find(|c| c.id == change.id)
            .unwrap()
            .status,
        "verified"
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn first_mcp_open_imports_existing_in_process_geometry_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("legacy.sqlite");
    let state = dir.path().join("mock.json");
    let old = Studio::open(&db).await.unwrap();
    let project = old.projects_create("Legacy geometry".into()).await.unwrap();
    let change = old
        .propose(&project.id, Default::default(), RunMode::Execute)
        .await
        .unwrap();
    old.approve(&change.id, &change.payload_hash).await.unwrap();
    let expected = old.context(&project.id).await.unwrap().geometry;
    old.shutdown().await;
    let binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(if cfg!(windows) {
            "cad-tool-host.exe"
        } else {
            "cad-tool-host"
        });
    let migrated = Studio::open_mcp(&db, &binary, &state).await.unwrap();
    let actual = migrated.context(&project.id).await.unwrap();
    assert_eq!(actual.revision, 1);
    assert_eq!(actual.geometry, expected);
    assert_eq!(
        migrated.changes_list(&project.id).await.unwrap()[0].id,
        change.id
    );
    migrated.shutdown().await;
}
