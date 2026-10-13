//! Read-only setup verification. No creation, port changes, exec or cleanup API.
use crate::{
    credential_store::{CredentialStore, GPUTW_KEY},
    error::{AppError, Result},
    http_client::{decode, http_client, protocol_error},
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

pub struct GputwClient {
    http: Client,
    credentials: Arc<dyn CredentialStore>,
}

#[cfg(test)]
#[path = "../tests/support/gputw_http.rs"]
mod tests;
#[derive(Deserialize)]
struct InstanceList {
    success: bool,
    data: Vec<Value>,
}
impl GputwClient {
    pub fn new(credentials: Arc<dyn CredentialStore>) -> Result<Self> {
        Ok(Self {
            http: http_client(20)?,
            credentials,
        })
    }
    pub async fn check_connection(&self) -> Result<()> {
        let key = self.credentials.get(GPUTW_KEY)?.ok_or_else(|| {
            AppError::new(
                "missing_gputw_key",
                "Add your GPUtw API key in Setup.",
                "Use a key with instances:read for the read-only connection check.",
            )
        })?;
        self.check_at("https://gputw.ai/api/instances", &key).await
    }
    async fn check_at(&self, url: &str, key: &str) -> Result<()> {
        let response = self
            .http
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                concat!("AI-Deck/", env!("CARGO_PKG_VERSION")),
            )
            .header(reqwest::header::ACCEPT, "application/json")
            .bearer_auth(key)
            .send()
            .await
            .map_err(|_| AppError::network())?;
        let list: InstanceList = decode(response).await?;
        if !list.success || list.data.iter().any(|item| !item.is_object()) {
            return Err(protocol_error());
        }
        // Discard all upstream account records. This proves read access only.
        Ok(())
    }
}
