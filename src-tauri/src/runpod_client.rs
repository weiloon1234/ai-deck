use crate::{
    cloud_provider::{CloudProvider, CloudResource, Hardware, ProviderId},
    credential_store::{CredentialStore, RUNPOD_KEY},
    error::{AppError, Result},
    http_client::{bounded_body, decode, http_client, protocol_error, resource_id, status_error},
    model_catalog::{identifier, ModelAccess},
    types::Deployment,
};
use async_trait::async_trait;
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct RunpodClient {
    http: Client,
    credentials: Arc<dyn CredentialStore>,
}

#[cfg(test)]
#[path = "../tests/support/runpod_http.rs"]
mod tests;

async fn decode_creation(response: Response) -> Result<CloudResource> {
    let status = response.status();
    if status.is_success() {
        return decode(response).await;
    }
    let mut error = status_error(status);
    error.message = format!(
        "Runpod: {} (HTTP {}).",
        error.message.trim_end_matches('.'),
        status.as_u16()
    );
    if matches!(status.as_u16(), 400 | 422) {
        // Never persist upstream messages/values: they can echo the credential-bearing payload.
        // Unknown, malformed or oversized bodies retain the known HTTP rejection status.
        if let Ok(bytes) = bounded_body(response, 32 * 1024).await {
            if let Ok(body) = serde_json::from_slice::<Value>(&bytes) {
                let mut fields = std::collections::BTreeSet::new();
                creation_error_fields(&body, 0, &mut fields);
                if !fields.is_empty() {
                    error.message.push_str(&format!(
                        " Response references: {}.",
                        fields.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
        }
    }
    Err(error)
}

fn creation_error_fields(
    value: &Value,
    depth: u8,
    fields: &mut std::collections::BTreeSet<&'static str>,
) {
    if depth > 5 {
        return;
    }
    match value {
        Value::String(message) => {
            for word in message.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                let label = match word {
                    "gpuTypeIds" => "GPU type",
                    "gpuCount" => "GPU count",
                    "dataCenterIds" => "location",
                    "imageName" => "container image",
                    "containerDiskInGb" => "temporary disk",
                    "networkVolumeId" | "volumeInGb" | "volumeMountPath" => "model storage",
                    "minRAMPerGPU" => "CPU memory",
                    "minVCPUPerGPU" => "CPU cores",
                    "ports" => "ports",
                    "dockerEntrypoint" | "dockerStartCmd" => "startup command",
                    "env" => "environment or secret references",
                    _ => continue,
                };
                fields.insert(label);
            }
        }
        Value::Array(items) => {
            for item in items {
                creation_error_fields(item, depth + 1, fields);
            }
        }
        Value::Object(object) => {
            for key in [
                "error",
                "message",
                "errors",
                "details",
                "issues",
                "path",
                "instancePath",
                "field",
                "property",
            ] {
                if let Some(value) = object.get(key) {
                    creation_error_fields(value, depth + 1, fields);
                }
            }
        }
        _ => {}
    }
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
impl CloudProvider for RunpodClient {
    fn prepare_create(
        &self,
        deployment: &Deployment,
        owner: &str,
        token: &str,
        hf_secret: Option<&str>,
    ) -> Result<Value> {
        create_payload(deployment, owner, token, hf_secret)
    }
    fn owns(&self, deployment: &Deployment, owner: &str, resource: &CloudResource) -> bool {
        owns(deployment, owner, resource)
    }
    async fn create(&self, payload: Value) -> Result<CloudResource> {
        // Deliberately no POST retry: an error may follow a successful creation.
        decode_creation(self.request(Method::POST, "/pods", Some(payload)).await?).await
    }
    async fn list(&self) -> Result<Vec<CloudResource>> {
        decode(self.request(Method::GET, "/pods", None).await?).await
    }
    async fn get(&self, id: &str) -> Result<Option<CloudResource>> {
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
            provider: ProviderId::Runpod,
            gpus,
            data_centers,
            network_volumes,
        })
    }
}

// Ownership keys are a persistent protocol shared with already-running Pods.
pub fn owns(d: &Deployment, owner: &str, pod: &CloudResource) -> bool {
    d.provider == ProviderId::Runpod
        && pod.name == d.pod_name
        && pod.image_name == d.profile.runtime.image
        && pod.env.get("RUNPOD_DECK_OWNER").map(String::as_str) == Some(owner)
        && pod.env.get("RUNPOD_DECK_DEPLOYMENT").map(String::as_str) == Some(d.id.as_str())
        && pod.network_volume_id.as_deref().filter(|s| !s.is_empty())
            == d.network_volume_id.as_deref()
}
pub fn create_payload(
    d: &Deployment,
    owner: &str,
    token: &str,
    hf_secret: Option<&str>,
) -> Result<Value> {
    if d.provider != ProviderId::Runpod {
        return Err(AppError::invalid(
            "This deployment does not belong to Runpod.",
        ));
    }
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
