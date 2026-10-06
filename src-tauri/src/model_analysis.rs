use crate::{
    error::{AppError, Result},
    hugging_face_client::HubModel,
    model_catalog::{
        parse_catalog, CatalogDocument, ModelProfile, ResolvedProfile, RuntimeProfile, Sampling,
    },
    runpod_client::Hardware,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Codex proposes requirements, never prices, credentials, URLs, images or commands.
#[derive(Clone, Debug, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelAnalysis {
    pub summary: String,
    pub runtime_compatible: bool,
    pub compatibility_reason: String,
    pub minimum_total_vram_gb: u32,
    pub gpu_count: u32,
    pub compatible_gpu_types: Vec<String>,
    pub min_cpu_ram_gb: u32,
    pub min_cache_gb: u32,
    pub disk_gb: u32,
    pub context_min_tokens: u32,
    pub context_max_tokens: u32,
    pub recommended_context_tokens: u32,
    pub tool_parser: Option<String>,
    pub reasoning_parser: Option<String>,
    pub quantization: Option<String>,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GpuQuote {
    pub gpu_type: String,
    pub gpu_count: u32,
    pub memory_per_gpu_gb: u32,
    pub hourly_usd: Option<f64>,
    pub available_regions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportedModel {
    pub id: String,
    pub source_url: String,
    pub metadata: HubModel,
    pub analysis: ModelAnalysis,
    pub analyzer_version: String,
    pub analyzed_at: u32,
    pub prices_checked_at: u32,
    pub quotes: Vec<GpuQuote>,
    pub runtime: RuntimeProfile,
    pub profile_version: u32,
    pub profile: Option<ResolvedProfile>,
}

impl ModelAnalysis {
    pub fn validate(&self, hardware: &Hardware) -> Result<()> {
        let bounded = |s: &String| !s.trim().is_empty() && s.len() <= 4000;
        if !bounded(&self.summary)
            || !bounded(&self.compatibility_reason)
            || self.assumptions.len() > 20
            || self.warnings.len() > 20
            || !self.assumptions.iter().chain(&self.warnings).all(bounded)
            || !(1..=8192).contains(&self.minimum_total_vram_gb)
            || !(1..=8).contains(&self.gpu_count)
            || !(1..=16384).contains(&self.min_cpu_ram_gb)
            || !(20..=2000).contains(&self.disk_gb)
            || self.min_cache_gb == 0
            || self.min_cache_gb > self.disk_gb
            || self.context_min_tokens < 1024
            || self.context_max_tokens > 2_097_152
            || self.context_min_tokens > self.recommended_context_tokens
            || self.recommended_context_tokens > self.context_max_tokens
            || self.compatible_gpu_types.len() > 50
            || self
                .compatible_gpu_types
                .iter()
                .any(|id| !hardware.gpus.iter().any(|g| &g.id == id))
            || self
                .tool_parser
                .iter()
                .chain(&self.reasoning_parser)
                .chain(&self.quantization)
                .any(|s| s.len() > 64 || !crate::model_catalog::identifier(s) || s.starts_with('-'))
        {
            return Err(invalid_analysis());
        }
        Ok(())
    }
}
pub fn invalid_analysis() -> AppError {
    AppError::new(
        "model_analysis_invalid",
        "Codex returned an incomplete or invalid hardware suggestion.",
        "Retry analysis. No existing model or deployment has been changed.",
    )
}

pub fn quotes(analysis: &ModelAnalysis, hardware: &Hardware) -> Vec<GpuQuote> {
    let mut quotes: Vec<_> = hardware
        .gpus
        .iter()
        .filter(|g| {
            g.secure_cloud
                && analysis.compatible_gpu_types.contains(&g.id)
                && g.memory_in_gb.saturating_mul(analysis.gpu_count)
                    >= analysis.minimum_total_vram_gb
        })
        .map(|g| GpuQuote {
            gpu_type: g.id.clone(),
            gpu_count: analysis.gpu_count,
            memory_per_gpu_gb: g.memory_in_gb,
            hourly_usd: g
                .secure_price
                .filter(|p| p.is_finite() && *p > 0.0)
                .map(|p| p * f64::from(analysis.gpu_count))
                .filter(|p| p.is_finite()),
            available_regions: hardware
                .data_centers
                .iter()
                .filter(|dc| {
                    dc.gpu_availability.iter().any(|a| {
                        a.gpu_type_id == g.id
                            && a.stock_status.as_deref().is_some_and(|s| {
                                ["high", "medium", "low"].contains(&s.to_ascii_lowercase().as_str())
                            })
                    })
                })
                .map(|dc| dc.id.clone())
                .collect(),
        })
        .collect();
    quotes.sort_by(|a, b| {
        a.hourly_usd
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.hourly_usd.unwrap_or(f64::INFINITY))
            .then(a.gpu_type.cmp(&b.gpu_type))
    });
    quotes
}

impl ImportedModel {
    pub fn build_profile(&self, hourly_limit: f64) -> Result<Option<ResolvedProfile>> {
        let a = &self.analysis;
        let gpus: Vec<_> = self
            .quotes
            .iter()
            .filter(|q| q.hourly_usd.is_some_and(|p| p <= hourly_limit))
            .collect();
        // Save unsupported analyses, but do not offer an invented working runtime.
        if !a.runtime_compatible
            || a.tool_parser.is_none()
            || gpus.is_empty()
            || self.metadata.config_json.is_empty()
            || self.metadata.weight_format.is_none()
            || self.metadata.is_adapter
            || self
                .metadata
                .weight_size_gb
                .is_some_and(|gb| f64::from(a.min_cache_gb) <= gb)
            || self
                .metadata
                .pipeline_tag
                .as_deref()
                .is_some_and(|p| !["text-generation", "image-text-to-text"].contains(&p))
        {
            return Ok(None);
        }
        let model = ModelProfile {
            id: self.id.clone(), display_name: self.metadata.repository.rsplit('/').next().unwrap_or(&self.metadata.repository).into(),
            schema_version: 1, version: self.profile_version,
            repository: self.metadata.repository.clone(), revision: self.metadata.revision.clone(), tokenizer_revision: None,
            served_model: self.id.clone(), runtime_id: self.runtime.id.clone(), access: self.metadata.access.clone(),
            gpu_types: gpus.iter().map(|g| g.gpu_type.clone()).collect(), gpu_count: a.gpu_count,
            min_gpu_memory_gb: a.minimum_total_vram_gb.div_ceil(a.gpu_count), min_cpu_ram_gb: a.min_cpu_ram_gb,
            disk_gb: a.disk_gb, min_cache_gb: a.min_cache_gb, quantization: a.quantization.clone(),
            context_tokens: a.recommended_context_tokens, max_sessions: 1,
            sampling: Sampling { temperature: 0.7, top_p: 0.95 }, tool_parser: a.tool_parser.clone(), reasoning_parser: a.reasoning_parser.clone(),
            chat_template: None, model_cache_path: "/workspace/cache/huggingface".into(), compile_cache_path: "/workspace/cache/compile".into(),
            supports_responses: true, supports_messages: true, evidence: vec![],
            limitations: std::iter::once("AI hardware suggestion for one coding session; no live compatibility or memory test has been performed.".into()).chain(a.warnings.clone()).collect(),
        };
        let json = serde_json::to_string(&CatalogDocument {
            schema_version: 1,
            runtimes: vec![self.runtime.clone()],
            profiles: vec![model],
        })
        .map_err(|_| invalid_analysis())?;
        Ok(parse_catalog(&json)?.into_iter().next())
    }
}
