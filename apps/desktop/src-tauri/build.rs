fn main() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Build clock")
        .as_secs();
    println!("cargo:rustc-env=CAD_BUILD_ID=r1b-{}", stamp);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "account_list",
            "account_add_key",
            "account_sign_in",
            "account_cancel_login",
            "account_sign_out",
            "model_catalog",
            "account_select_model",
            "standalone_conversations",
            "standalone_create",
            "standalone_messages",
            "standalone_start",
            "standalone_snapshot",
            "standalone_cancel",
            "health",
            "runtime_status",
            "demo_enter",
            "projects_list",
            "projects_create",
            "cad_get_context",
            "design_propose_plate",
            "changes_list",
            "changes_approve",
            "changes_reject",
            "changes_revert",
            "changes_recover",
            "conversations_create",
            "conversations_list",
            "conversation_snapshot",
            "agent_start",
            "agent_snapshot",
            "agent_stop",
            "provider_get_config",
            "provider_save_config",
            "provider_set_secret",
            "provider_delete_secret",
        ]),
    ))
    .expect("Tauri configuration failed");
}
