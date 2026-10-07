//! Explicit opt-in native acceptance runner. Uses the app vault, never exports credentials.
use cad_app_core::standalone::AppRuntime;
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};

#[tokio::main]
async fn main() {
    if let Err(e) = verify().await {
        // App errors have fixed, safe messages. No token/HTTP body is logged here.
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn verify() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 || args[1] != "--live" {
        return Err("Usage: verify_live --live APP_DATA_DIR EVIDENCE_JSON (real plan usage; close desktop first)".into());
    }
    let data = PathBuf::from(&args[2]);
    let output = PathBuf::from(&args[3]);
    let db = data.join("app.sqlite");
    if !db.is_file() {
        return Err("Existing application database required".into());
    }
    let app = AppRuntime::open(&db, &data.join("oauth-vault")).await?;
    let eligible: Vec<_> = app
        .accounts()
        .await?
        .into_iter()
        .filter(|a| {
            a.auth_mode == "chatgpt"
                && !["signed_out", "missing_permission"].contains(&a.status.as_str())
        })
        .collect();
    if eligible.len() != 1 {
        return Err(
            "Exactly one signed-in ChatGPT profile required; no implicit account or billing switch"
                .into(),
        );
    }
    let mut evidence = json!({"scope":"Native AppRuntime; not full desktop UI E2E", "cad":"disconnected", "auth_route":"chatgpt", "auth_status":eligible[0].status, "catalog":"NOT_RUN", "probe":"NOT_RUN", "chat":"NOT_RUN", "restart":"NOT_RUN"});
    let save = |e: &Value| -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(&output, serde_json::to_vec_pretty(e)?)?;
        Ok(())
    };
    save(&evidence)?;
    let account = match app.catalog(&eligible[0].id).await {
        Ok(a) => a,
        Err(e) => {
            evidence["catalog"] = json!({"result":"FAILED","code":e.code,"message":e.message});
            save(&evidence)?;
            return Err(e.into());
        }
    };
    let model = account
        .selected_model
        .clone()
        .filter(|m| account.models.iter().any(|x| &x.id == m))
        .or_else(|| account.models.first().map(|m| m.id.clone()))
        .ok_or("Catalog contains no available model")?;
    evidence["catalog"] = json!({"result":"LIVE_PASS","count":account.models.len(),"selected_model":model,"selection":"preserve saved model or first actual catalog entry"});
    save(&evidence)?;
    app.select_model(&account.id, &model).await?;
    let conversation = app.create_conversation(&account.id).await?;
    for (kind, prompt, probe) in [("probe", "", true), ("chat", "Tôi muốn thiết kế sơ bộ nhà 8x12 m. Hãy nêu ngắn gọn thông tin cần cung cấp. Chưa kết nối AutoCAD.", false)] {
        let started = app.start(&conversation, prompt.into(), probe).await?;
        let final_run = tokio::time::timeout(Duration::from_secs(200), async {
            loop {let run=app.snapshot(&started.id).await?;if run.status!="running" {break Ok::<_,cad_contracts::AppError>(run);}tokio::time::sleep(Duration::from_millis(300)).await;}
        }).await??;
        evidence[kind] = json!({"result":if final_run.status=="completed" {"LIVE_PASS"}else{"FAILED"},"run_id":final_run.id,"status":final_run.status,"error_code":final_run.error_code,"text":final_run.text,"trace":final_run.trace});
        save(&evidence)?;
        if final_run.status != "completed" {return Err(format!("{kind} failed; see safe evidence").into());}
    }
    let messages = app.messages(&conversation).await?;
    app.shutdown().await;
    drop(app);
    let reopened = AppRuntime::open(&db, &data.join("oauth-vault")).await?;
    let restored = reopened.messages(&conversation).await?;
    let accounts = reopened.accounts().await?;
    let preserved = restored == messages
        && restored.len() >= 4
        && accounts
            .iter()
            .any(|a| a.id == account.id && a.selected_model.as_deref() == Some(&model));
    evidence["restart"] = json!({"result":if preserved {"NATIVE_PASS"}else{"FAILED"},"messages":restored.len(),"scope":"runtime drop/reopen, not GUI restart"});
    save(&evidence)?;
    reopened.shutdown().await;
    if !preserved {
        return Err("Persistence check failed".into());
    }
    println!(
        "Catalog, probe, chat and runtime reopen passed. Evidence: {}",
        output.display()
    );
    Ok(())
}
