use super::*;
use async_trait::async_trait;
use cad_model_provider::{MockProvider, ProviderError, ProviderToolCall};
use std::sync::atomic::{AtomicUsize, Ordering};

const CREATE: &str = "Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm.";
const RESIZE: &str = "Tăng chiều dài lên 240 mm, giữ khoảng cách lỗ tới mép.";

async fn setup() -> (
    tempfile::TempDir,
    Arc<Studio>,
    Arc<AgentService>,
    Project,
    Conversation,
) {
    let dir = tempfile::tempdir().unwrap();
    let studio = Arc::new(Studio::open(&dir.path().join("app.sqlite")).await.unwrap());
    let project = studio
        .projects_create("Hội thoại kiểm thử".into())
        .await
        .unwrap();
    let conversation = studio
        .conversations_create(&project.id, "Chi tiết tấm đế".into())
        .await
        .unwrap();
    let service = Arc::new(AgentService::new(studio.clone()));
    (dir, studio, service, project, conversation)
}

async fn finished(service: &AgentService, run: &AgentRun) -> RunSnapshot {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let snapshot = service.snapshot(&run.id).await.unwrap();
            if snapshot.run.status != "running" {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("run must reach a terminal state")
}

async fn run_fixture(
    service: &Arc<AgentService>,
    conversation: &Conversation,
    prompt: &str,
    mode: RunMode,
) -> RunSnapshot {
    let run = service
        .start_run(
            &conversation.id,
            prompt.into(),
            mode,
            Arc::new(MockProvider::new()),
        )
        .await
        .unwrap();
    finished(service, &run).await
}

#[tokio::test]
async fn chat_create_reject_approve_resize_revert_and_restart() {
    let (dir, studio, service, project, conversation) = setup().await;
    let first = run_fixture(&service, &conversation, CREATE, RunMode::Execute).await;
    assert_eq!(first.run.status, "awaiting_approval");
    assert_eq!(first.change_sets.len(), 1);
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
    studio.reject(&first.change_sets[0].id).await.unwrap();
    service
        .change_finished(&first.change_sets[0].id, "rejected")
        .await
        .unwrap();
    assert_eq!(
        service.snapshot(&first.run.id).await.unwrap().run.status,
        "rejected"
    );
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());

    let create = run_fixture(&service, &conversation, CREATE, RunMode::Execute).await;
    let change = &create.change_sets[0];
    let receipt = studio
        .approve(&change.id, &change.payload_hash)
        .await
        .unwrap();
    assert_eq!(
        studio
            .approve(&change.id, &change.payload_hash)
            .await
            .unwrap()
            .operation_id,
        receipt.operation_id
    );
    service
        .change_finished(&change.id, "verified")
        .await
        .unwrap();
    service
        .change_finished(&change.id, "verified")
        .await
        .unwrap();
    let context = studio.context(&project.id).await.unwrap();
    assert_eq!(context.revision, 1);
    let circles: Vec<_> = context
        .geometry
        .unwrap()
        .entities
        .into_iter()
        .filter_map(|entity| match entity {
            Entity::Circle { center, radius, .. } => Some((center, radius)),
            _ => None,
        })
        .collect();
    assert_eq!(
        circles,
        vec![
            ([20., 20.], 5.),
            ([180., 20.], 5.),
            ([180., 100.], 5.),
            ([20., 100.], 5.)
        ]
    );

    let resize = run_fixture(&service, &conversation, RESIZE, RunMode::Execute).await;
    let change = &resize.change_sets[0];
    assert_eq!(
        studio
            .context(&project.id)
            .await
            .unwrap()
            .geometry
            .unwrap()
            .parameters
            .width,
        200.
    );
    studio
        .approve(&change.id, &change.payload_hash)
        .await
        .unwrap();
    service
        .change_finished(&change.id, "verified")
        .await
        .unwrap();
    let geometry = studio.context(&project.id).await.unwrap().geometry.unwrap();
    assert_eq!(geometry.parameters.width, 240.);
    assert_eq!(geometry.parameters.height, 120.);
    assert_eq!(geometry.parameters.edge_offset, 20.);
    assert_eq!(geometry.parameters.hole_diameter, 10.);
    assert!(geometry
        .entities
        .iter()
        .any(|e| matches!(e,Entity::Circle{center,..} if *center == [220.,100.])));
    let revert = studio.revert(&change.id).await.unwrap();
    studio
        .approve(&revert.id, &revert.payload_hash)
        .await
        .unwrap();
    let before = studio
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap();
    assert!(before
        .messages
        .iter()
        .any(|m| m.role == "system" && m.content.contains("xác minh")));
    assert!(before
        .messages
        .iter()
        .any(|m| m.role == "assistant" && m.content.contains("preview")));
    assert!(before.runs.iter().all(|r| r.status != "running"));
    studio.pool.close().await;
    let reopened = Studio::open(&dir.path().join("app.sqlite")).await.unwrap();
    let after = reopened
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap();
    assert_eq!(after.messages.len(), before.messages.len());
    assert_eq!(
        reopened
            .context(&project.id)
            .await
            .unwrap()
            .geometry
            .unwrap()
            .parameters
            .width,
        200.
    );
}

