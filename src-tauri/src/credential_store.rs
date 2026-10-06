use crate::error::{AppError, Result};

pub const RUNPOD_KEY: &str = "runpod-management";
pub trait CredentialStore: Send + Sync {
    fn get(&self, reference: &str) -> Result<Option<String>>;
    fn set(&self, reference: &str, value: &str) -> Result<()>;
    fn delete(&self, reference: &str) -> Result<()>;
}
pub struct OsCredentialStore;
fn entry(reference: &str) -> Result<keyring::Entry> {
    // Stable installed-app identity: a branding rename must not strand existing credentials.
    keyring::Entry::new("com.runpoddeck.desktop", reference).map_err(|_| credential_error())
}
fn credential_error() -> AppError {
    AppError::new(
        "credential_store",
        "The operating system credential store is unavailable or access was denied.",
        "Allow AI Deck to access its own Keychain entries, then retry.",
    )
}
impl CredentialStore for OsCredentialStore {
    fn get(&self, reference: &str) -> Result<Option<String>> {
        match entry(reference)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(credential_error()),
        }
    }
    fn set(&self, reference: &str, value: &str) -> Result<()> {
        entry(reference)?
            .set_password(value)
            .map_err(|_| credential_error())
    }
    fn delete(&self, reference: &str) -> Result<()> {
        match entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(credential_error()),
        }
    }
}
pub fn inference_credential() -> String {
    format!(
        "rpd_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
