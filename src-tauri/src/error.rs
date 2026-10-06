use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub recovery: String,
}
pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn new(code: &str, message: impl Into<String>, recovery: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recovery: recovery.into(),
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(
            "invalid_input",
            message,
            "Review the highlighted settings and try again.",
        )
    }
    pub fn storage() -> Self {
        Self::new(
            "local_storage",
            "Unable to read or safely save app state.",
            "Check app-data permissions and free disk space. Existing state has been preserved.",
        )
    }
    pub fn network() -> Self {
        Self::new("network_unavailable", "The service could not be reached.", "Check your connection, then reconcile. An uncertain create request must not be retried.")
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for AppError {}
