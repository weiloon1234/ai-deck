use crate::{
    credential_store::CredentialStore,
    error::{AppError, Result},
    model_catalog::{self, ResolvedProfile},
    runpod_client::RunpodApi,
    runtime_client::RuntimeApi,
    state_store::StateStore,
    terminal_process::TerminalManager,
    types::*,
};
use std::sync::Arc;

pub type Notify = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;
pub struct AppCore {
    pub store: Arc<StateStore>,
    pub credentials: Arc<dyn CredentialStore>,
    pub provider: Arc<dyn RunpodApi>,
    pub runtime: Arc<dyn RuntimeApi>,
    pub terminals: Arc<TerminalManager>,
    pub operations: tokio::sync::Mutex<()>,
    pub model_imports: crate::model_import_service::ModelImports,
    pub notify: Notify,
}
impl AppCore {
    pub fn catalog(&self) -> Result<Vec<ResolvedProfile>> {
        let path = self.store.directory().join("catalog.json");
        let mut profiles = if path.exists() {
            let json = std::fs::read_to_string(path).map_err(|_| AppError::storage())?;
            model_catalog::parse_catalog(&json)
        } else {
            model_catalog::builtin_catalog()
        }?;
        for record in self.store.read()?.imported_models {
            if let Some(profile) = record.profile {
                let document = model_catalog::CatalogDocument {
                    schema_version: 1,
                    runtimes: vec![profile.runtime],
                    profiles: vec![profile.model],
                };
                let json = serde_json::to_string(&document).map_err(|_| AppError::storage())?;
                for imported in model_catalog::parse_catalog(&json)? {
                    if profiles.iter().any(|p| p.model.id == imported.model.id) {
                        return Err(AppError::invalid("Catalog and imported model IDs conflict. Remove the duplicate profile from the catalog."));
                    }
                    profiles.push(imported);
                }
            }
        }
        Ok(profiles)
    }
    pub fn changed(&self) {
        (self.notify)("state-changed", serde_json::json!({}));
    }
    pub fn snapshot(&self) -> Result<AppSnapshot> {
        let state = self.store.read()?;
        let (catalog, catalog_error) = match self.catalog() {
            Ok(catalog) => (catalog, None),
            Err(mut error) => {
                if error.code == "invalid_input" {
                    error.recovery = "Open Models and import a valid catalog. Existing deployments retain their saved profiles.".into();
                }
                (vec![], Some(error))
            }
        };
        let (runpod_key_configured, credential_store_error) =
            match self.credentials.get(crate::credential_store::RUNPOD_KEY) {
                Ok(key) => (Some(key.is_some()), None),
                Err(error) => (None, Some(error)),
            };
        Ok(AppSnapshot {
            state,
            catalog,
            runpod_key_configured,
            catalog_error,
            credential_store_error,
            hourly_ceiling_usd: HOURLY_CEILING_USD,
            offline_expiry_verified: false,
            app_version: env!("CARGO_PKG_VERSION").into(),
        })
    }
    pub fn deployment(&self, id: &str) -> Result<Deployment> {
        self.store
            .read()?
            .deployments
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| AppError::invalid("Deployment not found."))
    }
    pub fn stage(&self, id: &str, stage: DeploymentStage, message: &str) -> Result<()> {
        self.store.update(|state| {
            let deployment = state
                .deployments
                .iter_mut()
                .find(|d| d.id == id)
                .ok_or_else(|| AppError::invalid("Deployment not found."))?;
            if stage == DeploymentStage::Failed {
                deployment.failure_reason = Some(message.into());
            }
            deployment.stage = stage;
            deployment.updated_at = now();
            deployment.message = message.into();
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
    pub fn disconnect(&self, id: &str) -> Result<()> {
        self.store.update(|state| {
            if let Some(d) = state.deployments.iter_mut().find(|d| d.id == id) {
                d.verified_at = None;
            }
            for session in state
                .sessions
                .iter_mut()
                .filter(|s| s.deployment_id == id && s.status == SessionStatus::Running)
            {
                session.status = SessionStatus::Disconnected;
            }
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
}

pub fn estimated_cost(deployment: &Deployment, at: u32) -> f64 {
    f64::from(
        deployment
            .terminated_at
            .unwrap_or(at)
            .saturating_sub(deployment.created_at),
    ) / 3600.0
        * deployment.hourly_usd
}