struct ForcedTool {
    name: &'static str,
    arguments: String,
    calls: AtomicUsize,
}
#[async_trait]
impl ModelProvider for ForcedTool {
    async fn respond(
        &self,
        request: ModelRequest,
        _: mpsc::Sender<ModelDelta>,
        _: CancellationToken,
    ) -> std::result::Result<ModelResponse, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if request.tools.is_empty() {
            return Ok(ModelResponse {
                text: "Đã nhận output.".into(),
                ..Default::default()
            });
        }
        Ok(ModelResponse {
            tool_calls: vec![ProviderToolCall {
                call_id: "forced-call".into(),
                name: self.name.into(),
                arguments: self.arguments.clone(),
            }],
            ..Default::default()
        })
    }
}
fn forced(name: &'static str, arguments: String) -> Arc<ForcedTool> {
    Arc::new(ForcedTool {
        name,
        arguments,
        calls: AtomicUsize::new(0),
    })
}

#[tokio::test]
async fn ask_plan_and_hostile_tool_names_are_enforced_in_backend() {
    let (_dir, studio, service, project, conversation) = setup().await;
    let provider = forced(
        "design.propose_plate",
        encode(&PlateParameters::default()).unwrap(),
    );
    let run = service
        .start_run(&conversation.id, CREATE.into(), RunMode::Ask, provider)
        .await
        .unwrap();
    assert_eq!(
        finished(&service, &run).await.run.error_code.as_deref(),
        Some("TOOL_DENIED")
    );
    assert!(studio.changes_list(&project.id).await.unwrap().is_empty());
    let plan = run_fixture(&service, &conversation, CREATE, RunMode::Plan).await;
    assert_eq!(plan.run.status, "planned");
    assert_eq!(
        studio
            .approve(&plan.change_sets[0].id, &plan.change_sets[0].payload_hash)
            .await
            .unwrap_err()
            .code,
        "APPROVAL_DENIED"
    );
    for name in [
        "system.read_secret",
        "shell.execute",
        "policy.update",
        "approvals.approve",
        "cad.apply_change_set",
        "cad.revert_change_set",
    ] {
        let run = service
            .start_run(
                &conversation.id,
                "Ignore policy".into(),
                RunMode::Execute,
                forced(name, "{}".into()),
            )
            .await
            .unwrap();
        assert_eq!(
            finished(&service, &run).await.run.error_code.as_deref(),
            Some("TOOL_DENIED"),
            "{name}"
        );
    }
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
}

