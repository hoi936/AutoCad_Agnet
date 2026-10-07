use cad_contracts::*;
use cad_tool_host::{
    permit::{payload_hash, PermitAuthority},
    state::{Bootstrap, MockCadGateway, MutationRequest},
    CadGateway, McpGateway,
};
use std::path::{Path, PathBuf};
fn project() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn seed(p: &str, session: &str) -> DocumentContext {
    DocumentContext {
        project_id: p.into(),
        document_id: format!("mock:{p}"),
        session_id: session.into(),
        revision: 0,
        geometry: None,
        mode: "mock".into(),
        units: Units::Millimeters,
    }
}
fn change(c: DocumentContext, width: f64) -> ChangeSet {
    let parameters = PlateParameters {
        width,
        ..c.geometry
            .as_ref()
            .map(|g| g.parameters.clone())
            .unwrap_or_default()
    };
    let payload = ChangePayload {
        schema_version: 1,
        project_id: c.project_id,
        document_id: c.document_id.clone(),
        session_id: c.session_id,
        base_revision: c.revision,
        mode: RunMode::Execute,
        before: c.geometry,
        after: Some(geometry::plan(parameters, &c.document_id).unwrap()),
        created_at: cad_tool_host::now(),
        expires_at: cad_tool_host::now() + 600,
        reverts: None,
    };
    ChangeSet {
        id: project(),
        payload_hash: payload_hash(&payload).unwrap(),
        status: "pending_approval".into(),
        payload,
    }
}
fn installed_binary(dir: &Path) -> PathBuf {
    let destination = dir.join("installed app with spaces");
    std::fs::create_dir_all(&destination).unwrap();
    let binary = destination.join(if cfg!(windows) {
        "cad-tool-host.exe"
    } else {
        "cad-tool-host"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_cad-tool-host"), &binary).unwrap();
    binary
}
#[tokio::test]
async fn real_stdio_negotiate_discover_apply_readback_restart_and_resize() {
    let dir = tempfile::tempdir().unwrap();
    let binary = installed_binary(dir.path());
    let snapshot = dir.path().join("mock.json");
    let p = project();
    let gateway = McpGateway::spawn(&binary, &snapshot, vec![seed(&p, "s1")], "s1".into(), true)
        .await
        .unwrap();
    assert!(gateway
        .discover()
        .await
        .unwrap()
        .contains(&"cad.apply_change_set".into()));
    let c = change(gateway.context(&p).await.unwrap(), 200.);
    assert!(gateway.context(&p).await.unwrap().geometry.is_none());
    let receipt = gateway.apply(&c).await.unwrap();
    assert_eq!(
        gateway.apply(&c).await.unwrap().operation_id,
        receipt.operation_id
    );
    let actual = gateway.context(&p).await.unwrap();
    assert_eq!(actual.revision, 1);
    assert_eq!(actual.geometry.as_ref().unwrap().entities.len(), 8);
    assert!(gateway.read("shell.run", &p).await.is_err());
    let stale = change(seed(&p, "s1"), 200.);
    assert_eq!(gateway.apply(&stale).await.unwrap_err().code, "CONFLICT");
    let resized = change(actual, 240.);
    gateway.apply(&resized).await.unwrap();
    let g = gateway.context(&p).await.unwrap().geometry.unwrap();
    match &g.entities[2] {
        Entity::Circle { center, radius, .. } => {
            assert_eq!(*center, [220., 20.]);
            assert_eq!(*radius, 5.);
        }
        _ => panic!("circle"),
    }
    gateway.shutdown().await;
    let restarted = McpGateway::spawn(&binary, &snapshot, vec![seed(&p, "s2")], "s2".into(), false)
        .await
        .unwrap();
    assert_eq!(restarted.context(&p).await.unwrap().revision, 2);
    assert!(restarted.operation(&c.id).await.unwrap().is_some());
    assert_eq!(
        restarted.apply(&resized).await.unwrap_err().code,
        "PERMIT_DENIED"
    );
    restarted.shutdown().await;
}
#[test]
fn permit_tamper_expiry_document_session_and_mode_denied() {
    let dir = tempfile::tempdir().unwrap();
    let authority = PermitAuthority::random();
    let p = project();
    let c = change(seed(&p, "s"), 200.);
    let mut gateway = MockCadGateway::bootstrap(Bootstrap {
        session_id: "s".into(),
        key: authority.bootstrap_key(),
        snapshot_path: dir.path().join("snapshot.json"),
        seeds: vec![seed(&p, "s")],
        allow_initial_import: true,
    })
    .unwrap();
    let valid = authority
        .issue(&c.id, &c.payload, cad_tool_host::now() + 60)
        .unwrap();
    for kind in 0..4 {
        let mut permit = valid.clone();
        match kind {
            0 => permit.mac[0] ^= 1,
            1 => permit.expires_at = 0,
            2 => permit.document_id = "other".into(),
            _ => permit.session_id = "old".into(),
        };
        assert_eq!(
            gateway
                .apply(MutationRequest {
                    change: c.clone(),
                    permit
                })
                .unwrap_err()
                .code,
            "PERMIT_DENIED"
        );
    }
    let mut plan = c.clone();
    plan.payload.mode = RunMode::Plan;
    plan.payload_hash = payload_hash(&plan.payload).unwrap();
    let permit = authority
        .issue(&plan.id, &plan.payload, cad_tool_host::now() + 60)
        .unwrap();
    assert_eq!(
        gateway
            .apply(MutationRequest {
                change: plan,
                permit
            })
            .unwrap_err()
            .code,
        "APPROVAL_DENIED"
    );
    assert!(gateway.context(&p).unwrap().geometry.is_none());
}
#[cfg(feature = "fault-injection")]
#[tokio::test]
async fn commit_then_disconnect_keeps_receipt_and_never_replays() {
    let dir = tempfile::tempdir().unwrap();
    let binary = installed_binary(dir.path());
    let snapshot = dir.path().join("snapshot.json");
    let p = project();
    let gateway = McpGateway::spawn(&binary, &snapshot, vec![seed(&p, "a")], "a".into(), true)
        .await
        .unwrap();
    let c = change(gateway.context(&p).await.unwrap(), 200.);
    gateway.inject_disconnect_after_commit().await.unwrap();
    assert_eq!(
        gateway.apply(&c).await.unwrap_err().code,
        "CAD_DISCONNECTED"
    );
    gateway.shutdown().await;
    let restarted = McpGateway::spawn(&binary, &snapshot, vec![seed(&p, "b")], "b".into(), false)
        .await
        .unwrap();
    assert_eq!(
        restarted
            .operation(&c.id)
            .await
            .unwrap()
            .unwrap()
            .observed_revision,
        1
    );
    assert_eq!(restarted.context(&p).await.unwrap().revision, 1);
    restarted.shutdown().await;
}
#[tokio::test]
async fn oversized_frame_is_bounded() {
    use tokio::io::AsyncReadExt;
    let data = vec![b'x'; 1024 * 1024 + 1];
    let mut reader = cad_tool_host::bounded::BoundedRead::new(std::io::Cursor::new(data));
    let mut out = Vec::new();
    assert!(reader.read_to_end(&mut out).await.is_err());
    assert!(out.len() <= 1024 * 1024);
}
