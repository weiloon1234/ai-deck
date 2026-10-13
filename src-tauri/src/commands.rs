use crate::{
    app_core::AppCore,
    cli_adapters,
    cloud_provider::{Hardware, ProviderId},
    deployment_service::validate_policy,
    error::{AppError, Result},
    model_catalog,
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
    core.save_provider_key(ProviderId::Runpod, &key).await
}
#[tauri::command]
pub async fn remove_runpod_key(core: Core<'_>) -> Result<()> {
    core.remove_provider_key(ProviderId::Runpod).await
}
#[tauri::command]
pub async fn save_provider_key(core: Core<'_>, provider: ProviderId, key: String) -> Result<()> {
    core.save_provider_key(provider, &key).await
}
#[tauri::command]
pub async fn remove_provider_key(core: Core<'_>, provider: ProviderId) -> Result<()> {
    core.remove_provider_key(provider).await
}
#[tauri::command]
pub async fn check_provider_connection(core: Core<'_>, provider: ProviderId) -> Result<()> {
    core.check_provider_connection(provider).await
}
#[tauri::command]
pub async fn select_provider(core: Core<'_>, provider: ProviderId) -> Result<()> {
    core.select_provider(provider).await
}
#[tauri::command]
pub async fn save_settings(core: Core<'_>, mut settings: AppSettings) -> Result<()> {
    let _guard = core.operations.lock().await;
    // Provider selection has its own immediate-save command; an older form must not undo it.
    settings.provider = core.store.read()?.settings.provider;
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
pub async fn hardware(core: Core<'_>, provider: ProviderId) -> Result<Hardware> {
    core.hardware_for(provider).await
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
pub async fn configure_imported_model(
    core: Core<'_>,
    request: crate::model_import_service::ConfigureImportedModelRequest,
) -> Result<()> {
    core.configure_imported_model(request).await
}
#[tauri::command]
pub async fn imported_model_hardware(core: Core<'_>, id: String) -> Result<Hardware> {
    core.imported_model_hardware(&id).await
}
#[tauri::command]
pub fn remove_imported_model(core: Core<'_>, id: String) -> Result<()> {
    core.remove_imported_model(&id)
}
fn validated_external_url(url: &str) -> Result<reqwest::Url> {
    let parsed =
        reqwest::Url::parse(url).map_err(|_| AppError::invalid("Invalid documentation link."))?;
    let allowed = parsed.scheme() == "https"
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.port().is_none()
        && match parsed.host_str() {
            Some("console.runpod.io") => matches!(parsed.path(), "/pods" | "/user/settings"),
            Some("gputw.ai") => parsed.path() == "/",
            Some("docs.gputw.ai") => parsed.path() == "/zh-TW/docs/api-keys",
            Some("huggingface.co") => {
                let parts: Vec<_> = parsed.path().trim_start_matches('/').split('/').collect();
                match parts.as_slice() {
                    [organization, repository] => {
                        model_catalog::identifier(organization)
                            && model_catalog::identifier(repository)
                    }
                    [organization, repository, "blob", revision, "config.json"] => {
                        model_catalog::identifier(organization)
                            && model_catalog::identifier(repository)
                            && revision.len() == 40
                            && revision.bytes().all(|b| b.is_ascii_hexdigit())
                    }
                    _ => false,
                }
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
    Ok(parsed)
}
#[tauri::command]
pub fn open_external(url: String) -> Result<()> {
    let parsed = validated_external_url(&url)?;
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

#[cfg(test)]
mod tests {
    use super::validated_external_url;

    const REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn external_links_allow_provider_pages_and_pinned_model_config() {
        for url in [
            "https://console.runpod.io/pods".to_owned(),
            "https://console.runpod.io/user/settings".to_owned(),
            "https://gputw.ai/".to_owned(),
            "https://docs.gputw.ai/zh-TW/docs/api-keys".to_owned(),
            "https://huggingface.co/fixture/coding-model".to_owned(),
            format!("https://huggingface.co/fixture/coding-model/blob/{REVISION}/config.json"),
        ] {
            assert_eq!(validated_external_url(&url).unwrap().as_str(), url);
        }
    }

    #[test]
    fn external_links_reject_unpinned_configs_and_unapproved_urls() {
        for url in [
            "not a URL".to_owned(),
            "http://huggingface.co/fixture/coding-model".to_owned(),
            "https://huggingface.co.evil/fixture/coding-model".to_owned(),
            "https://user@huggingface.co/fixture/coding-model".to_owned(),
            "https://huggingface.co:8443/fixture/coding-model".to_owned(),
            "https://huggingface.co/fixture/coding-model?token=fake".to_owned(),
            "https://huggingface.co/fixture/coding-model#config".to_owned(),
            "https://console.runpod.io/account".to_owned(),
            "https://gputw.ai.evil/".to_owned(),
            "https://gputw.ai/api/instances/create".to_owned(),
            "https://docs.gputw.ai/zh-TW/docs/api-keys?token=fake".to_owned(),
            "https://huggingface.co/fixture/coding-model/blob/main/config.json".to_owned(),
            format!("https://huggingface.co/fixture/coding-model/tree/{REVISION}/config.json"),
            format!("https://huggingface.co/fixture/coding-model/blob/{REVISION}/README.md"),
            format!(
                "https://huggingface.co/fixture/coding-model/blob/{REVISION}/config.json/extra"
            ),
            format!("https://huggingface.co/fixture/coding-model/blob/{REVISION}z/config.json"),
            format!("https://huggingface.co/fixture%2fcoding-model/blob/{REVISION}/config.json"),
        ] {
            assert!(validated_external_url(&url).is_err(), "accepted {url}");
        }
    }
}
