use crate::{
    app_core::AppCore,
    codex_model_analyzer::{check_cancelled, AnalysisInput, CodexModelAnalyzer, ModelAnalyzer},
    error::{AppError, Result},
    hugging_face_client::{HuggingFaceApi, HuggingFaceClient, ModelSource},
    model_analysis::{ExperimentalModelConfiguration, ImportedModel},
    model_catalog::builtin_catalog,
    types::{now, HOURLY_CEILING_USD},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use ts_rs::TS;

#[derive(Default)]
pub struct ModelImports {
    lock: tokio::sync::Mutex<()>,
    cancelled: Arc<AtomicBool>,
}
impl ModelImports {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub async fn shutdown(&self) -> Result<()> {
        self.cancel();
        let _guard = tokio::time::timeout(Duration::from_secs(20), self.lock.lock())
            .await
            .map_err(|_| {
                AppError::new(
                    "model_analysis_stopping",
                    "Model analysis is still stopping.",
                    "Wait briefly and retry quitting.",
                )
            })?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportResult {
    pub id: String,
    pub reused: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigureImportedModelRequest {
    pub id: String,
    pub expected_profile_version: u32,
    pub configuration: ExperimentalModelConfiguration,
}

async fn cancellable<T>(
    operation: impl Future<Output = Result<T>>,
    cancelled: &AtomicBool,
) -> Result<T> {
    tokio::select! {
        result = operation => result,
        _ = async { loop { if cancelled.load(Ordering::Acquire) { break; } tokio::time::sleep(Duration::from_millis(100)).await; } } => { check_cancelled(cancelled)?; unreachable!() }
    }
}
impl AppCore {
    fn import_progress(&self, message: &str) {
        (self.notify)(
            "model-import-progress",
            serde_json::json!({"message":message}),
        );
    }
    pub async fn import_hugging_face_model(
        &self,
        url: &str,
        reanalyze: bool,
    ) -> Result<ModelImportResult> {
        self.import_model_using(url, reanalyze, &HuggingFaceClient, &CodexModelAnalyzer)
            .await
    }
    pub async fn import_model_using(
        &self,
        url: &str,
        reanalyze: bool,
        hub: &dyn HuggingFaceApi,
        analyzer: &dyn ModelAnalyzer,
    ) -> Result<ModelImportResult> {
        let source = ModelSource::parse(url)?;
        let _guard = self
            .model_imports
            .lock
            .try_lock()
            .map_err(|_| import_busy())?;
        self.model_imports.cancelled.store(false, Ordering::Release);
        let state = self.store.read()?;
        let existing = state
            .imported_models
            .iter()
            .find(|m| m.source_url == source.url());
        if let Some(existing) = existing.filter(|_| !reanalyze) {
            return Ok(ModelImportResult {
                id: existing.id.clone(),
                reused: true,
            });
        }
        let provider_id = state.settings.provider;
        let provider = self.providers.get(provider_id)?;
        if existing.is_none() && state.imported_models.len() >= 100 {
            return Err(AppError::invalid("The local library holds up to 100 imported models. Remove an unused suggestion first."));
        }
        self.import_progress("Reading Hugging Face model details…");
        let metadata = cancellable(hub.model(&source), &self.model_imports.cancelled).await?;
        self.import_progress("Checking Runpod hardware and current GPU prices…");
        let hardware = cancellable(provider.hardware(), &self.model_imports.cancelled).await?;
        let prices_checked_at = now();
        let runtime = builtin_catalog()?
            .first()
            .ok_or_else(|| AppError::invalid("No built-in runtime is available."))?
            .runtime
            .clone();
        let hourly_limit = state.settings.policy.max_hourly_usd.min(HOURLY_CEILING_USD);
        self.import_progress("Codex is estimating hardware and a usable context range…");
        let (analysis, analyzer_version) = analyzer
            .analyze(
                AnalysisInput {
                    metadata: metadata.clone(),
                    hardware: hardware.clone(),
                    runtime: runtime.clone(),
                    hourly_limit,
                    codex_path: state.settings.codex_path,
                    directory: self.store.directory().to_owned(),
                },
                self.model_imports.cancelled.clone(),
            )
            .await?;
        analysis.validate(&hardware)?;
        check_cancelled(&self.model_imports.cancelled)?;
        let mut record = ImportedModel {
            provider: provider_id,
            id: format!("hf-{:x}", Sha256::digest(source.url().as_bytes())),
            source_url: source.url(),
            quotes: vec![],
            metadata,
            analysis,
            analyzer_version,
            analyzed_at: now(),
            prices_checked_at,
            runtime,
            profile_version: existing.map_or(1, |m| m.profile_version.saturating_add(1)),
            profile: None,
            experimental_configuration: existing.and_then(|m| m.experimental_configuration.clone()),
        };
        record.quotes = record.current_quotes(&hardware);
        self.import_progress("Saving model and suggestions…");
        self.store.update(|s| {
            check_cancelled(&self.model_imports.cancelled)?;
            record.profile =
                record.build_profile(s.settings.policy.max_hourly_usd.min(HOURLY_CEILING_USD))?;
            s.imported_models.retain(|m| m.id != record.id);
            s.imported_models.push(record.clone());
            Ok(())
        })?;
        self.changed();
        Ok(ModelImportResult {
            id: record.id,
            reused: false,
        })
    }
    pub async fn refresh_model_prices(&self, id: &str) -> Result<()> {
        let _guard = self
            .model_imports
            .lock
            .try_lock()
            .map_err(|_| import_busy())?;
        self.model_imports.cancelled.store(false, Ordering::Release);
        let provider_id = self
            .store
            .read()?
            .imported_models
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.provider)
            .ok_or_else(|| AppError::invalid("Saved model not found."))?;
        self.import_progress("Refreshing Runpod prices without another Codex analysis…");
        let hardware = cancellable(
            self.providers.get(provider_id)?.hardware(),
            &self.model_imports.cancelled,
        )
        .await?;
        check_cancelled(&self.model_imports.cancelled)?;
        self.store.update(|s| {
            let record = s
                .imported_models
                .iter_mut()
                .find(|m| m.id == id)
                .ok_or_else(|| AppError::invalid("Saved model not found."))?;
            record.quotes = record.current_quotes(&hardware);
            record.prices_checked_at = now();
            record.profile_version = record.profile_version.saturating_add(1);
            record.profile =
                record.build_profile(s.settings.policy.max_hourly_usd.min(HOURLY_CEILING_USD))?;
            Ok(())
        })?;
        self.changed();
        Ok(())
    }

    pub async fn configure_imported_model(
        &self,
        request: ConfigureImportedModelRequest,
    ) -> Result<()> {
        let _guard = self
            .model_imports
            .lock
            .try_lock()
            .map_err(|_| import_busy())?;
        request.configuration.validate()?;
        self.model_imports.cancelled.store(false, Ordering::Release);
        let record = self
            .store
            .read()?
            .imported_models
            .into_iter()
            .find(|m| m.id == request.id)
            .ok_or_else(|| AppError::invalid("Saved model not found."))?;
        if record.profile_version != request.expected_profile_version {
            return Err(configuration_changed());
        }
        record.provider.ensure_provisioning()?;
        // Read the saved provider's actual GPU prices; no resource is created.
        let hardware = cancellable(
            self.imported_model_hardware(&request.id),
            &self.model_imports.cancelled,
        )
        .await?;
        self.store.update(|s| {
            check_cancelled(&self.model_imports.cancelled)?;
            let model = s.imported_models.iter_mut().find(|m| m.id == request.id)
                .ok_or_else(|| AppError::invalid("Saved model not found."))?;
            if model.profile_version != request.expected_profile_version { return Err(configuration_changed()); }
            model.experimental_configuration = Some(request.configuration.clone());
            model.profile_version = model.profile_version.saturating_add(1);
            model.quotes = model.current_quotes(&hardware);
            model.prices_checked_at = now();
            model.profile = Some(model.build_profile(s.settings.policy.max_hourly_usd.min(HOURLY_CEILING_USD))?
                .ok_or_else(|| AppError::invalid(model.profile_metadata_error(request.configuration.min_cache_gb)
                    .unwrap_or("Choose a GPU with a current positive price within your hourly limit. Review its count and refresh GPU options.")))?);
            Ok(())
        })?;
        self.changed();
        Ok(())
    }

    pub async fn imported_model_hardware(
        &self,
        id: &str,
    ) -> Result<crate::cloud_provider::Hardware> {
        let provider = self
            .store
            .read()?
            .imported_models
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| AppError::invalid("Saved model not found."))?
            .provider;
        self.providers.get(provider)?.hardware().await
    }
    pub fn remove_imported_model(&self, id: &str) -> Result<()> {
        let _guard = self
            .model_imports
            .lock
            .try_lock()
            .map_err(|_| import_busy())?;
        self.store.update(|s| {
            s.imported_models.retain(|m| m.id != id);
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
}
fn import_busy() -> AppError {
    AppError::new(
        "model_import_busy",
        "A model import or price refresh is already running.",
        "Wait for it to finish or cancel it in Models.",
    )
}

fn configuration_changed() -> AppError {
    AppError::new(
        "model_configuration_changed",
        "This saved model changed while you were editing.",
        "Close and reopen Edit configuration to load its latest revision and settings.",
    )
}
