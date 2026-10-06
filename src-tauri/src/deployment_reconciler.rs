use crate::{
    app_core::AppCore, deployment_service::owns, error::Result, runpod_client::Pod, types::*,
};

impl AppCore {
    pub async fn reconcile_all(&self) -> Result<()> {
        let _guard = self.operations.lock().await;
        self.reconcile_locked().await
    }
    pub async fn background_tick(&self) -> Result<()> {
        let Ok(_guard) = self.operations.try_lock() else {
            return Ok(());
        };
        self.reconcile_locked().await
    }
    async fn reconcile_locked(&self) -> Result<()> {
        let state = self.store.read()?;
        if state.deployments.is_empty() {
            return Ok(());
        }
        let pods = match self.provider.list().await {
            Ok(pods) => pods,
            Err(error) => {
                for d in state.deployments.iter().filter(|d| !d.stage.is_terminal()) {
                    self.disconnect(&d.id)?;
                    self.stage(
                        &d.id,
                        if d.intended_action == "terminate" {
                            DeploymentStage::CleanupPending
                        } else {
                            DeploymentStage::ReconciliationRequired
                        },
                        "Runpod is unreachable. Resource state and ongoing charges are uncertain.",
                    )?;
                }
                return Err(error);
            }
        };
        let mut first_error = None;
        for d in state.deployments {
            if let Err(error) = self
                .reconcile_one(&d.id, &state.installation_id, &pods)
                .await
            {
                first_error.get_or_insert(error);
            }
        }
        self.changed();
        first_error.map_or(Ok(()), Err)
    }
    async fn reconcile_one(&self, id: &str, owner: &str, pods: &[Pod]) -> Result<()> {
        let mut d = self.deployment(id)?;
        let matches: Vec<_> = pods.iter().filter(|p| owns(&d, owner, p)).collect();
        // Also detect late creations after an explicitly resolved ambiguous request.
        if d.stage.is_terminal() {
            if matches.is_empty() {
                return Ok(());
            }
            self.store.update(|s| {
                let item = s.deployments.iter_mut().find(|d| d.id == id).unwrap();
                item.pod_ids = matches.iter().map(|p| p.id.clone()).collect();
                item.terminated_at = None;
                item.intended_action = "terminate".into();
                item.stage = DeploymentStage::CleanupPending;
                Ok(())
            })?;
            return self.finish_locked(id).await;
        }
        if !matches.is_empty() {
            for pod in &matches {
                if !d.pod_ids.contains(&pod.id) {
                    d.pod_ids.push(pod.id.clone());
                }
            }
            self.store.update(|s| {
                s.deployments
                    .iter_mut()
                    .find(|d| d.id == id)
                    .unwrap()
                    .pod_ids = d.pod_ids.clone();
                Ok(())
            })?;
        }
        if d.intended_action == "terminate"
            || d.stage.is_cleaning()
            || now() >= d.deadline_at
            || d.cleanup_due_at.is_some_and(|deadline| now() >= deadline)
        {
            return self.finish_locked(id).await;
        }
        if d.pod_ids.len() != 1 {
            self.disconnect(id)?;
            self.stage(id, DeploymentStage::ReconciliationRequired, if d.pod_ids.is_empty() {
                "No owned Pod is visible yet. Creation is unresolved; no automatic retry will occur."
            } else { "Multiple owned Pods were found. Use Finish to clean up all of them before provisioning again." })?;
            return Ok(());
        }
        let pod = match self.provider.get(&d.pod_ids[0]).await {
            Ok(Some(pod)) => pod,
            Ok(None) => {
                self.stop_deployment_sessions(id)?;
                self.confirm_terminated(id, "Runpod confirmed that the Pod no longer exists.")?;
                return self.wait_deployment_sessions(id).await;
            }
            Err(error) => {
                self.disconnect(id)?;
                return Err(error);
            }
        };
        if !owns(&d, owner, &pod) {
            self.disconnect(id)?;
            return self.stage(
                id,
                DeploymentStage::ReconciliationRequired,
                "Pod ownership no longer matches the saved record. Inspect the Runpod console.",
            );
        }
        if !self.accept_price_locked(id, pod.cost_per_hr).await? {
            return Ok(());
        }
        if now() >= self.deployment(id)?.deadline_at {
            return self.finish_locked(id).await;
        }
        if pod.desired_status != "RUNNING" {
            self.disconnect(id)?;
            self.stage(
                id,
                DeploymentStage::Failed,
                "The Pod stopped unexpectedly. Cleaning up owned compute.",
            )?;
            return self.finish_locked(id).await;
        }
        let startup_limit = d.profile.runtime.download_timeout_seconds
            + d.profile.runtime.load_timeout_seconds
            + 300;
        if d.startup_completed == Some(false) && now().saturating_sub(d.created_at) >= startup_limit
        {
            self.disconnect(id)?;
            self.stage(
                id,
                DeploymentStage::Failed,
                "Startup exceeded the bounded wait. Cleaning up owned compute.",
            )?;
            return self.finish_locked(id).await;
        }
        match self.runtime.status(&d).await {
            Ok(status) => {
                match status.stage.as_str() {
                    "failed" => {
                        self.disconnect(id)?;
                        self.stage(id, DeploymentStage::Failed, startup_failure(&status.code))?;
                        self.finish_locked(id).await?;
                    }
                    "downloading" => {
                        self.disconnect(id)?;
                        self.stage(id, DeploymentStage::Downloading, "Checking protected-file access and downloading missing revision files.")?;
                    }
                    "loading" => {
                        self.disconnect(id)?;
                        self.stage(
                            id,
                            DeploymentStage::Loading,
                            "Loading weights and preparing the inference runtime.",
                        )?;
                    }
                    "verifying" => {
                        if d.verified_at
                            .is_none_or(|at| now().saturating_sub(at) >= 60)
                        {
                            self.stage(id, DeploymentStage::Verifying, "Checking authentication, model identity, and a real inference response.")?;
                            match self.runtime.verify(&d).await {
                                Ok(()) => {
                                    if now() >= self.deployment(id)?.deadline_at {
                                        self.finish_locked(id).await?;
                                    } else {
                                        self.mark_ready(id)?;
                                    }
                                }
                                Err(error) => {
                                    self.disconnect(id)?;
                                    if matches!(
                                        error.code.as_str(),
                                        "wrong_model"
                                            | "endpoint_not_protected"
                                            | "authentication"
                                            | "inference_credential_missing"
                                            | "inference_verification"
                                    ) {
                                        self.stage(id, DeploymentStage::Failed, &error.message)?;
                                        self.finish_locked(id).await?;
                                    } else {
                                        self.stage(
                                            id,
                                            DeploymentStage::ReconciliationRequired,
                                            &error.message,
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        self.disconnect(id)?;
                        self.stage(
                            id,
                            DeploymentStage::ContainerStarting,
                            "Waiting for the runtime supervisor.",
                        )?;
                    }
                }
            }
            Err(_) => {
                self.disconnect(id)?;
                self.stage(id, if d.startup_completed == Some(false) { DeploymentStage::ContainerStarting } else { DeploymentStage::ReconciliationRequired }, "Authenticated runtime status is unavailable. Rechecking the connection; the Pod may still be billing.")?;
            }
        }
        Ok(())
    }
    pub(crate) fn mark_ready(&self, id: &str) -> Result<()> {
        self.store.update(|s| {
            let d = s.deployments.iter_mut().find(|d| d.id == id).unwrap();
            d.stage = DeploymentStage::Ready;
            d.verified_at = Some(now());
            d.startup_completed = Some(true);
            d.updated_at = now();
            d.message =
                "Authenticated inference is ready. Enable selects it for new sessions.".into();
            for session in s.sessions.iter_mut().filter(|session| {
                session.deployment_id == id && session.status == SessionStatus::Disconnected
            }) {
                if self.terminals.is_running(&session.id) {
                    session.status = SessionStatus::Running;
                }
            }
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
}

fn startup_failure(code: &str) -> &'static str {
    match code {
        "token_permissions" => "Hugging Face rejected the token. Check its read permissions.",
        "model_access_denied" => {
            "Model access was denied. Check approval on the model page and token permissions."
        }
        "model_not_found" => {
            "The model repository or pinned revision could not be found or accessed."
        }
        "gpu_out_of_memory" => {
            "The model exceeded available GPU memory. Review context, concurrency, and hardware."
        }
        "download_timeout" => "Model download exceeded its time limit.",
        "load_timeout" => "Model loading exceeded its time limit.",
        "runtime_configuration" => "The serving runtime rejected a profile parameter.",
        "weights_missing" => "No supported weight files were found at the pinned revision.",
        "insufficient_cache_space" => "The model cache has insufficient free space. Choose a larger volume or free unused cache space before provisioning again.",
        "cache_space_unavailable" => "The model cache could not be checked for available space. Review its mount and permissions.",
        _ => "Runtime startup failed. Inspect the model profile and sanitized diagnostics.",
    }
}
