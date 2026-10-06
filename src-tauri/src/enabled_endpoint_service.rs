use crate::{
    app_core::AppCore,
    deployment_service::owns,
    error::{AppError, Result},
    types::*,
};

impl AppCore {
    pub async fn enable(&self, id: &str) -> Result<()> {
        let _guard = self.operations.lock().await;
        self.ensure_ready_locked(id).await?;
        self.store.update(|state| {
            state.enabled_deployment_id = Some(id.to_owned());
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
    pub(crate) async fn ensure_ready_locked(&self, id: &str) -> Result<Deployment> {
        let deployment = self.deployment(id)?;
        if deployment.stage != DeploymentStage::Ready
            || deployment.intended_action == "terminate"
            || now() >= deployment.deadline_at
        {
            return Err(AppError::new(
                "endpoint_not_ready",
                "This deployment is not ready for a connected session.",
                "Reconcile the Pod and wait for verified readiness, then Enable it.",
            ));
        }
        let owner = self.store.read()?.installation_id;
        let pod_id = deployment
            .pod_ids
            .first()
            .ok_or_else(|| AppError::invalid("Deployment has no Pod."))?;
        match self.provider.get(pod_id).await {
            Ok(Some(pod)) if owns(&deployment, &owner, &pod) && pod.desired_status == "RUNNING" => {
            }
            Ok(None) => {
                self.stop_deployment_sessions(id)?;
                self.confirm_terminated(id, "Runpod confirmed the enabled Pod is absent.")?;
                self.wait_deployment_sessions(id).await?;
                return Err(AppError::invalid("The Pod no longer exists."));
            }
            _ => {
                self.disconnect(id)?;
                return Err(AppError::new("endpoint_unavailable", "The enabled Pod could not be verified.", "Reconcile it before opening another session. No subscription or other-model fallback is used."));
            }
        }
        if let Err(error) = self.runtime.verify(&deployment).await {
            self.disconnect(id)?;
            return Err(error);
        }
        if now() >= deployment.deadline_at {
            self.finish_locked(id).await?;
            return Err(AppError::invalid(
                "The deployment deadline passed during verification.",
            ));
        }
        self.mark_ready(id)?;
        self.deployment(id)
    }
}
