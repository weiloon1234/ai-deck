use super::{controlled_environment, LaunchSpec};
use crate::{
    error::Result,
    runtime_client::endpoint,
    state_store::{atomic_write, private_directory},
    types::*,
};
use std::path::{Path, PathBuf};

pub fn prepare(
    executable: PathBuf,
    session: &Session,
    deployment: &Deployment,
    token: &str,
    resume: bool,
) -> Result<LaunchSpec> {
    let directory = Path::new(&session.config_directory);
    private_directory(directory)?;
    let base = format!(
        "{}/v1",
        endpoint(deployment, deployment.profile.runtime.inference_port)?
    );
    // JSON string literals are valid TOML basic strings for these validated values.
    let model = serde_json::to_string(&deployment.profile.model.served_model).unwrap();
    // Apply the same critical settings in the owned file and at CLI precedence.
    // A project config cannot replace this provider's URL or credential source.
    // Preserve the provider ID stored in existing Codex conversations across app renames.
    let settings = vec![
        ("model", model),
        ("model_provider", "\"runpod_deck\"".into()),
        ("web_search", "\"disabled\"".into()),
        ("cli_auth_credentials_store", "\"file\"".into()),
        ("model_context_window", deployment.profile.model.context_tokens.to_string()),
        ("model_providers.runpod_deck", format!("{{ name = \"AI Deck\", base_url = {}, wire_api = \"responses\", env_key = \"RUNPOD_DECK_INFERENCE_TOKEN\", requires_openai_auth = false }}", serde_json::to_string(&base).unwrap())),
    ];
    let config = settings
        .iter()
        .map(|(key, value)| format!("{key} = {value}\n"))
        .collect::<String>();
    atomic_write(&directory.join("config.toml"), config.as_bytes())?;
    let mut env = controlled_environment();
    env.insert("CODEX_HOME".into(), session.config_directory.clone());
    env.insert("RUNPOD_DECK_INFERENCE_TOKEN".into(), token.into());
    let mut arguments = vec![
        "--no-daemon".into(),
        "--sandbox".into(),
        "workspace-write".into(),
        "--ask-for-approval".into(),
        "on-request".into(),
        "--model".into(),
        deployment.profile.model.served_model.clone(),
        "--cd".into(),
        session.project_path.clone(),
    ];
    for (key, value) in settings {
        arguments.extend(["--config".into(), format!("{key}={value}")]);
    }
    if resume {
        arguments.push("resume".into());
        if let Some(id) = &session.conversation_id {
            arguments.push(id.clone());
        } else {
            arguments.push("--last".into());
        }
    }
    Ok(LaunchSpec {
        executable,
        arguments,
        environment: env,
        directory: PathBuf::from(&session.project_path),
        redacted_values: vec![token.into()],
    })
}
