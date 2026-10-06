use crate::{
    app_core::AppCore,
    cli_adapters::{self, claude_adapter, codex_adapter},
    error::{AppError, Result},
    state_store::private_directory,
    types::*,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

impl AppCore {
    pub async fn launch(self: &Arc<Self>, request: LaunchRequest) -> Result<String> {
        let _guard = self.operations.lock().await;
        let state = self.store.read()?;
        let deployment_id = state.enabled_deployment_id.as_deref().ok_or_else(|| {
            AppError::new(
                "no_enabled_endpoint",
                "Enable a ready deployment before opening a connected CLI.",
                "Provision Runpod, wait for Ready, then select Enable.",
            )
        })?;
        let deployment = self.ensure_ready_locked(deployment_id).await?;
        let project = state
            .projects
            .iter()
            .find(|p| p.id == request.project_id)
            .ok_or_else(|| AppError::invalid("Choose a local project."))?;
        validate_project(&project.path)?;
        self.check_capacity(&deployment, None)?;
        let configured = match request.cli {
            CliKind::Codex => state.settings.codex_path.clone(),
            CliKind::Claude => state.settings.claude_path.clone(),
        };
        if request.cli == CliKind::Claude && !state.settings.policy.claude_enabled {
            return Err(AppError::invalid(
                "Enable the experimental Claude adapter in Setup first.",
            ));
        }
        let cli = request.cli.clone();
        let installation =
            tokio::task::spawn_blocking(move || cli_adapters::discover(cli, configured.as_deref()))
                .await
                .map_err(|_| AppError::invalid("CLI discovery failed."))?;
        let version = installation
            .version
            .ok_or_else(|| AppError::invalid(installation.message))?;
        cli_adapters::assert_known_launcher(&request.cli, &version)?;
        let validated =
            deployment
                .profile
                .is_validated(&request.cli, &version, &deployment.gpu_type);
        if !validated && !request.allow_unverified {
            return Err(AppError::new("compatibility_unverified", "This exact CLI/model/runtime combination has no complete coding-test evidence.", "Use the explicit candidate-test option when you are ready to validate it. It will not be labeled supported."));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let directory = self.store.directory().join("sessions").join(&id);
        private_directory(&directory)?;
        let session = Session {
            id: id.clone(),
            name: session_name(&request.name)?,
            project_id: project.id.clone(),
            project_path: project.path.clone(),
            cli: request.cli.clone(),
            cli_version: version,
            deployment_id: deployment.id.clone(),
            profile_id: deployment.profile.model.id.clone(),
            profile_version: deployment.profile.model.version,
            model: deployment.profile.model.served_model.clone(),
            status: SessionStatus::Starting,
            created_at: now(),
            ended_at: None,
            conversation_id: (request.cli == CliKind::Claude).then(|| id.clone()),
            config_directory: directory.to_string_lossy().into(),
            unverified_combination: !validated,
            exit_code: None,
        };
        self.store.update(|s| {
            s.sessions.push(session.clone());
            if let Some(d) = s.deployments.iter_mut().find(|d| d.id == deployment.id) {
                d.cleanup_due_at = None;
            }
            Ok(())
        })?;
        self.spawn_session(
            session,
            deployment,
            PathBuf::from(installation.path.unwrap()),
            false,
        )?;
        Ok(id)
    }
    pub async fn resume(self: &Arc<Self>, id: &str, allow_unverified: bool) -> Result<()> {
        let _guard = self.operations.lock().await;
        let state = self.store.read()?;
        let mut session = state
            .sessions
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or_else(|| AppError::invalid("Session not found."))?;
        if self.terminals.is_running(id) {
            return Err(AppError::invalid("This session is already running."));
        }
        // Resumption intentionally uses the original binding, never the currently enabled model.
        let deployment = self.ensure_ready_locked(&session.deployment_id).await?;
        validate_project(&session.project_path)?;
        self.check_capacity(&deployment, Some(id))?;
        if session.cli == CliKind::Claude && !state.settings.policy.claude_enabled {
            return Err(AppError::invalid("The Claude adapter is disabled."));
        }
        let configured = match session.cli {
            CliKind::Codex => state.settings.codex_path.clone(),
            CliKind::Claude => state.settings.claude_path.clone(),
        };
        let cli = session.cli.clone();
        let installation =
            tokio::task::spawn_blocking(move || cli_adapters::discover(cli, configured.as_deref()))
                .await
                .map_err(|_| AppError::invalid("CLI discovery failed."))?;
        let version = installation
            .version
            .ok_or_else(|| AppError::invalid(installation.message))?;
        cli_adapters::assert_known_launcher(&session.cli, &version)?;
        if !deployment
            .profile
            .is_validated(&session.cli, &version, &deployment.gpu_type)
            && !allow_unverified
        {
            return Err(AppError::invalid(
                "Acknowledge the unverified combination before resuming a candidate test.",
            ));
        }
        session.unverified_combination =
            !deployment
                .profile
                .is_validated(&session.cli, &version, &deployment.gpu_type);
        session.cli_version = version;
        session.status = SessionStatus::Starting;
        session.ended_at = None;
        session.exit_code = None;
        self.store.update(|s| {
            *s.sessions.iter_mut().find(|s| s.id == id).unwrap() = session.clone();
            if let Some(d) = s.deployments.iter_mut().find(|d| d.id == deployment.id) {
                d.cleanup_due_at = None;
            }
            Ok(())
        })?;
        self.spawn_session(
            session,
            deployment,
            PathBuf::from(installation.path.unwrap()),
            true,
        )
    }
    fn check_capacity(&self, deployment: &Deployment, excluding: Option<&str>) -> Result<()> {
        let active = self
            .store
            .read()?
            .sessions
            .iter()
            .filter(|s| {
                s.deployment_id == deployment.id
                    && Some(s.id.as_str()) != excluding
                    && matches!(
                        s.status,
                        SessionStatus::Running
                            | SessionStatus::Starting
                            | SessionStatus::Disconnected
                    )
            })
            .count();
        if active >= deployment.profile.model.max_sessions as usize {
            return Err(AppError::new("session_capacity", "This deployment has reached its configured session limit.", "Close a session before opening another. Candidate capacity still requires live validation."));
        }
        Ok(())
    }
    fn spawn_session(
        self: &Arc<Self>,
        session: Session,
        deployment: Deployment,
        executable: PathBuf,
        resume: bool,
    ) -> Result<()> {
        let result = (|| {
            let token = self
                .credentials
                .get(&deployment.credential_ref)?
                .ok_or_else(|| AppError::invalid("The deployment credential is missing."))?;
            let spec =
                match session.cli {
                    CliKind::Codex if deployment.profile.model.supports_responses => {
                        codex_adapter::prepare(executable, &session, &deployment, &token, resume)?
                    }
                    CliKind::Claude if deployment.profile.model.supports_messages => {
                        claude_adapter::prepare(executable, &session, &deployment, &token, resume)?
                    }
                    _ => return Err(AppError::invalid(
                        "This profile does not expose the protocol required by the selected CLI.",
                    )),
                };
            let weak = Arc::downgrade(self);
            let id = session.id.clone();
            self.terminals.spawn(
                &session.id,
                spec,
                self.notify.clone(),
                Arc::new(move |code| {
                    if let Some(core) = weak.upgrade() {
                        let _ = core.session_exited(&id, code);
                    }
                }),
            )
        })();
        match result {
            Ok(()) => {
                self.store.update(|s| {
                    if let Some(item) = s.sessions.iter_mut().find(|s| s.id == session.id) {
                        if item.status == SessionStatus::Starting {
                            item.status = SessionStatus::Running;
                        }
                    }
                    Ok(())
                })?;
            }
            Err(error) => {
                self.session_exited(&session.id, 1)?;
                return Err(error);
            }
        }
        self.changed();
        Ok(())
    }
    pub fn close_session(&self, id: &str) -> Result<()> {
        self.terminals.close(id)
    }
    fn deployment_session_ids(&self, id: &str) -> Result<Vec<String>> {
        Ok(self
            .store
            .read()?
            .sessions
            .into_iter()
            .filter(|s| s.deployment_id == id)
            .map(|s| s.id)
            .collect())
    }
    pub(crate) fn stop_deployment_sessions(&self, id: &str) -> Result<()> {
        for sid in self.deployment_session_ids(id)? {
            self.terminals.close(&sid)?;
        }
        Ok(())
    }
    pub(crate) async fn wait_deployment_sessions(&self, id: &str) -> Result<()> {
        self.terminals
            .wait_for(&self.deployment_session_ids(id)?)
            .await
    }
    fn session_exited(&self, id: &str, code: u32) -> Result<()> {
        self.store.update(|state| {
            let Some(session) = state.sessions.iter_mut().find(|s| s.id == id) else {
                return Ok(());
            };
            session.status = SessionStatus::Ended;
            session.ended_at = Some(now());
            session.exit_code = Some(code);
            let deployment_id = session.deployment_id.clone();
            state.arm_idle_cleanup(&deployment_id, now());
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
    pub fn rename_session(&self, id: &str, name: &str) -> Result<()> {
        let name = session_name(name)?;
        self.store.update(|s| {
            s.sessions
                .iter_mut()
                .find(|s| s.id == id)
                .ok_or_else(|| AppError::invalid("Session not found."))?
                .name = name;
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
}
pub fn validate_project(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    if !path.is_absolute() || !path.is_dir() {
        return Err(AppError::invalid(
            "Choose an existing local project folder.",
        ));
    }
    path.canonicalize()
        .map_err(|_| AppError::invalid("Project folder could not be resolved."))
}
fn session_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(AppError::invalid(
            "Use a session name of 1–80 characters without control characters.",
        ));
    }
    Ok(name.into())
}
