use crate::{
    error::{AppError, Result},
    types::Deployment,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ProviderId {
    #[default]
    Runpod,
    Gputw,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub name: String,
    pub account_url: String,
    pub key_docs_url: String,
    pub provisioning_available: bool,
    pub limitations: Vec<String>,
}

impl ProviderId {
    pub const ALL: [Self; 2] = [Self::Runpod, Self::Gputw];

    pub fn info(self) -> ProviderInfo {
        match self {
            Self::Runpod => ProviderInfo {
                id: self, name: "Runpod".into(),
                account_url: "https://console.runpod.io/user/settings".into(),
                key_docs_url: "https://console.runpod.io/user/settings".into(),
                provisioning_available: true, limitations: vec![],
            },
            Self::Gputw => ProviderInfo {
                id: self, name: "GPUtw.ai".into(),
                account_url: "https://gputw.ai/".into(),
                key_docs_url: "https://docs.gputw.ai/zh-TW/docs/api-keys".into(),
                provisioning_available: false,
                limitations: vec![
                    "Confirm ownership markers or idempotency for recovery after a lost creation response.".into(),
                    "Confirm authenticated startup readiness during long model downloads.".into(),
                    "Confirm enforced workspace/Vault capacity, billing and compatible GPU profiles.".into(),
                ],
            },
        }
    }

    pub fn ensure_provisioning(self) -> Result<()> {
        if self.info().provisioning_available {
            return Ok(());
        }
        Err(AppError::new("provider_not_ready", "GPUtw provisioning is not available yet.",
            "You can save and check its key in Setup. Recovery, startup and storage requirements still need confirmation; no instance was created."))
    }
}

/// Route by the saved provider, never by the current Setup selection.
pub struct CloudProviders {
    adapters: HashMap<ProviderId, Arc<dyn CloudProvider>>,
}
impl CloudProviders {
    pub fn new(runpod: Arc<dyn CloudProvider>) -> Self {
        Self {
            adapters: HashMap::from([(ProviderId::Runpod, runpod)]),
        }
    }
    pub fn get(&self, id: ProviderId) -> Result<&dyn CloudProvider> {
        self.adapters.get(&id).map(Arc::as_ref).ok_or_else(|| {
            AppError::new(
                "provider_not_ready",
                "This provider has no deployment adapter.",
                "Choose a supported provider in Setup. No cloud request was sent.",
            )
        })
    }
}

// Normalized provider data. Keep the existing serialized field names stable.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudResource {
    pub id: String,
    pub name: String,
    pub image_name: String,
    pub desired_status: String,
    #[serde(default)]
    pub cost_per_hr: f64,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub network_volume_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GpuType {
    pub id: String,
    pub display_name: String,
    pub memory_in_gb: u32,
    pub secure_cloud: bool,
    pub secure_price: Option<f64>,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GpuAvailability {
    pub gpu_type_id: String,
    pub stock_status: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DataCenter {
    pub id: String,
    pub name: String,
    pub location: String,
    #[serde(default)]
    pub gpu_availability: Vec<GpuAvailability>,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NetworkVolume {
    pub id: String,
    pub name: String,
    pub size: u32,
    pub data_center_id: String,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Hardware {
    #[serde(default)]
    pub provider: ProviderId,
    pub gpus: Vec<GpuType>,
    pub data_centers: Vec<DataCenter>,
    pub network_volumes: Vec<NetworkVolume>,
}

#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Build the adapter's opaque create request before persisting creation intent.
    /// This must not make network calls or create resources.
    fn prepare_create(
        &self,
        deployment: &Deployment,
        owner: &str,
        token: &str,
        hf_secret: Option<&str>,
    ) -> Result<Value>;
    /// Confirm app-specific ownership, not merely access to the same account.
    fn owns(&self, deployment: &Deployment, owner: &str, resource: &CloudResource) -> bool;
    async fn create(&self, payload: Value) -> Result<CloudResource>;
    async fn list(&self) -> Result<Vec<CloudResource>>;
    async fn get(&self, id: &str) -> Result<Option<CloudResource>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn hardware(&self) -> Result<Hardware>;
}
