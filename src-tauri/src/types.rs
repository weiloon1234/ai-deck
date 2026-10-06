use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const STATE_VERSION: u32 = 1;
pub const HOURLY_CEILING_USD: f64 = 10.0;

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentStage {
    Validating,
    Provisioning,
    ContainerStarting,
    Downloading,
    Loading,
    Verifying,
    Ready,
    Draining,
    Terminating,
    Terminated,
    Failed,
    CleanupPending,
    ReconciliationRequired,
}

impl DeploymentStage {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminated)
    }
    pub fn is_cleaning(&self) -> bool {
        matches!(
            self,
            Self::Draining | Self::Terminating | Self::CleanupPending
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CliKind {
    Codex,
    Claude,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    Starting,
    Running,
    Disconnected,
    Ended,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub paid_provisioning_enabled: bool,
    pub max_hourly_usd: f64,
    pub total_budget_usd: Option<f64>,
    pub max_lifetime_minutes: Option<u32>,
    pub acknowledge_offline_risk: bool,
    pub auto_terminate_delay_seconds: Option<u32>,
    pub claude_enabled: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            paid_provisioning_enabled: false,
            max_hourly_usd: HOURLY_CEILING_USD,
            total_budget_usd: None,
            max_lifetime_minutes: None,
            acknowledge_offline_risk: false,
            auto_terminate_delay_seconds: None,
            claude_enabled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub policy: Policy,
    pub hugging_face_secret: Option<String>,
    pub codex_path: Option<String>,
    pub claude_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: String,
    pub pod_name: String,
    pub pod_ids: Vec<String>,
    pub profile: crate::model_catalog::ResolvedProfile,
    pub stage: DeploymentStage,
    pub intended_action: String,
    pub created_at: u32,
    pub updated_at: u32,
    pub terminated_at: Option<u32>,
    pub deadline_at: u32,
    pub verified_at: Option<u32>,
    /// None means a legacy record whose startup history is unknown.
    #[serde(default)]
    pub startup_completed: Option<bool>,
    pub hourly_usd: f64,
    pub budget_usd: f64,
    pub gpu_type: String,
    pub gpu_count: u32,
    pub data_center_id: String,
    pub network_volume_id: Option<String>,
    pub retained_storage_monthly_usd: Option<f64>,
    pub credential_ref: String,
    #[serde(default)]
    pub failure_reason: Option<String>,
    pub message: String,
    pub cleanup_due_at: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub project_path: String,
    pub cli: CliKind,
    pub cli_version: String,
    pub deployment_id: String,
    pub profile_id: String,
    pub profile_version: u32,
    pub model: String,
    pub status: SessionStatus,
    pub created_at: u32,
    pub ended_at: Option<u32>,
    pub conversation_id: Option<String>,
    pub config_directory: String,
    pub unverified_combination: bool,
    pub exit_code: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LocalState {
    pub schema_version: u32,
    pub installation_id: String,
    pub settings: AppSettings,
    pub projects: Vec<Project>,
    pub deployments: Vec<Deployment>,
    pub sessions: Vec<Session>,
    pub enabled_deployment_id: Option<String>,
    #[serde(default)]
    pub imported_models: Vec<crate::model_analysis::ImportedModel>,
}
impl Default for LocalState {
    fn default() -> Self {
        Self {
            schema_version: STATE_VERSION,
            installation_id: uuid::Uuid::new_v4().to_string(),
            settings: AppSettings::default(),
            projects: vec![],
            deployments: vec![],
            sessions: vec![],
            enabled_deployment_id: None,
            imported_models: vec![],
        }
    }
}

impl LocalState {
    pub fn arm_idle_cleanup(&mut self, deployment_id: &str, at: u32) {
        let Some(delay) = self.settings.policy.auto_terminate_delay_seconds else {
            return;
        };
        if self
            .sessions
            .iter()
            .any(|s| s.deployment_id == deployment_id && s.status != SessionStatus::Ended)
        {
            return;
        }
        if let Some(d) = self
            .deployments
            .iter_mut()
            .find(|d| d.id == deployment_id && !d.stage.is_terminal())
        {
            let due = at.saturating_add(delay);
            d.cleanup_due_at = Some(d.cleanup_due_at.map_or(due, |existing| existing.min(due)));
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub state: LocalState,
    pub catalog: Vec<crate::model_catalog::ResolvedProfile>,
    pub runpod_key_configured: Option<bool>,
    pub catalog_error: Option<crate::error::AppError>,
    pub credential_store_error: Option<crate::error::AppError>,
    pub hourly_ceiling_usd: f64,
    pub offline_expiry_verified: bool,
    pub app_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionRequest {
    pub profile_id: String,
    pub gpu_type: String,
    pub data_center_id: String,
    pub network_volume_id: Option<String>,
    pub acknowledge_paid_creation: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub project_id: String,
    pub cli: CliKind,
    pub name: String,
    pub allow_unverified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallation {
    pub cli: CliKind,
    pub path: Option<String>,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalChunk {
    pub session_id: String,
    pub sequence: u32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalReplay {
    pub chunks: Vec<TerminalChunk>,
    pub running: bool,
}

pub fn now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(u32::MAX as u64) as u32
}
