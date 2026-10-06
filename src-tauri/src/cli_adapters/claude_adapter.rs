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
    let model = &deployment.profile.model.served_model;
    // --bare prevents subscription Keychain/OAuth reads; empty setting sources
    // prevents project settings from replacing the gateway or its credential.
    let settings = serde_json::json!({"model":model,"permissions":{"defaultMode":"manual"},"disableAllHooks":true});
    let settings_path = directory.join("settings.json");
    atomic_write(
        &settings_path,
        &serde_json::to_vec_pretty(&settings).unwrap(),
    )?;
    let mut env = controlled_environment();
    env.insert("CLAUDE_CONFIG_DIR".into(), session.config_directory.clone());
    env.insert(
        "ANTHROPIC_BASE_URL".into(),
        endpoint(deployment, deployment.profile.runtime.inference_port)?,
    );
    env.insert("ANTHROPIC_API_KEY".into(), token.into());
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), token.into());
    for key in [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_SMALL_FAST_MODEL",
        "CLAUDE_CODE_SUBAGENT_MODEL",
    ] {
        env.insert(key.into(), model.clone());
    }
    env.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".into(),
        "1".into(),
    );
    env.insert("DISABLE_AUTOUPDATER".into(), "1".into());
    let mut arguments = vec![
        "--bare".into(),
        "--setting-sources".into(),
        "".into(),
        "--settings".into(),
        settings_path.to_string_lossy().into(),
        "--strict-mcp-config".into(),
        "--no-chrome".into(),
        "--permission-mode".into(),
        "manual".into(),
        "--model".into(),
        model.clone(),
    ];
    arguments.extend([
        if resume {
            "--resume".into()
        } else {
            "--session-id".into()
        },
        session
            .conversation_id
            .clone()
            .unwrap_or_else(|| session.id.clone()),
    ]);
    Ok(LaunchSpec {
        executable,
        arguments,
        environment: env,
        directory: PathBuf::from(&session.project_path),
        redacted_values: vec![token.into()],
    })
}
