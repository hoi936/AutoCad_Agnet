use super::*;
use std::sync::atomic::{AtomicI64, Ordering};
struct TestClock(AtomicI64);
impl Clock for TestClock {
    fn now(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}
#[tokio::test]
async fn approval_resize_revert_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let app = Studio::open(&path).await.unwrap();
    let p = app
        .projects_create("Tấm đế thử nghiệm".into())
        .await
        .unwrap();
    let first = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    assert!(app.context(&p.id).await.unwrap().geometry.is_none());
    assert_eq!(
        app.approve(&first.id, "tampered").await.unwrap_err().code,
        "HASH_MISMATCH"
    );
    let receipt = app.approve(&first.id, &first.payload_hash).await.unwrap();
    assert_eq!(
        app.approve(&first.id, &first.payload_hash)
            .await
            .unwrap()
            .operation_id,
        receipt.operation_id
    );
    assert_eq!(app.context(&p.id).await.unwrap().revision, 1);
    let resized = app
        .propose(
            &p.id,
            PlateParameters {
                width: 240.,
                ..PlateParameters::default()
            },
            RunMode::Execute,
        )
        .await
        .unwrap();
    app.approve(&resized.id, &resized.payload_hash)
        .await
        .unwrap();
    let inverse = app.revert(&resized.id).await.unwrap();
    assert_eq!(
        app.context(&p.id)
            .await
            .unwrap()
            .geometry
            .unwrap()
            .parameters
            .width,
        240.
    );
    app.approve(&inverse.id, &inverse.payload_hash)
        .await
        .unwrap();
    assert_eq!(
        app.context(&p.id)
            .await
            .unwrap()
            .geometry
            .unwrap()
            .parameters
            .width,
        200.
    );
    app.pool.close().await;
    let reopened = Studio::open(&path).await.unwrap();
    assert_eq!(reopened.projects_list().await.unwrap().len(), 1);
    assert_eq!(reopened.context(&p.id).await.unwrap().revision, 3);
    println!(
        "SQLite runtime {}",
        reopened.sqlite_version().await.unwrap()
    );
}
#[tokio::test]
async fn policy_reject_conflict_expiry_session() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let clock = Arc::new(TestClock(AtomicI64::new(1000)));
    let app = Studio::open_with_clock(&path, clock.clone()).await.unwrap();
    let p = app.projects_create("Policy".into()).await.unwrap();
    assert!(app
        .propose(&p.id, PlateParameters::default(), RunMode::Ask)
        .await
        .is_err());
    let plan = app
        .propose(&p.id, PlateParameters::default(), RunMode::Plan)
        .await
        .unwrap();
    assert!(app.approve(&plan.id, &plan.payload_hash).await.is_err());
    let c = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    app.reject(&c.id).await.unwrap();
    assert!(app.approve(&c.id, &c.payload_hash).await.is_err());
    assert!(app.context(&p.id).await.unwrap().geometry.is_none());
    let stale = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    let valid = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    app.approve(&valid.id, &valid.payload_hash).await.unwrap();
    assert_eq!(
        app.approve(&stale.id, &stale.payload_hash)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    let expired = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    clock.0.store(1600, Ordering::SeqCst);
    assert_eq!(
        app.approve(&expired.id, &expired.payload_hash)
            .await
            .unwrap_err()
            .code,
        "EXPIRED"
    );
    let old = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    app.pool.close().await;
    let next = Studio::open(&path).await.unwrap();
    assert_eq!(
        next.approve(&old.id, &old.payload_hash)
            .await
            .unwrap_err()
            .code,
        "SESSION_CHANGED"
    );
}
#[tokio::test]
async fn concurrent_approval_and_foreign_keys() {
    let dir = tempfile::tempdir().unwrap();
    let app = Studio::open(&dir.path().join("db")).await.unwrap();
    let p = app.projects_create("Concurrent".into()).await.unwrap();
    let c = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        app.approve(&c.id, &c.payload_hash),
        app.approve(&c.id, &c.payload_hash)
    );
    assert_eq!(a.unwrap().operation_id, b.unwrap().operation_id);
    assert_eq!(app.context(&p.id).await.unwrap().revision, 1);
    assert!(sqlx::query("DELETE FROM projects WHERE id=?")
        .bind(&p.id)
        .execute(&app.pool)
        .await
        .is_err());
    assert!(app
        .propose(
            "other-project",
            PlateParameters::default(),
            RunMode::Execute
        )
        .await
        .is_err());
}
#[test]
fn strict_contract() {
    let mut value = serde_json::to_value(PlateParameters::default()).unwrap();
    value["execute_shell"] = true.into();
    assert!(serde_json::from_value::<PlateParameters>(value).is_err());
}

#[tokio::test]
async fn receipt_failure_rolls_back_geometry_and_approval() {
    let dir = tempfile::tempdir().unwrap();
    let app = Studio::open(&dir.path().join("db")).await.unwrap();
    let p = app.projects_create("Rollback".into()).await.unwrap();
    let c = app
        .propose(&p.id, PlateParameters::default(), RunMode::Execute)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_receipt BEFORE INSERT ON execution_receipts BEGIN SELECT RAISE(ABORT,'injected test failure'); END;").execute(&app.pool).await.unwrap();
    assert!(app.approve(&c.id, &c.payload_hash).await.is_err());
    assert!(app.context(&p.id).await.unwrap().geometry.is_none());
    assert_eq!(app.context(&p.id).await.unwrap().revision, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM approvals")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER fail_receipt")
        .execute(&app.pool)
        .await
        .unwrap();
    app.approve(&c.id, &c.payload_hash).await.unwrap();
    assert_eq!(app.context(&p.id).await.unwrap().revision, 1);
}
