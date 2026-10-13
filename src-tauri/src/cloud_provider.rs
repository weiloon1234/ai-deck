use crate::{error::Result, types::Deployment};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use ts_rs::TS;

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
