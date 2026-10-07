#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]
use cad_app_core::standalone::{Account, AppRuntime, ChatRun};
use cad_app_core::{AgentService, ProviderService, Studio};
use cad_contracts::*;
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};
use tauri::{Manager, State};
type Result<T> = std::result::Result<T, AppError>;
struct Services {
    studio: Arc<Studio>,
    agent: Arc<AgentService>,
    providers: ProviderService,
}
struct RuntimeState {
    live: Arc<AppRuntime>,
    services: OnceLock<Services>,
    initializing: tauri::async_runtime::Mutex<()>,
    data: PathBuf,
    binary: PathBuf,
}
impl RuntimeState {
    fn services(&self) -> Result<&Services> {
        self.services.get().ok_or_else(|| {
            AppError::new(
                "CONNECTION_REQUIRED",
                "Chưa kết nối CAD. Chọn Demo rõ ràng để dùng mô phỏng.",
            )
        })
    }
    fn studio(&self) -> Result<&Arc<Studio>> {
        Ok(&self.services()?.studio)
    }
    fn agent(&self) -> Result<&Arc<AgentService>> {
        Ok(&self.services()?.agent)
    }
    fn providers(&self) -> Result<&ProviderService> {
        Ok(&self.services()?.providers)
    }
}
type AppState<'a> = State<'a, RuntimeState>;
type AgentState<'a> = State<'a, RuntimeState>;
#[tauri::command]
async fn runtime_status(app: AppState<'_>) -> Result<serde_json::Value> {
    let cad = app.live.cad_status().await.unwrap_or_else(|_| serde_json::json!({"connected":false}));
    let connected = cad["connected"] == true;
    Ok(
        serde_json::json!({"build_id": env!("CAD_BUILD_ID"), "runtime":"desktop", "mode": if app.services.get().is_some() {"demo"} else if connected {"live_read_only"} else {"live_unconnected"}, "cad": if app.services.get().is_some() {"mock"} else if connected {"autocad_read_only"} else {"disconnected"}, "mcp": if app.services.get().is_some() || connected {"handshake"} else {"not_started"}, "oauth":"implemented_live_not_tested"}),
    )
}
#[tauri::command]
async fn cad_candidates() -> Result<Vec<u32>> {
    let output = std::process::Command::new("tasklist.exe")
        .args(["/FI", "IMAGENAME eq acad.exe", "/FO", "CSV", "/NH"])
        .output().map_err(|_| AppError::new("PROCESS_DISCOVERY", "Không dò được AutoCAD."))?;
    if !output.status.success() { return Err(AppError::new("PROCESS_DISCOVERY", "Không dò được AutoCAD.")); }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().filter_map(|line| {
        let fields: Vec<_> = line.trim().trim_start_matches('"').split("\",\"").collect();
        if fields.first().is_some_and(|name| name.eq_ignore_ascii_case("acad.exe")) {
            fields.get(1).and_then(|v| v.trim_matches('"').parse::<u32>().ok())
        } else { None }
    }).collect())
}
#[tauri::command]
async fn cad_pair(app: AppState<'_>, pid: u32, code: String) -> Result<serde_json::Value> {
    app.live.cad_pair(&app.binary, pid, &code).await
}
#[tauri::command]
async fn cad_live_status(app: AppState<'_>) -> Result<serde_json::Value> { app.live.cad_status().await }
#[tauri::command]
async fn cad_disconnect(app: AppState<'_>) -> Result<()> { app.live.cad_disconnect().await; Ok(()) }
#[tauri::command]
async fn demo_enter(app: AppState<'_>, handle: tauri::AppHandle) -> Result<()> {
    let _guard = app.initializing.lock().await;
    if app.services.get().is_none() {
        let studio = Arc::new(
            Studio::open_mcp(
                &app.data.join("app.sqlite"),
                &app.binary,
                &app.data.join("mock/state.json"),
            )
            .await?,
        );
        let services = Services {
            agent: Arc::new(AgentService::new(studio.clone())),
            providers: ProviderService::new(studio.clone()),
            studio,
        };
        app.services
            .set(services)
            .map_err(|_| AppError::new("RUNTIME_ERROR", "Không thể mở Demo."))?;
    }
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.set_title(&format!(
            "CAD Agent Studio — Demo — {}",
            env!("CAD_BUILD_ID")
        ));
    }
    Ok(())
}
#[tauri::command]
async fn health(app: AppState<'_>) -> Result<String> {
    Ok(match app.services.get() {
        Some(services) => format!(
            "Rust IPC · SQLite {} · CAD mock via MCP",
            services.studio.sqlite_version().await?
        ),
        None => "Rust IPC · Live chưa kết nối · MCP chưa khởi chạy".into(),
    })
}
#[tauri::command]
async fn projects_list(app: AppState<'_>) -> Result<Vec<Project>> {
    app.studio()?.projects_list().await
}
#[tauri::command]
async fn projects_create(app: AppState<'_>, name: String) -> Result<Project> {
    app.studio()?.projects_create(name).await
}
#[tauri::command]
async fn cad_get_context(app: AppState<'_>, project_id: String) -> Result<DocumentContext> {
    app.studio()?.context(&project_id).await
}
#[tauri::command]
async fn design_propose_plate(
    app: AppState<'_>,
    project_id: String,
    parameters: PlateParameters,
    mode: RunMode,
) -> Result<ChangeSet> {
    app.studio()?.propose(&project_id, parameters, mode).await
}
#[tauri::command]
async fn changes_list(app: AppState<'_>, project_id: String) -> Result<Vec<ChangeSet>> {
    app.studio()?.changes_list(&project_id).await
}
#[tauri::command]
async fn changes_approve(
    app: AppState<'_>,
    agent: AgentState<'_>,
    change_id: String,
    payload_hash: String,
) -> Result<ExecutionReceipt> {
    let result = app.studio()?.approve(&change_id, &payload_hash).await;
    match &result {
        Ok(_) => {
            agent
                .agent()?
                .change_finished(&change_id, "verified")
                .await?
        }
        Err(e) if e.code == "OUTCOME_UNKNOWN" => {
            agent
                .agent()?
                .change_finished(&change_id, "outcome_unknown")
                .await?
        }
        _ => {}
    }
    result
}
#[tauri::command]
async fn changes_reject(app: AppState<'_>, agent: AgentState<'_>, change_id: String) -> Result<()> {
    app.studio()?.reject(&change_id).await?;
    agent.agent()?.change_finished(&change_id, "rejected").await
}
#[tauri::command]
async fn changes_revert(app: AppState<'_>, change_id: String) -> Result<ChangeSet> {
    app.studio()?.revert(&change_id).await
}
#[tauri::command]
async fn changes_recover(
    app: AppState<'_>,
    agent: AgentState<'_>,
    change_id: String,
) -> Result<ExecutionReceipt> {
    let receipt = app.studio()?.recover_change(&change_id).await?;
    agent
        .agent()?
        .change_finished(&change_id, "verified")
        .await?;
    Ok(receipt)
}
#[tauri::command]
async fn conversations_create(
    app: AppState<'_>,
    project_id: String,
    title: String,
) -> Result<Conversation> {
    app.studio()?.conversations_create(&project_id, title).await
}
#[tauri::command]
async fn conversations_list(app: AppState<'_>, project_id: String) -> Result<Vec<Conversation>> {
    app.studio()?.conversations_list(&project_id).await
}
#[tauri::command]
async fn conversation_snapshot(
    app: AppState<'_>,
    conversation_id: String,
) -> Result<ConversationSnapshot> {
    app.studio()?.conversation_snapshot(&conversation_id).await
}
#[tauri::command]
async fn agent_start(
    app: AppState<'_>,
    agent: AgentState<'_>,
    providers: State<'_, RuntimeState>,
    conversation_id: String,
    content: String,
    mode: RunMode,
) -> Result<AgentRun> {
    let conversation = app
        .studio()?
        .conversation_snapshot(&conversation_id)
        .await?
        .conversation;
    let provider = providers
        .providers()?
        .model_for(&conversation.project_id)
        .await?;
    agent
        .agent()?
        .start_run(&conversation_id, content, mode, provider)
        .await
}
#[tauri::command]
async fn agent_snapshot(agent: AgentState<'_>, run_id: String) -> Result<RunSnapshot> {
    agent.agent()?.snapshot(&run_id).await
}
#[tauri::command]
async fn agent_stop(agent: AgentState<'_>, run_id: String) -> Result<()> {
    agent.agent()?.cancel(&run_id).await
}
#[tauri::command]
async fn provider_get_config(
    providers: State<'_, RuntimeState>,
    project_id: String,
) -> Result<ProviderConfig> {
    providers.providers()?.config(&project_id).await
}
#[tauri::command]
async fn provider_save_config(
    providers: State<'_, RuntimeState>,
    project_id: String,
    provider: String,
    model: String,
    cloud_consent: bool,
    storage: String,
) -> Result<ProviderConfig> {
    providers
        .providers()?
        .save_config(&project_id, &provider, &model, cloud_consent, &storage)
        .await
}
#[tauri::command]
async fn provider_set_secret(
    providers: State<'_, RuntimeState>,
    project_id: String,
    secret: String,
) -> Result<()> {
    providers.providers()?.set_secret(&project_id, secret).await
}
#[tauri::command]
async fn provider_delete_secret(
    providers: State<'_, RuntimeState>,
    project_id: String,
) -> Result<()> {
    providers.providers()?.delete_secret(&project_id).await
}

#[tauri::command]
async fn account_list(app: AppState<'_>) -> Result<Vec<Account>> {
    app.live.accounts().await
}
#[tauri::command]
async fn account_add_key(
    app: AppState<'_>,
    label: String,
    secret: String,
    consent: bool,
) -> Result<Account> {
    app.live.add_api_key(&label, secret, consent).await
}
#[tauri::command]
async fn account_sign_in(
    app: AppState<'_>,
    account_id: Option<String>,
    consent: bool,
) -> Result<Account> {
    let (pending, url, cancel) = app.live.begin_login(account_id, consent).await?;
    if webbrowser::open(&url).is_err() {
        app.live.abandon_login().await;
        return Err(AppError::new(
            "BROWSER_FAILED",
            "Không mở được browser hệ thống.",
        ));
    }
    app.live.finish_login(pending, cancel).await
}
#[tauri::command]
async fn account_cancel_login(app: AppState<'_>) -> Result<()> {
    app.live.cancel_login().await;
    Ok(())
}
#[tauri::command]
async fn account_sign_out(app: AppState<'_>, account_id: String) -> Result<String> {
    app.live.sign_out(&account_id).await
}
#[tauri::command]
async fn model_catalog(app: AppState<'_>, account_id: String) -> Result<Account> {
    app.live.catalog(&account_id).await
}
#[tauri::command]
async fn account_select_model(
    app: AppState<'_>,
    account_id: String,
    model: String,
) -> Result<Account> {
    app.live.select_model(&account_id, &model).await
}
#[tauri::command]
async fn standalone_conversations(
    app: AppState<'_>,
    account_id: String,
) -> Result<Vec<serde_json::Value>> {
    app.live.conversations(&account_id).await
}
#[tauri::command]
async fn standalone_create(app: AppState<'_>, account_id: String) -> Result<String> {
    app.live.create_conversation(&account_id).await
}
#[tauri::command]
async fn standalone_messages(
    app: AppState<'_>,
    conversation_id: String,
) -> Result<Vec<serde_json::Value>> {
    app.live.messages(&conversation_id).await
}
#[tauri::command]
async fn standalone_start(
    app: AppState<'_>,
    conversation_id: String,
    content: String,
    probe: bool,
) -> Result<ChatRun> {
    app.live.start(&conversation_id, content, probe).await
}
#[tauri::command]
async fn standalone_snapshot(app: AppState<'_>, run_id: String) -> Result<ChatRun> {
    app.live.snapshot(&run_id).await
}
#[tauri::command]
async fn standalone_cancel(app: AppState<'_>, run_id: String) -> Result<()> {
    app.live.cancel(&run_id).await
}

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data = app.path().app_data_dir()?;

            let exe = std::env::current_exe()?;
            let binary = exe
                .parent()
                .ok_or("Executable directory unavailable")?
                .join(if cfg!(windows) {
                    "cad-tool-host.exe"
                } else {
                    "cad-tool-host"
                });
            std::fs::create_dir_all(&data)?;
            let live = tauri::async_runtime::block_on(AppRuntime::open(
                &data.join("app.sqlite"),
                &data.join("oauth-vault"),
            ))?;
            app.manage(RuntimeState {
                live,
                services: OnceLock::new(),
                initializing: tauri::async_runtime::Mutex::new(()),
                data,
                binary,
            });
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&format!(
                    "CAD Agent Studio — CAD chưa kết nối — {}",
                    env!("CAD_BUILD_ID")
                ))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            account_list,
            account_add_key,
            account_sign_in,
            account_cancel_login,
            account_sign_out,
            model_catalog,
            account_select_model,
            standalone_conversations,
            standalone_create,
            standalone_messages,
            standalone_start,
            standalone_snapshot,
            standalone_cancel,
            cad_candidates,
            cad_pair,
            cad_live_status,
            cad_disconnect,
            health,
            runtime_status,
            demo_enter,
            projects_list,
            projects_create,
            cad_get_context,
            design_propose_plate,
            changes_list,
            changes_approve,
            changes_reject,
            changes_revert,
            changes_recover,
            conversations_create,
            conversations_list,
            conversation_snapshot,
            agent_start,
            agent_snapshot,
            agent_stop,
            provider_get_config,
            provider_save_config,
            provider_set_secret,
            provider_delete_secret
        ])
        .build(tauri::generate_context!())
        .expect("Không thể khởi động CAD Agent Studio; kiểm tra sidecar và dữ liệu ứng dụng");
    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            tauri::async_runtime::block_on(async {
                handle.state::<RuntimeState>().live.shutdown().await;
                if let Some(services) = handle.state::<RuntimeState>().services.get() {
                    services.agent.shutdown().await;
                    services.studio.shutdown().await;
                }
            });
        }
    });
}