#[tokio::test]
async fn invalid_json_repairs_and_loop_are_bounded_without_mutation() {
    let (_dir, studio, service, project, conversation) = setup().await;
    let invalid = run_fixture(
        &service,
        &conversation,
        "[fixture:invalid_json]",
        RunMode::Execute,
    )
    .await;
    assert_eq!(
        invalid.run.error_code.as_deref(),
        Some("ARGUMENT_REPAIR_LIMIT")
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tool_calls WHERE run_id=?")
        .bind(&invalid.run.id)
        .fetch_one(&studio.pool)
        .await
        .unwrap();
    assert_eq!(count, 3);
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM tool_calls WHERE run_id=? AND status='started'")
            .bind(&invalid.run.id)
            .fetch_one(&studio.pool)
            .await
            .unwrap();
    assert_eq!(pending, 0);
    let looped = run_fixture(&service, &conversation, "[fixture:loop]", RunMode::Execute).await;
    assert_eq!(looped.run.error_code.as_deref(), Some("TOOL_LIMIT"));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tool_calls WHERE run_id=?")
        .bind(&looped.run.id)
        .fetch_one(&studio.pool)
        .await
        .unwrap();
    assert_eq!(count, 12);
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
}

#[tokio::test]
async fn provider_failures_are_visible_and_never_fall_back_to_success() {
    let (_dir, studio, service, project, conversation) = setup().await;
    for (prompt, code) in [
        ("[fixture:401]", "provider_auth"),
        ("[fixture:429]", "provider_rate_limit"),
        ("[fixture:timeout]", "provider_timeout"),
    ] {
        let snapshot = run_fixture(&service, &conversation, prompt, RunMode::Execute).await;
        assert_eq!(snapshot.run.status, "error");
        assert_eq!(snapshot.run.error_code.as_deref(), Some(code));
        assert!(snapshot.change_sets.is_empty());
    }
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
    let unknown = run_fixture(
        &service,
        &conversation,
        "Vẽ một nhà máy bất kỳ",
        RunMode::Execute,
    )
    .await;
    assert_eq!(unknown.run.status, "completed");
    assert!(unknown.change_sets.is_empty());
    assert!(studio
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap()
        .messages
        .last()
        .unwrap()
        .content
        .contains("không phải LLM tổng quát"));
}

#[tokio::test]
async fn cancellation_and_one_active_run_are_persisted() {
    let (_dir, studio, service, project, conversation) = setup().await;
    let run = service
        .start_run(
            &conversation.id,
            "[fixture:cancel]".into(),
            RunMode::Execute,
            Arc::new(MockProvider::new()),
        )
        .await
        .unwrap();
    assert_eq!(
        service
            .start_run(
                &conversation.id,
                CREATE.into(),
                RunMode::Execute,
                Arc::new(MockProvider::new())
            )
            .await
            .unwrap_err()
            .code,
        "RUN_ACTIVE"
    );
    service.cancel(&run.id).await.unwrap();
    let snapshot = finished(&service, &run).await;
    assert_eq!(snapshot.run.status, "cancelled");
    assert!(snapshot.events.iter().any(|e| e.kind == "run.cancelled"));
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
    assert!(
        run_fixture(&service, &conversation, CREATE, RunMode::Execute)
            .await
            .change_sets
            .len()
            == 1
    );
}

struct EchoSecret;
#[async_trait]
impl ModelProvider for EchoSecret {
    async fn respond(
        &self,
        _: ModelRequest,
        events: mpsc::Sender<ModelDelta>,
        _: CancellationToken,
    ) -> std::result::Result<ModelResponse, ProviderError> {
        events
            .send(ModelDelta::Text("API sk-test".into()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        events
            .send(ModelDelta::Text("-privatekey text".into()))
            .await
            .unwrap();
        Ok(ModelResponse {
            text: "API sk-test-privatekey text".into(),
            ..Default::default()
        })
    }
}
#[tokio::test]
async fn secrets_are_redacted_from_prompt_messages_stream_and_database() {
    let (dir, studio, service, project, conversation) = setup().await;
    assert_eq!(
        studio
            .provider_save_config(&project.id, "openai", "sk-test-privatekey", true, "session")
            .await
            .unwrap_err()
            .code,
        "INVALID_MODEL"
    );
    let run = service
        .start_run(
            &conversation.id,
            "User sk-test-privatekey message".into(),
            RunMode::Ask,
            Arc::new(EchoSecret),
        )
        .await
        .unwrap();
    loop {
        let snapshot = service.snapshot(&run.id).await.unwrap();
        let serialized = encode(&snapshot).unwrap();
        assert!(!serialized.contains("sk-test"));
        if snapshot.run.status != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let messages = studio
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap();
    assert!(!encode(&messages).unwrap().contains("sk-test"));
    let token_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM run_events WHERE kind='text.delta'")
            .fetch_one(&studio.pool)
            .await
            .unwrap();
    assert_eq!(token_rows, 0, "token deltas must not become SQLite rows");
    studio.pool.close().await;
    let bytes = std::fs::read(dir.path().join("app.sqlite")).unwrap();
    assert!(!bytes
        .windows(b"sk-test-privatekey".len())
        .any(|w| w == b"sk-test-privatekey"));
}

#[tokio::test]
async fn consent_prevents_provider_call_and_config_never_contains_secret() {
    let (_dir, studio, service, project, conversation) = setup().await;
    let config = studio
        .provider_save_config(&project.id, "openai", "test-model", false, "session")
        .await
        .unwrap();
    assert!(config.secret_ref.unwrap().starts_with("cad-agent-studio:"));
    assert!(!config.configured);
    let provider = forced(
        "design.propose_plate",
        encode(&PlateParameters::default()).unwrap(),
    );
    assert_eq!(
        service
            .start_run(
                &conversation.id,
                CREATE.into(),
                RunMode::Execute,
                provider.clone()
            )
            .await
            .unwrap_err()
            .code,
        "CLOUD_CONSENT_REQUIRED"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    assert!(studio
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap()
        .messages
        .is_empty());
}

#[tokio::test]
async fn stale_running_state_is_interrupted_on_reopen_without_replay() {
    let (dir, studio, service, project, conversation) = setup().await;
    let run = run_fixture(
        &service,
        &conversation,
        "Một câu ngoài fixture",
        RunMode::Ask,
    )
    .await;
    sqlx::query("UPDATE agent_runs SET status='running',finished_at=NULL WHERE id=?")
        .bind(&run.run.id)
        .execute(&studio.pool)
        .await
        .unwrap();
    studio.pool.close().await;
    let reopened = Studio::open(&dir.path().join("app.sqlite")).await.unwrap();
    reopened.agent_recover_interrupted().await.unwrap();
    let snapshot = reopened
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap();
    assert_eq!(snapshot.runs[0].status, "interrupted");
    assert!(snapshot
        .messages
        .last()
        .unwrap()
        .content
        .contains("Không tự chạy lại"));
    assert!(reopened
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
}

#[tokio::test]
async fn version_one_migration_preserves_project_document_geometry_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let old_migrations = tempfile::tempdir().unwrap();
    std::fs::write(
        old_migrations.path().join("0001_mock_foundation.sql"),
        include_str!("../../../database/migrations/0001_mock_foundation.sql"),
    )
    .unwrap();
    let path = dir.path().join("upgrade.sqlite");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(old_migrations.path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let geometry = geometry::plan(PlateParameters::default(), "legacy-document").unwrap();
    sqlx::query("INSERT INTO projects VALUES('legacy-project','Dự án cũ',100)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO documents VALUES('legacy-document','legacy-project','mock',7,?)")
        .bind(encode(&geometry).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO change_sets VALUES('legacy-change','legacy-document','{}','legacy-hash','verified',100)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO approvals VALUES('legacy-approval','legacy-change','legacy-hash','old-session',100,'approved')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO execution_receipts VALUES('legacy-operation','legacy-change','{}')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let studio = Studio::open(&path).await.unwrap();
    assert_eq!(studio.projects_list().await.unwrap().len(), 1);
    let context = studio.context("legacy-project").await.unwrap();
    assert_eq!(context.revision, 7);
    assert_eq!(context.geometry.unwrap(), geometry);
    for query in [
        "SELECT count(*) FROM change_sets",
        "SELECT count(*) FROM approvals",
        "SELECT count(*) FROM execution_receipts",
    ] {
        let count: i64 = sqlx::query_scalar(query)
            .fetch_one(&studio.pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
    let conversation = studio
        .conversations_create("legacy-project", "Hội thoại sau nâng cấp".into())
        .await
        .unwrap();
    assert!(studio
        .conversation_snapshot(&conversation.id)
        .await
        .unwrap()
        .messages
        .is_empty());
    assert!(
        sqlx::query("INSERT INTO conversations VALUES('orphan','missing','Invalid',100)")
            .execute(&studio.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn new_unsupported_request_invalidates_old_form_and_chat_approvals() {
    let (_dir, studio, service, project, conversation) = setup().await;
    let mut parameters = PlateParameters::default();
    parameters.width = 204.0;
    let form = studio
        .propose(&project.id, parameters, RunMode::Execute)
        .await
        .unwrap();
    let unsupported = run_fixture(
        &service,
        &conversation,
        "Vẽ hình 260 x 160 mm",
        RunMode::Execute,
    )
    .await;
    assert!(unsupported.change_sets.is_empty());
    assert_eq!(studio.change(&form.id).await.unwrap().status, "superseded");
    assert!(studio.approve(&form.id, &form.payload_hash).await.is_err());
    let old = run_fixture(&service, &conversation, CREATE, RunMode::Execute)
        .await
        .change_sets
        .remove(0);
    let _ = run_fixture(&service, &conversation, "Vẽ nhà ba tầng", RunMode::Execute).await;
    assert!(studio.approve(&old.id, &old.payload_hash).await.is_err());
    assert!(studio
        .context(&project.id)
        .await
        .unwrap()
        .geometry
        .is_none());
}

#[tokio::test]
async fn new_proposal_and_provider_change_revoke_old_approvals() {
    let (_dir, studio, _service, project, _conversation) = setup().await;
    let first = studio
        .propose(&project.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    let next = studio
        .propose(&project.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    assert_eq!(studio.change(&first.id).await.unwrap().status, "superseded");
    assert!(studio
        .approve(&first.id, &first.payload_hash)
        .await
        .is_err());
    assert_eq!(
        studio.change(&next.id).await.unwrap().status,
        "pending_approval"
    );
    studio
        .provider_save_config(&project.id, "openai", "test-model", true, "session")
        .await
        .unwrap();
    assert_eq!(studio.change(&next.id).await.unwrap().status, "superseded");
    assert!(studio.approve(&next.id, &next.payload_hash).await.is_err());
}

#[tokio::test]
async fn version_three_pending_upgrade_preserves_unknown_outcomes() {
    let dir = tempfile::tempdir().unwrap();
    let migrations = tempfile::tempdir().unwrap();
    for (name, sql) in [
        (
            "0001_mock_foundation.sql",
            include_str!("../../../database/migrations/0001_mock_foundation.sql"),
        ),
        (
            "0002_agent.sql",
            include_str!("../../../database/migrations/0002_agent.sql"),
        ),
        (
            "0003_gateway.sql",
            include_str!("../../../database/migrations/0003_gateway.sql"),
        ),
    ] {
        std::fs::write(migrations.path().join(name), sql).unwrap();
    }
    let path = dir.path().join("upgrade.sqlite");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO projects VALUES('p','Legacy',100)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO documents VALUES('d','p','mock',0,NULL)")
        .execute(&pool)
        .await
        .unwrap();
    for id in ["pending", "unknown"] {
        sqlx::query("INSERT INTO change_sets VALUES(?,'d','{}','hash','pending_approval',100)")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO gateway_operations VALUES('unknown','op','hash','outcome_unknown',100)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let studio = Studio::open(&path).await.unwrap();
    let invalid: Vec<String> =
        sqlx::query_scalar("SELECT change_set_id FROM proposal_invalidations")
            .fetch_all(&studio.pool)
            .await
            .unwrap();
    assert_eq!(invalid, vec!["pending"]);
    let unknown: String =
        sqlx::query_scalar("SELECT state FROM gateway_operations WHERE change_set_id='unknown'")
            .fetch_one(&studio.pool)
            .await
            .unwrap();
    assert_eq!(unknown, "outcome_unknown");
    // Subsequent proposal supersession also leaves unresolved sends for recovery.
    studio
        .propose("p", PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM proposal_invalidations WHERE change_set_id='unknown'",
    )
    .fetch_one(&studio.pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}
