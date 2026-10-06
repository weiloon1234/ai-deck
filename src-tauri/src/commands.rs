use crate::{
    app_core::AppCore,
    cli_adapters,
    credential_store::RUNPOD_KEY,
    deployment_service::validate_policy,
    error::{AppError, Result},
    model_catalog,
    runpod_client::Hardware,
    session_service::validate_project,
    state_store::atomic_write,
    types::*,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{Manager, State};

#[derive(Default)]
pub struct ExitControl(pub AtomicBool);
type Core<'a> = State<'a, Arc<AppCore>>;

#[tauri::command]
pub fn snapshot(core: Core<'_>) -> Result<AppSnapshot> {
    core.snapshot()
}
#[tauri::command]
pub async fn save_runpod_key(core: Core<'_>, key: String) -> Result<()> {
    let _guard = core.operations.lock().await;
    let key = key.trim();
    if key.len() < 16 || key.len() > 1024 || key.chars().any(char::is_whitespace) {
        return Err(AppError::invalid("Enter a valid Runpod API key."));
    }
    if core
        .store
        .read()?
        .deployments
        .iter()
        .any(|d| !d.stage.is_terminal())
    {
        return Err(AppError::invalid("Finish existing deployments before replacing the management key, so cleanup keeps the same account."));
    }
    core.credentials.set(RUNPOD_KEY, key)?;
    core.changed();
    Ok(())
}
#[tauri::command]
pub async fn remove_runpod_key(core: Core<'_>) -> Result<()> {
    let _guard = core.operations.lock().await;
    if core
        .store
        .read()?
        .deployments
        .iter()
        .any(|d| !d.stage.is_terminal())
    {
        return Err(AppError::invalid(
            "Finish existing deployments before removing the management key.",
        ));
    }
    core.credentials.delete(RUNPOD_KEY)?;
    core.changed();
    Ok(())
}
#[tauri::command]
pub async fn save_settings(core: Core<'_>, settings: AppSettings) -> Result<()> {
    let _guard = core.operations.lock().await;
    validate_policy(&settings.policy)?;
    if settings
        .hugging_face_secret
        .as_deref()
        .is_some_and(|s| !model_catalog::identifier(s))
    {
        return Err(AppError::invalid(
            "Use the Runpod secret name, not its token value.",
        ));
    }
    for path in [&settings.codex_path, &settings.claude_path]
        .into_iter()
        .flatten()
    {
        cli_adapters::executable_path(Path::new(path))?;
    }
    core.store.update(|s| {
        let at = now();
        let spent: f64 = s
            .deployments
            .iter()
            .map(|d| crate::app_core::estimated_cost(d, at))
            .sum();
        for d in s.deployments.iter_mut().filter(|d| !d.stage.is_terminal()) {
            if let Some(minutes) = settings.policy.max_lifetime_minutes {
                d.deadline_at = d.deadline_at.min(d.created_at.saturating_add(minutes * 60));
            }
            if let Some(total) = settings.policy.total_budget_usd {
                let seconds = ((total - spent).max(0.0) / d.hourly_usd * 3600.0) as u32;
                d.deadline_at = d.deadline_at.min(at.saturating_add(seconds));
            }
        }
        s.settings = settings;
        Ok(())
    })?;
    core.changed();
    Ok(())
}
#[tauri::command]
pub async fn detect_clis(core: Core<'_>) -> Result<Vec<CliInstallation>> {
    let settings = core.store.read()?.settings;
    tokio::task::spawn_blocking(move || {
        vec![
            cli_adapters::discover(CliKind::Codex, settings.codex_path.as_deref()),
            cli_adapters::discover(CliKind::Claude, settings.claude_path.as_deref()),
        ]
    })
    .await
    .map_err(|_| AppError::invalid("CLI discovery failed."))
}
#[tauri::command]
pub async fn choose_executable() -> Result<Option<String>> {
    Ok(rfd::AsyncFileDialog::new()
        .set_title("Choose an installed coding CLI executable")
        .pick_file()
        .await
        .map(|f| f.path().to_string_lossy().into()))
}
#[tauri::command]
pub async fn choose_project(core: Core<'_>) -> Result<Option<Project>> {
    let Some(folder) = rfd::AsyncFileDialog::new()
        .set_title("Choose a local project folder")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    let path = validate_project(&folder.path().to_string_lossy())?;
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path: path.to_string_lossy().into(),
    };
    let saved = core.store.update(|s| {
        if let Some(existing) = s.projects.iter().find(|p| p.path == project.path) {
            return Ok(existing.clone());
        }
        s.projects.push(project.clone());
        Ok(project)
    })?;
    core.changed();
    Ok(Some(saved))
}
#[tauri::command]
pub fn remove_project(core: Core<'_>, id: String) -> Result<()> {
    core.store.update(|s| {
        if s.sessions
            .iter()
            .any(|session| session.project_id == id && session.status != SessionStatus::Ended)
        {
            return Err(AppError::invalid(
                "Close this project's sessions before removing it from the list.",
            ));
        }
        s.projects.retain(|p| p.id != id);
        Ok(())
    })?;
    core.changed();
    Ok(())
}
#[tauri::command]
pub async fn hardware(core: Core<'_>) -> Result<Hardware> {
    core.provider.hardware().await
}
#[tauri::command]
pub async fn provision(core: Core<'_>, request: ProvisionRequest) -> Result<String> {
    core.provision(request).await
}
#[tauri::command]
pub async fn reconcile(core: Core<'_>) -> Result<()> {
    core.reconcile_all().await
}
#[tauri::command]
pub async fn enable(core: Core<'_>, id: String) -> Result<()> {
    core.enable(&id).await
}
#[tauri::command]
pub async fn finish(core: Core<'_>, id: String) -> Result<()> {
    core.finish(&id).await
}
#[tauri::command]
pub async fn confirm_absent_creation(core: Core<'_>, id: String, typed_name: String) -> Result<()> {
    core.confirm_absent_creation(&id, &typed_name).await
}
#[tauri::command]
pub async fn launch_session(core: Core<'_>, request: LaunchRequest) -> Result<String> {
    core.inner().launch(request).await
}
#[tauri::command]
pub async fn resume_session(core: Core<'_>, id: String, allow_unverified: bool) -> Result<()> {
    core.inner().resume(&id, allow_unverified).await
}
#[tauri::command]
pub fn close_session(core: Core<'_>, id: String) -> Result<()> {
    core.close_session(&id)
}
#[tauri::command]
pub fn rename_session(core: Core<'_>, id: String, name: String) -> Result<()> {
    core.rename_session(&id, &name)
}
#[tauri::command]
pub fn terminal_input(core: Core<'_>, id: String, data: String) -> Result<()> {
    core.terminals.input(&id, &data)
}
#[tauri::command]
pub fn terminal_resize(core: Core<'_>, id: String, rows: u16, cols: u16) -> Result<()> {
    core.terminals.resize(&id, rows, cols)
}
#[tauri::command]
pub fn terminal_replay(core: Core<'_>, id: String) -> Result<TerminalReplay> {
    core.terminals.replay(&id)
}
#[tauri::command]
pub async fn import_catalog(core: Core<'_>) -> Result<bool> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("Model catalog", &["json"])
        .pick_file()
        .await
    else {
        return Ok(false);
    };
    let metadata = std::fs::metadata(file.path()).map_err(|_| AppError::storage())?;
    if metadata.len() > 1024 * 1024 {
        return Err(AppError::invalid("Catalog exceeds the 1 MiB limit."));
    }
    let contents = std::fs::read_to_string(file.path()).map_err(|_| AppError::storage())?;
    model_catalog::parse_catalog(&contents)?;
    atomic_write(
        &core.store.directory().join("catalog.json"),
        contents.as_bytes(),
    )?;
    core.changed();
    Ok(true)
}
#[tauri::command]
pub async fn import_hugging_face_model(
    core: Core<'_>,
    url: String,
    reanalyze: bool,
) -> Result<crate::model_import_service::ModelImportResult> {
    core.import_hugging_face_model(&url, reanalyze).await
}
#[tauri::command]
pub fn cancel_model_import(core: Core<'_>) {
    core.model_imports.cancel();
}
#[tauri::command]
pub async fn refresh_model_prices(core: Core<'_>, id: String) -> Result<()> {
    core.refresh_model_prices(&id).await
}
#[tauri::command]
pub fn remove_imported_model(core: Core<'_>, id: String) -> Result<()> {
    core.remove_imported_model(&id)
}
#[tauri::command]
pub fn open_external(url: String) -> Result<()> {
    let parsed =
        reqwest::Url::parse(&url).map_err(|_| AppError::invalid("Invalid documentation link."))?;
    let allowed = parsed.scheme() == "https"
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.port().is_none()
        && match parsed.host_str() {
            Some("console.runpod.io") => matches!(parsed.path(), "/pods" | "/user/settings"),
            Some("huggingface.co") => {
                parsed.path().trim_start_matches('/').split('/').count() == 2
                    && parsed
                        .path()
                        .trim_start_matches('/')
                        .split('/')
                        .all(model_catalog::identifier)
            }
            _ => false,
        }
        && parsed.query().is_none()
        && parsed.fragment().is_none();
    if !allowed {
        return Err(AppError::invalid(
            "This link is outside the app's supported provider pages.",
        ));
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg(parsed.as_str())
            .spawn()
            .map_err(|_| AppError::invalid("The browser could not open this link."))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(AppError::invalid(
            "External links are currently implemented for macOS only.",
        ))
    }
}

