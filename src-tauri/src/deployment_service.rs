use crate::{
    app_core::{estimated_cost, AppCore},
    credential_store::inference_credential,
    error::{AppError, Result},
    model_catalog::{identifier, ModelAccess},
    runpod_client::{Hardware, Pod},
    types::*,
};
use serde_json::{json, Value};

impl AppCore {
    pub async fn provision(&self, request: ProvisionRequest) -> Result<String> {
        let _guard = self.operations.lock().await;
        let state = self.store.read()?;
        let policy = &state.settings.policy;
        validate_policy(policy)?;
        if !policy.paid_provisioning_enabled || !request.acknowledge_paid_creation {
            return Err(AppError::new("paid_actions_disabled", "Paid provisioning is disabled.", "When you are ready to test, enable paid provisioning in Setup and review the deployment confirmation."));
        }
        if !policy.acknowledge_offline_risk {
            return Err(AppError::invalid(
                "Acknowledge the offline cleanup limitation before paid provisioning.",
            ));
        }
        if state.deployments.iter().any(|d| !d.stage.is_terminal()) {
            return Err(AppError::invalid(
                "Finish or reconcile the existing deployment before provisioning another Pod.",
            ));
        }
        let profile = self
            .catalog()?
            .into_iter()
            .find(|p| p.model.id == request.profile_id)
            .ok_or_else(|| AppError::invalid("Select a catalog model."))?;
        if !profile.model.gpu_types.contains(&request.gpu_type) {
            return Err(AppError::invalid(
                "This GPU is not included in the selected profile.",
            ));
        }
        if request.data_center_id.is_empty() {
            return Err(AppError::invalid("Choose a Runpod data center."));
        }
        if profile.model.access != ModelAccess::Public
            && state.settings.hugging_face_secret.is_none()
        {
            return Err(AppError::new("hugging_face_secret", "This model requires a Runpod Hugging Face secret reference.", "Accept any model access conditions and add the name of a read-scoped Runpod secret in Setup."));
        }
        let hardware = self.provider.hardware().await?;
        let hourly = quote(
            &hardware,
            &request,
            profile.model.gpu_count,
            profile.model.min_gpu_memory_gb,
            profile.model.min_cache_gb,
        )?;
        if hourly > policy.max_hourly_usd {
            return Err(AppError::invalid(
                "The current GPU quote exceeds your hourly limit.",
            ));
        }
        let budget = policy
            .total_budget_usd
            .ok_or_else(|| AppError::invalid("Set a total experiment budget in Setup."))?;
        let spent: f64 = state
            .deployments
            .iter()
            .map(|d| estimated_cost(d, now()))
            .sum();
        let remaining = budget - spent;
        if remaining < hourly / 60.0 {
            return Err(AppError::invalid(
                "The remaining budget is less than one minute at the current GPU rate.",
            ));
        }
        let lifetime = policy
            .max_lifetime_minutes
            .ok_or_else(|| AppError::invalid("Set a maximum Pod lifetime in Setup."))?
            * 60;
        let id = uuid::Uuid::new_v4().to_string();
        let created = now();
        let credential_ref = format!("inference-{id}");
        let token = inference_credential();
        self.credentials.set(&credential_ref, &token)?;
        let deployment = Deployment {
            id: id.clone(),
            pod_name: format!("ai-deck-{}-{id}", &state.installation_id[..8]),
            pod_ids: vec![],
            profile,
            stage: DeploymentStage::Validating,
            intended_action: "create".into(),
            created_at: created,
            updated_at: created,
            terminated_at: None,
            deadline_at: created.saturating_add(lifetime.min((remaining / hourly * 3600.0) as u32)),
            verified_at: None,
            startup_completed: Some(false),
            hourly_usd: hourly,
            budget_usd: remaining,
            gpu_type: request.gpu_type,
            gpu_count: 0,
            data_center_id: request.data_center_id,
            network_volume_id: request.network_volume_id,
            retained_storage_monthly_usd: None,
            credential_ref,
            failure_reason: None,
            message: "Validated; preparing the owned deployment record.".into(),
            cleanup_due_at: None,
        };
        let mut deployment = deployment;
        deployment.gpu_count = deployment.profile.model.gpu_count;
        let payload = create_payload(
            &deployment,
            &state.installation_id,
            &token,
            state.settings.hugging_face_secret.as_deref(),
        )?;
        if let Err(error) = self.store.update(|s| {
            s.deployments.push(deployment.clone());
            Ok(())
        }) {
            let _ = self.credentials.delete(&deployment.credential_ref);
            return Err(error);
        }
        self.stage(
            &id,
            DeploymentStage::Provisioning,
            "Creation requested. GPU billing can begin before Enable.",
        )?;
        match self.provider.create(payload).await {
            Ok(pod) => {
                self.store.update(|s| {
                    let d = s.deployments.iter_mut().find(|d| d.id == id).unwrap();
                    d.pod_ids = vec![pod.id.clone()];
                    Ok(())
                })?;
                if !owns(&deployment, &state.installation_id, &pod) {
                    self.stage(
                        &id,
                        DeploymentStage::ReconciliationRequired,
                        "The create response needs ownership verification. No retry will be sent.",
                    )?;
                } else if self.accept_price_locked(&id, pod.cost_per_hr).await? {
                    self.stage(
                        &id,
                        DeploymentStage::ContainerStarting,
                        "Pod created. Waiting for authenticated startup status.",
                    )?;
                }
            }
            Err(error) => {
                self.stage(
                    &id,
                    DeploymentStage::ReconciliationRequired,
                    "Creation was not confirmed. Reconcile before taking any further action.",
                )?;
                return Err(AppError::new("ambiguous_create", error.message, "Use Reconcile or inspect the Runpod console. This app will not automatically create a second Pod."));
            }
        }
        self.changed();
        Ok(id)
    }

