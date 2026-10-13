use crate::error::{AppError, Result};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::time::Duration;

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
pub(crate) fn protocol_error() -> AppError {
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
        400 | 422 => ("provider_configuration", "The provider rejected the deployment configuration.", "Check image, GPU availability, storage location, and account balance in the provider console."),
        _ => ("provider_unavailable", "The provider could not complete the request.", "Reconcile resources before retrying. Check the provider console for ongoing charges."),
    };
    AppError::new(code, message, recovery)
}