#[tauri::command]
pub async fn export_diagnostics(core: Core<'_>) -> Result<bool> {
    let state = core.store.read()?;
    // Explicit allowlist; no settings, project paths, terminal output, env, or secrets.
    let deployments: Vec<_> = state.deployments.iter().map(|d| serde_json::json!({"deploymentId":d.id,"podIds":d.pod_ids,
        "profileId":d.profile.model.id,"fingerprint":d.profile.fingerprint,"stage":d.stage,"message":d.message,"failureReason":d.failure_reason,
        "createdAt":d.created_at,"terminatedAt":d.terminated_at,"verifiedAt":d.verified_at})).collect();
    let report = serde_json::json!({"appVersion":env!("CARGO_PKG_VERSION"),"generatedAt":now(),"offlineExpiryVerified":false,"deployments":deployments});
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_file_name("ai-deck-diagnostics.json")
        .save_file()
        .await
    else {
        return Ok(false);
    };
    atomic_write(
        file.path(),
        &serde_json::to_vec_pretty(&report).map_err(|_| AppError::storage())?,
    )?;
    Ok(true)
}
#[tauri::command]
pub async fn request_quit(
    app: tauri::AppHandle,
    core: Core<'_>,
    finish_pods: bool,
    acknowledge_running_costs: bool,
) -> Result<()> {
    let state = core.store.read()?;
    let active: Vec<_> = state
        .deployments
        .iter()
        .filter(|d| !d.stage.is_terminal())
        .map(|d| d.id.clone())
        .collect();
    if !active.is_empty() && !finish_pods && !acknowledge_running_costs {
        return Err(AppError::invalid(
            "Choose whether to finish active Pods or explicitly leave their billing running.",
        ));
    }
    if finish_pods {
        for id in &active {
            core.finish(id).await?;
        }
        if core
            .store
            .read()?
            .deployments
            .iter()
            .any(|d| !d.stage.is_terminal())
        {
            return Err(AppError::new(
                "cleanup_pending",
                "Pod deletion is not confirmed, so Finish and Quit has not completed.",
                "Reconcile cleanup or explicitly choose to quit with running costs.",
            ));
        }
    }
    core.model_imports.shutdown().await?;
    core.terminals.shutdown().await?;
    app.state::<ExitControl>().0.store(true, Ordering::Release);
    app.exit(0);
    Ok(())
}
