use crate::{
    app_core::AppCore,
    cloud_provider::{Hardware, ProviderId, ProviderInfo},
    credential_store::management_key,
    error::{AppError, Result},
    gputw_client::GputwClient,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub info: ProviderInfo,
    pub key_configured: Option<bool>,
    pub credential_store_error: Option<AppError>,
}

impl AppCore {
    pub async fn select_provider(&self, provider: ProviderId) -> Result<()> {
        let _guard = self.operations.lock().await;
        self.store.update(|s| {
            s.settings.provider = provider;
            Ok(())
        })?;
        self.changed();
        Ok(())
    }
    pub fn provider_statuses(&self) -> Vec<ProviderStatus> {
        ProviderId::ALL
            .into_iter()
            .map(|provider| {
                let (key_configured, credential_store_error) =
                    match self.credentials.get(management_key(provider)) {
                        Ok(key) => (Some(key.is_some()), None),
                        Err(error) => (None, Some(error)),
                    };
                ProviderStatus {
                    info: provider.info(),
                    key_configured,
                    credential_store_error,
                }
            })
            .collect()
    }

    pub async fn save_provider_key(&self, provider: ProviderId, key: &str) -> Result<()> {
        let _guard = self.operations.lock().await;
        let key = key.trim();
        if key.len() < 16
            || key.len() > 1024
            || key.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(AppError::invalid(format!(
                "Enter a valid {} API key.",
                provider.info().name
            )));
        }
        self.ensure_key_change(provider)?;
        self.credentials.set(management_key(provider), key)?;
        self.changed();
        Ok(())
    }

    pub async fn remove_provider_key(&self, provider: ProviderId) -> Result<()> {
        let _guard = self.operations.lock().await;
        self.ensure_key_change(provider)?;
        self.credentials.delete(management_key(provider))?;
        self.changed();
        Ok(())
    }

    fn ensure_key_change(&self, provider: ProviderId) -> Result<()> {
        if self
            .store
            .read()?
            .deployments
            .iter()
            .any(|d| d.provider == provider && !d.stage.is_terminal())
        {
            return Err(AppError::invalid(format!("Finish existing {} deployments before changing its management key, so cleanup keeps the same account.", provider.info().name)));
        }
        Ok(())
    }

    pub async fn check_provider_connection(&self, provider: ProviderId) -> Result<()> {
        // Read-only setup requests must not delay deployment cleanup.
        match provider {
            ProviderId::Runpod => {
                self.providers.get(provider)?.hardware().await?;
            }
            ProviderId::Gputw => {
                GputwClient::new(self.credentials.clone())?
                    .check_connection()
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn hardware_for(&self, provider: ProviderId) -> Result<Hardware> {
        if self.store.read()?.settings.provider != provider {
            return Err(AppError::invalid(
                "The provider selection changed. Refresh availability for the saved provider.",
            ));
        }
        self.providers.get(provider)?.hardware().await
    }
}
