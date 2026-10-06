use crate::{
    error::{AppError, Result},
    types::CliKind,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeProfile {
    pub id: String,
    pub engine: String,
    pub version: String,
    pub image: String,
    pub inference_port: u16,
    pub status_port: u16,
    pub download_timeout_seconds: u32,
    pub load_timeout_seconds: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sampling {
    pub temperature: f64,
    pub top_p: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModelAccess {
    Public,
    Gated,
    Private,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityEvidence {
    pub cli: String,
    pub cli_version: String,
    pub gpu_type: String,
    pub fingerprint: String,
    pub tested_at: String,
    pub checks: Vec<String>,
    pub report: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelProfile {
    pub id: String,
    pub display_name: String,
    pub schema_version: u32,
    pub version: u32,
    pub repository: String,
    pub revision: String,
    pub tokenizer_revision: Option<String>,
    pub served_model: String,
    pub runtime_id: String,
    pub access: ModelAccess,
    pub gpu_types: Vec<String>,
    pub gpu_count: u32,
    pub min_gpu_memory_gb: u32,
    pub min_cpu_ram_gb: u32,
    pub disk_gb: u32,
    /// Zero is accepted only when deserializing legacy deployment snapshots.
    #[serde(default)]
    pub min_cache_gb: u32,
    pub quantization: Option<String>,
    pub context_tokens: u32,
    pub max_sessions: u32,
    pub sampling: Sampling,
    pub tool_parser: Option<String>,
    pub reasoning_parser: Option<String>,
    pub chat_template: Option<String>,
    pub model_cache_path: String,
    pub compile_cache_path: String,
    pub supports_responses: bool,
    pub supports_messages: bool,
    pub evidence: Vec<CompatibilityEvidence>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogDocument {
    pub schema_version: u32,
    pub runtimes: Vec<RuntimeProfile>,
    pub profiles: Vec<ModelProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProfile {
    pub model: ModelProfile,
    pub runtime: RuntimeProfile,
    pub fingerprint: String,
}

pub const REQUIRED_CODING_CHECKS: &[&str] = &[
    "streaming",
    "file_read",
    "file_edit",
    "safe_command",
    "tool_result_followup",
    "cancellation",
    "model_identity",
    "context_limit",
    "subscription_isolation",
    "invalid_credential_no_fallback",
    "two_sessions",
];

impl ResolvedProfile {
    pub fn is_validated(&self, cli: &CliKind, version: &str, gpu_type: &str) -> bool {
        let name = match cli {
            CliKind::Codex => "codex",
            CliKind::Claude => "claude",
        };
        self.model.evidence.iter().any(|e| {
            e.cli == name
                && e.cli_version == version
                && e.gpu_type == gpu_type
                && e.fingerprint == self.fingerprint
                && !e.tested_at.is_empty()
                && !e.report.is_empty()
                && REQUIRED_CODING_CHECKS
                    .iter()
                    .all(|check| e.checks.iter().any(|v| v == check))
                && (*cli != CliKind::Claude || e.checks.iter().any(|v| v == "helper_models"))
        })
    }
    pub fn arguments(&self) -> Vec<String> {
        let p = &self.model;
        let mut args = vec![
            "--model".into(),
            p.repository.clone(),
            "--revision".into(),
            p.revision.clone(),
            "--served-model-name".into(),
            p.served_model.clone(),
            "--host".into(),
            "0.0.0.0".into(),
            "--port".into(),
            self.runtime.inference_port.to_string(),
            "--tensor-parallel-size".into(),
            p.gpu_count.to_string(),
            "--max-model-len".into(),
            p.context_tokens.to_string(),
            "--max-num-seqs".into(),
            p.max_sessions.to_string(),
            "--generation-config".into(),
            "vllm".into(),
            "--override-generation-config".into(),
            serde_json::json!({"temperature":p.sampling.temperature,"top_p":p.sampling.top_p})
                .to_string(),
        ];
        for (flag, value) in [
            ("--tokenizer-revision", &p.tokenizer_revision),
            ("--quantization", &p.quantization),
            ("--tool-call-parser", &p.tool_parser),
            ("--reasoning-parser", &p.reasoning_parser),
            ("--chat-template", &p.chat_template),
        ] {
            if let Some(value) = value {
                args.extend([flag.to_owned(), value.clone()]);
            }
        }
        if p.tool_parser.is_some() {
            args.push("--enable-auto-tool-choice".into());
        }
        args
    }
}

pub fn builtin_catalog() -> Result<Vec<ResolvedProfile>> {
    parse_catalog(include_str!("../../model-catalog/catalog.json"))
}
pub fn parse_catalog(json: &str) -> Result<Vec<ResolvedProfile>> {
    if json.len() > 1024 * 1024 {
        return Err(AppError::invalid("Catalog is too large."));
    }
    let catalog: CatalogDocument = serde_json::from_str(json)
        .map_err(|_| AppError::invalid("Catalog does not match the model schema."))?;
    if catalog.schema_version != 1 {
        return Err(AppError::invalid("Unsupported catalog schema version."));
    }
    let mut ids = std::collections::HashSet::new();
    for runtime in &catalog.runtimes {
        if !ids.insert(runtime.id.clone())
            || runtime.engine != "vllm"
            || runtime.version.is_empty()
            || !runtime.image.starts_with("vllm/vllm-openai@sha256:")
            || !is_hex(runtime.image.rsplit(':').next().unwrap_or(""), 64)
            || runtime.inference_port < 1024
            || runtime.status_port < 1024
            || runtime.inference_port == runtime.status_port
            || !(60..=14400).contains(&runtime.download_timeout_seconds)
            || !(60..=7200).contains(&runtime.load_timeout_seconds)
        {
            return Err(AppError::invalid("Runtime must use a pinned vLLM digest, distinct unprivileged ports, and bounded timeouts."));
        }
    }
    ids.clear();
    catalog.profiles.into_iter().map(|model| {
        if !ids.insert(model.id.clone()) || !identifier(&model.id) || model.schema_version != 1 || model.version == 0
            || !is_hex(&model.revision, 40) || model.tokenizer_revision.as_ref().is_some_and(|v| !is_hex(v, 40))
            || model.repository.split('/').count() != 2 || !model.repository.split('/').all(identifier)
            || !identifier(&model.served_model) || model.gpu_types.is_empty() || model.gpu_count == 0 || model.gpu_count > 8
            || model.max_sessions == 0 || model.max_sessions > 16 || model.context_tokens < 1024
            || model.disk_gb < 20 || model.disk_gb > 2000 || model.min_cpu_ram_gb == 0 || model.min_gpu_memory_gb == 0
            || model.min_cache_gb == 0 || model.min_cache_gb > model.disk_gb
            || !model.sampling.temperature.is_finite() || !(0.0..=2.0).contains(&model.sampling.temperature)
            || !model.sampling.top_p.is_finite() || !(0.0..=1.0).contains(&model.sampling.top_p)
            || !safe_cache_path(&model.model_cache_path) || !safe_cache_path(&model.compile_cache_path)
            || model.model_cache_path == model.compile_cache_path {
            return Err(AppError::invalid("Model profile has invalid identity, immutable revision, hardware, inference, or cache settings."));
        }
        let runtime = catalog.runtimes.iter().find(|r| r.id == model.runtime_id).cloned().ok_or_else(|| AppError::invalid("Model references an unknown runtime."))?;
        let mut unsigned = model.clone(); unsigned.evidence.clear();
        let encoded = serde_json::to_vec(&(unsigned, &runtime)).map_err(|_| AppError::invalid("Cannot fingerprint profile."))?;
        let mut digest = Sha256::new();
        digest.update(encoded);
        digest.update(include_bytes!("../../containers/vllm-runtime/launcher.py"));
        let fingerprint = format!("{:x}", digest.finalize());
        Ok(ResolvedProfile { model, runtime, fingerprint })
    }).collect()
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && value != "."
        && value != ".."
}
fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn safe_cache_path(s: &str) -> bool {
    s.starts_with("/workspace/cache/") && !s.contains("..") && !s.contains('\0') && s.len() < 200
}
