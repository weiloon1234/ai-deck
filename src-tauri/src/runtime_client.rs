use crate::{
    credential_store::CredentialStore,
    error::{AppError, Result},
    runpod_client::{decode, http_client, resource_id, status_error},
    types::Deployment,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Deserialize)]
pub struct RuntimeStatus {
    pub stage: String,
    pub code: String,
}

#[async_trait]
pub trait RuntimeApi: Send + Sync {
    async fn status(&self, deployment: &Deployment) -> Result<RuntimeStatus>;
    async fn verify(&self, deployment: &Deployment) -> Result<()>;
}
pub struct RuntimeClient {
    http: Client,
    credentials: Arc<dyn CredentialStore>,
}
impl RuntimeClient {
    pub fn new(credentials: Arc<dyn CredentialStore>) -> Result<Self> {
        Ok(Self {
            http: http_client(60)?,
            credentials,
        })
    }
    fn token(&self, deployment: &Deployment) -> Result<String> {
        self.credentials
            .get(&deployment.credential_ref)?
            .ok_or_else(|| {
                AppError::new(
                    "inference_credential_missing",
                    "This deployment's inference credential is unavailable.",
                    "Restore Keychain access or finish this deployment and provision a new one.",
                )
            })
    }
}
pub fn endpoint(deployment: &Deployment, port: u16) -> Result<String> {
    if deployment.pod_ids.len() != 1 {
        return Err(AppError::invalid(
            "Exactly one reconciled Pod is required for routing.",
        ));
    }
    let id = &deployment.pod_ids[0];
    resource_id(id)?;
    Ok(format!("https://{id}-{port}.proxy.runpod.net"))
}
#[async_trait]
impl RuntimeApi for RuntimeClient {
    async fn status(&self, deployment: &Deployment) -> Result<RuntimeStatus> {
        let url = format!(
            "{}/status",
            endpoint(deployment, deployment.profile.runtime.status_port)?
        );
        decode(
            self.http
                .get(url)
                .bearer_auth(self.token(deployment)?)
                .send()
                .await
                .map_err(|_| AppError::network())?,
        )
        .await
    }
    async fn verify(&self, deployment: &Deployment) -> Result<()> {
        let base = endpoint(deployment, deployment.profile.runtime.inference_port)?;
        self.verify_at(
            &base,
            &deployment.profile.model.served_model,
            &self.token(deployment)?,
        )
        .await
    }
}

impl RuntimeClient {
    async fn verify_at(&self, base: &str, expected: &str, token: &str) -> Result<()> {
        let invalid = self
            .http
            .get(format!("{base}/v1/models"))
            .bearer_auth("ai-deck-invalid-probe")
            .send()
            .await
            .map_err(|_| AppError::network())?;
        if !matches!(invalid.status().as_u16(), 401 | 403) {
            if !invalid.status().is_success() {
                return Err(status_error(invalid.status()));
            }
            return Err(AppError::new(
                "endpoint_not_protected",
                "The inference endpoint did not reject an invalid credential.",
                "Fix endpoint authentication before enabling this deployment.",
            ));
        }
        let models: Value = decode(
            self.http
                .get(format!("{base}/v1/models"))
                .bearer_auth(token)
                .send()
                .await
                .map_err(|_| AppError::network())?,
        )
        .await?;
        if !models["data"]
            .as_array()
            .is_some_and(|items| items.iter().any(|m| m["id"].as_str() == Some(expected)))
        {
            return Err(AppError::new(
                "wrong_model",
                "The endpoint is serving a different model.",
                "Finish this deployment and inspect its pinned profile before recreating it.",
            ));
        }
        let response: Value = decode(self.http.post(format!("{base}/v1/responses")).bearer_auth(token)
            .json(&json!({"model":expected,"input":"Reply with OK.","max_output_tokens":64,"stream":false}))
            .send().await.map_err(|_| AppError::network())?).await?;
        if response["model"].as_str() != Some(expected)
            || !matches!(
                response["status"].as_str(),
                Some("completed" | "incomplete")
            )
            || !response["output"]
                .as_array()
                .is_some_and(|output| !output.is_empty())
        {
            return Err(AppError::new(
                "inference_verification",
                "The endpoint did not return a valid inference result for the selected model.",
                "Review runtime compatibility. A RUNNING Pod alone is not ready for coding.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/support/runtime_http.rs"]
mod tests;