    pub(crate) async fn accept_price_locked(&self, id: &str, hourly: f64) -> Result<bool> {
        let policy = self.store.read()?.settings.policy;
        if hourly.is_finite() && hourly > 0.0 {
            self.store.update(|s| {
                let d = s.deployments.iter_mut().find(|d| d.id == id).unwrap();
                d.hourly_usd = hourly;
                d.deadline_at = d.deadline_at.min(
                    d.created_at
                        .saturating_add((d.budget_usd / hourly * 3600.0) as u32),
                );
                Ok(())
            })?;
        }
        if !hourly.is_finite()
            || hourly <= 0.0
            || hourly > policy.max_hourly_usd.min(HOURLY_CEILING_USD)
        {
            self.stage(
                id,
                DeploymentStage::Failed,
                "Actual GPU price was missing or exceeded the hourly limit. Cleanup requested.",
            )?;
            self.finish_locked(id).await?;
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn finish(&self, id: &str) -> Result<()> {
        let _guard = self.operations.lock().await;
        self.finish_locked(id).await
    }
    pub(crate) async fn finish_locked(&self, id: &str) -> Result<()> {
        let mut deployment = self.deployment(id)?;
        if deployment.stage.is_terminal() {
            self.stop_deployment_sessions(id)?;
            return self.wait_deployment_sessions(id).await;
        }
        self.store.update(|state| {
            if state.enabled_deployment_id.as_deref() == Some(id) {
                state.enabled_deployment_id = None;
            }
            let d = state.deployments.iter_mut().find(|d| d.id == id).unwrap();
            d.intended_action = "terminate".into();
            d.verified_at = None;
            Ok(())
        })?;
        self.stage(
            id,
            DeploymentStage::Draining,
            "Interrupting sessions before terminating their shared Pod.",
        )?;
        self.stop_deployment_sessions(id)?;
        let owner = self.store.read()?.installation_id;
        if deployment.pod_ids.is_empty() {
            let pods = match self.provider.list().await {
                Ok(pods) => pods,
                Err(e) => {
                    self.stage(
                        id,
                        DeploymentStage::CleanupPending,
                        "Could not check for an unconfirmed Pod. Check the Runpod console.",
                    )?;
                    return Err(e);
                }
            };
            deployment.pod_ids = pods
                .into_iter()
                .filter(|p| owns(&deployment, &owner, p))
                .map(|p| p.id)
                .collect();
            if deployment.pod_ids.is_empty() {
                self.stage(id, DeploymentStage::ReconciliationRequired, "No matching Pod is visible, but the original creation was not confirmed. Confirm absence in the console before clearing this record.")?;
                return Ok(());
            }
            self.store.update(|s| {
                s.deployments
                    .iter_mut()
                    .find(|d| d.id == id)
                    .unwrap()
                    .pod_ids = deployment.pod_ids.clone();
                Ok(())
            })?;
        }
        self.stage(
            id,
            DeploymentStage::Terminating,
            "Requesting deletion of owned Pods. Retained network storage will be preserved.",
        )?;
        let mut pending = false;
        let mut first_error = None;
        for pod_id in &deployment.pod_ids {
            match self.provider.get(pod_id).await {
                Ok(None) => continue,
                Ok(Some(pod)) if owns(&deployment, &owner, &pod) => {
                    let _ = self.provider.delete(pod_id).await;
                }
                Ok(Some(_)) => {
                    pending = true;
                    first_error.get_or_insert(AppError::new("ownership_mismatch", "Cannot establish ownership of a tracked Pod.", "Inspect it in the Runpod console. The app will not delete unrelated resources."));
                    continue;
                }
                Err(error) => {
                    pending = true;
                    first_error.get_or_insert(error);
                    continue;
                }
            }
            if !matches!(self.provider.get(pod_id).await, Ok(None)) {
                pending = true;
            }
        }
        if pending {
            self.stage(id, DeploymentStage::CleanupPending, "Deletion of every owned Pod is not confirmed. Billing may continue; reconcile or inspect ownership in the Runpod console.")?;
            return first_error.map_or(Ok(()), Err);
        }
        self.confirm_terminated(id, "Runpod confirmed that every tracked Pod is absent.")?;
        // Confirm cloud deletion even if local process cleanup needs a retry.
        // A successful Finish additionally waits for local tools to stop.
        self.wait_deployment_sessions(id).await
    }
    pub(crate) fn confirm_terminated(&self, id: &str, message: &str) -> Result<()> {
        self.store.update(|s| {
            let d = s
                .deployments
                .iter_mut()
                .find(|d| d.id == id)
                .ok_or_else(|| AppError::invalid("Deployment not found."))?;
            d.stage = DeploymentStage::Terminated;
            d.terminated_at = Some(now());
            d.updated_at = now();
            d.verified_at = None;
            d.message = message.into();
            d.cleanup_due_at = None;
            if s.enabled_deployment_id.as_deref() == Some(id) {
                s.enabled_deployment_id = None;
            }
            for session in s.sessions.iter_mut().filter(|session| {
                session.deployment_id == id && session.status == SessionStatus::Running
            }) {
                session.status = SessionStatus::Disconnected;
            }
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
    pub async fn confirm_absent_creation(&self, id: &str, typed_name: &str) -> Result<()> {
        let _guard = self.operations.lock().await;
        let d = self.deployment(id)?;
        if !d.pod_ids.is_empty()
            || d.pod_name != typed_name
            || now().saturating_sub(d.created_at) < 120
        {
            return Err(AppError::invalid("Wait at least two minutes, verify the Runpod console, and enter the exact Pod name to resolve an unconfirmed creation."));
        }
        let owner = self.store.read()?.installation_id;
        if self
            .provider
            .list()
            .await?
            .iter()
            .any(|p| owns(&d, &owner, p))
        {
            return Err(AppError::invalid(
                "A matching Pod exists. Reconcile and Finish it.",
            ));
        }
        self.confirm_terminated(id, "Fresh Runpod listing found no owned Pod; the user confirmed its absence in the console.")
    }
}

pub fn validate_policy(p: &Policy) -> Result<()> {
    if !p.max_hourly_usd.is_finite()
        || p.max_hourly_usd <= 0.0
        || p.max_hourly_usd > HOURLY_CEILING_USD
        || p.total_budget_usd
            .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > 10000.0)
        || p.max_lifetime_minutes
            .is_some_and(|v| !(1..=1440).contains(&v))
        || p.auto_terminate_delay_seconds
            .is_some_and(|v| !(30..=3600).contains(&v))
    {
        return Err(AppError::invalid("Use a positive hourly limit at or below $10, a positive total budget, and a lifetime of 1–1440 minutes. Idle cleanup delays must be 30–3600 seconds."));
    }
    if p.paid_provisioning_enabled
        && (p.total_budget_usd.is_none()
            || p.max_lifetime_minutes.is_none()
            || !p.acknowledge_offline_risk)
    {
        return Err(AppError::invalid("Paid provisioning requires a total budget, maximum lifetime, and acknowledgement of the offline cleanup limitation."));
    }
    Ok(())
}
// Ownership keys are a persistent protocol shared with already-running Pods.
pub fn owns(d: &Deployment, owner: &str, pod: &Pod) -> bool {
    pod.name == d.pod_name
        && pod.image_name == d.profile.runtime.image
        && pod.env.get("RUNPOD_DECK_OWNER").map(String::as_str) == Some(owner)
        && pod.env.get("RUNPOD_DECK_DEPLOYMENT").map(String::as_str) == Some(d.id.as_str())
        && pod.network_volume_id.as_deref().filter(|s| !s.is_empty())
            == d.network_volume_id.as_deref()
}
fn quote(
    hardware: &Hardware,
    request: &ProvisionRequest,
    count: u32,
    memory: u32,
    min_cache_gb: u32,
) -> Result<f64> {
    let gpu = hardware
        .gpus
        .iter()
        .find(|g| g.id == request.gpu_type && g.secure_cloud && g.memory_in_gb >= memory)
        .ok_or_else(|| {
            AppError::invalid(
                "The selected GPU is not currently offered in Secure Cloud with enough memory.",
            )
        })?;
    let dc = hardware
        .data_centers
        .iter()
        .find(|d| d.id == request.data_center_id)
        .ok_or_else(|| AppError::invalid("Data center not found."))?;
    if !dc.gpu_availability.iter().any(|g| {
        g.gpu_type_id == gpu.id
            && g.stock_status
                .as_ref()
                .is_some_and(|s| ["high", "medium", "low"].contains(&s.to_lowercase().as_str()))
    }) {
        return Err(AppError::new(
            "gpu_unavailable",
            "The selected GPU has no confirmed stock in this location.",
            "Refresh availability and select another location. No Pod was created.",
        ));
    }
    if let Some(id) = &request.network_volume_id {
        let volume = hardware
            .network_volumes
            .iter()
            .find(|v| v.id == *id && v.data_center_id == dc.id)
            .ok_or_else(|| {
                AppError::invalid("Select a retained network volume in the same data center.")
            })?;
        if volume.size < min_cache_gb {
            return Err(AppError::new("insufficient_cache_space", format!("This model requires at least {min_cache_gb} GB of cache capacity; the selected volume has {} GB.", volume.size), "Choose a larger retained volume or fully temporary storage. No Pod was created."));
        }
    }
    let price = gpu.secure_price.unwrap_or(0.0) * f64::from(count);
    if !price.is_finite() || price <= 0.0 {
        return Err(AppError::invalid(
            "Runpod did not return a usable price quote.",
        ));
    }
    Ok(price)
}
pub fn create_payload(
    d: &Deployment,
    owner: &str,
    token: &str,
    hf_secret: Option<&str>,
) -> Result<Value> {
    let p = &d.profile.model;
    let r = &d.profile.runtime;
    let config = json!({"repository":p.repository,"revision":p.revision,"inferencePort":r.inference_port,"statusPort":r.status_port,
        "downloadTimeoutSeconds":r.download_timeout_seconds,"loadTimeoutSeconds":r.load_timeout_seconds,"minCacheGb":p.min_cache_gb,"arguments":d.profile.arguments()});
    let mut env = json!({"VLLM_API_KEY":token,"RUNPOD_DECK_OWNER":owner,"RUNPOD_DECK_DEPLOYMENT":d.id,
        "RUNPOD_DECK_RUNTIME":config.to_string(),"HF_HOME":"/tmp/ai-deck-hf","HF_HUB_CACHE":p.model_cache_path,
        "HF_HUB_DISABLE_PROGRESS_BARS":"1","HF_HUB_DISABLE_TELEMETRY":"1","VLLM_NO_USAGE_STATS":"1",
        "VLLM_CACHE_ROOT":format!("{}/{}",p.compile_cache_path,d.profile.fingerprint),"PYTHONUNBUFFERED":"1"});
    if p.access != ModelAccess::Public {
        let name = hf_secret
            .filter(|s| identifier(s))
            .ok_or_else(|| AppError::invalid("Set a valid Hugging Face Runpod secret name."))?;
        env["HF_TOKEN"] = json!(format!("{{{{ RUNPOD_SECRET_{name} }}}}"));
    }
    let mut payload = json!({"name":d.pod_name,"imageName":r.image,"cloudType":"SECURE","computeType":"GPU",
        "gpuTypeIds":[d.gpu_type],"gpuCount":d.gpu_count,"dataCenterIds":[d.data_center_id],"interruptible":false,
        "containerDiskInGb":p.disk_gb,"volumeInGb":0,"volumeMountPath":"/workspace",
        "minRAMPerGPU":p.min_cpu_ram_gb.div_ceil(d.gpu_count),"minVCPUPerGPU":4,
        "ports":[format!("{}/http",r.inference_port),format!("{}/http",r.status_port)],
        "dockerEntrypoint":["python3","-u","-c"],"dockerStartCmd":[include_str!("../../containers/vllm-runtime/launcher.py")],"env":env});
    if let Some(id) = &d.network_volume_id {
        payload["networkVolumeId"] = json!(id);
    }
    Ok(payload)
}
