use crate::{
    credential_store::{CredentialStore, RUNPOD_KEY},
    error::{AppError, Result},
};
use async_trait::async_trait;
use reqwest::{Client, Method, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use ts_rs::TS;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pod {
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
pub trait RunpodApi: Send + Sync {
    async fn create(&self, payload: Value) -> Result<Pod>;
    async fn list(&self) -> Result<Vec<Pod>>;
    async fn get(&self, id: &str) -> Result<Option<Pod>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn hardware(&self) -> Result<Hardware>;
}

pub struct RunpodClient {
    http: Client,
    credentials: Arc<dyn CredentialStore>,
}
impl RunpodClient {
    pub fn new(credentials: Arc<dyn CredentialStore>) -> Result<Self> {
        Ok(Self {
            http: http_client(35)?,
            credentials,
        })
    }
    async fn request(
        &self,
        method: Method,
        path: &str,
        payload: Option<Value>,
    ) -> Result<Response> {
        let key = self.credentials.get(RUNPOD_KEY)?.ok_or_else(|| {
            AppError::new(
                "missing_runpod_key",
                "Add your Runpod API key in Setup.",
                "The key is stored in your operating system credential store.",
            )
        })?;
        let url = if path == "graphql" {
            "https://api.runpod.io/graphql".to_owned()
        } else {
            format!("https://rest.runpod.io/v1{path}")
        };
        let mut request = self.http.request(method, url).bearer_auth(key);
        if let Some(payload) = payload {
            request = request.json(&payload);
        }
        request.send().await.map_err(|_| AppError::network())
    }
}

#[async_trait]
impl RunpodApi for RunpodClient {
    async fn create(&self, payload: Value) -> Result<Pod> {
        // Deliberately no POST retry: an error may follow a successful creation.
        decode(self.request(Method::POST, "/pods", Some(payload)).await?).await
    }
    async fn list(&self) -> Result<Vec<Pod>> {
        decode(self.request(Method::GET, "/pods", None).await?).await
    }
    async fn get(&self, id: &str) -> Result<Option<Pod>> {
        resource_id(id)?;
        let response = self
            .request(Method::GET, &format!("/pods/{id}"), None)
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        decode(response).await.map(Some)
    }
    async fn delete(&self, id: &str) -> Result<()> {
        resource_id(id)?;
        let response = self
            .request(Method::DELETE, &format!("/pods/{id}"), None)
            .await?;
        if response.status().is_success() || response.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(status_error(response.status()))
        }
    }
    async fn hardware(&self) -> Result<Hardware> {
        let query = "query { gpuTypes { id displayName memoryInGb secureCloud securePrice } dataCenters { id name location gpuAvailability { gpuTypeId stockStatus } } }";
        let response: Value = decode(
            self.request(
                Method::POST,
                "graphql",
                Some(serde_json::json!({"query":query})),
            )
            .await?,
        )
        .await?;
        if response
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|e| !e.is_empty())
        {
            return Err(AppError::new(
                "availability_error",
                "Runpod could not provide GPU availability.",
                "Check account permissions and retry the availability check.",
            ));
        }
        let data = response.get("data").ok_or_else(protocol_error)?;
        let gpus =
            serde_json::from_value(data["gpuTypes"].clone()).map_err(|_| protocol_error())?;
        let data_centers =
            serde_json::from_value(data["dataCenters"].clone()).map_err(|_| protocol_error())?;
        let network_volumes =
            decode(self.request(Method::GET, "/networkvolumes", None).await?).await?;
        Ok(Hardware {
            gpus,
            data_centers,
            network_volumes,
        })
    }
}

pub fn resource_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(AppError::invalid("Invalid cloud resource identity."));
    }
    Ok(())
}
pub fn http_client(timeout_seconds: u64) -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| AppError::network())
}
pub async fn decode<T: DeserializeOwned>(response: Response) -> Result<T> {
    if !response.status().is_success() {
        return Err(status_error(response.status()));
    }
    let bytes = bounded_body(response, 2 * 1024 * 1024).await?;
    serde_json::from_slice(&bytes).map_err(|_| protocol_error())
}
pub async fn bounded_body(mut response: Response, limit: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| AppError::network())? {
        if body.len() + chunk.len() > limit {
            return Err(protocol_error());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
fn protocol_error() -> AppError {
    AppError::new(
        "provider_protocol",
        "The provider returned an unexpected response.",
        "Reconcile existing resources before trying another deployment.",
    )
}
pub fn status_error(status: StatusCode) -> AppError {
    let (code, message, recovery) = match status.as_u16() {
        401 | 403 => ("authentication", "The service rejected the credential or account permissions.", "Check the correct credential and its permissions. Subscription fallback is disabled."),
        429 => ("rate_limit", "The service is rate limiting requests.", "Wait briefly, then reconcile before retrying."),
        400 | 422 => ("provider_configuration", "The provider rejected the deployment configuration.", "Check image, GPU availability, storage location, and account balance in the Runpod console."),
        _ => ("provider_unavailable", "The provider could not complete the request.", "Reconcile resources before retrying. Check the Runpod console for ongoing charges."),
    };
    AppError::new(code, message, recovery)
}
