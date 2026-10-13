use crate::{
    cloud_provider::{Hardware, ProviderId},
    error::{AppError, Result},
    hugging_face_client::HubModel,
    model_catalog::{
        parse_catalog, CatalogDocument, ModelProfile, ResolvedProfile, RuntimeProfile, Sampling,
    },
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
    #[serde(default)]
    pub provider: ProviderId,
    pub gpu_type: String,
    pub gpu_count: u32,
    pub memory_per_gpu_gb: u32,
    pub hourly_usd: Option<f64>,
    pub available_regions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExperimentalModelConfiguration {
    pub gpu_type: String,
    pub gpu_count: u32,
    pub min_cpu_ram_gb: u32,
    pub disk_gb: u32,
    pub min_cache_gb: u32,
    pub context_tokens: u32,
    pub tool_parser: Option<String>,
    pub reasoning_parser: Option<String>,
    pub quantization: Option<String>,
}
impl ExperimentalModelConfiguration {
    pub fn validate(&self) -> Result<()> {
        if self.gpu_type.trim().is_empty()
            || self.gpu_type.len() > 200
            || !(1..=8).contains(&self.gpu_count)
            || !(1..=16384).contains(&self.min_cpu_ram_gb)
            || !(20..=2000).contains(&self.disk_gb)
            || self.min_cache_gb == 0
            || self.min_cache_gb > self.disk_gb
            || !(1024..=2_097_152).contains(&self.context_tokens)
            || self
                .tool_parser
                .iter()
                .chain(&self.reasoning_parser)
                .chain(&self.quantization)
                .any(|s| s.len() > 64 || !crate::model_catalog::identifier(s) || s.starts_with('-'))
        {
            return Err(AppError::invalid("Choose a GPU, bounded memory/context/storage settings and built-in parser or quantization names. Paths, commands and extra flags are not accepted."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportedModel {
    #[serde(default)]
    pub provider: ProviderId,
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
    #[serde(default)]
    pub experimental_configuration: Option<ExperimentalModelConfiguration>,
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
    gpu_quotes(
        &analysis.compatible_gpu_types,
        analysis.gpu_count,
        analysis.minimum_total_vram_gb,
        hardware,
    )
}

fn gpu_quotes(
    gpu_types: &[String],
    gpu_count: u32,
    minimum_total_vram_gb: u32,
    hardware: &Hardware,
) -> Vec<GpuQuote> {
    let mut quotes: Vec<_> = hardware
        .gpus
        .iter()
        .filter(|g| {
            g.secure_cloud
                && gpu_types.contains(&g.id)
                && g.memory_in_gb > 0
                && g.memory_in_gb.saturating_mul(gpu_count) >= minimum_total_vram_gb
        })
        .map(|g| GpuQuote {
            provider: hardware.provider,
            gpu_type: g.id.clone(),
            gpu_count,
            memory_per_gpu_gb: g.memory_in_gb,
            hourly_usd: g
                .secure_price
                .filter(|p| p.is_finite() && *p > 0.0)
                .map(|p| p * f64::from(gpu_count))
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
    pub(crate) fn profile_metadata_error(&self, min_cache_gb: u32) -> Option<&'static str> {
        if self.metadata.is_adapter
            || !matches!(
                self.metadata.weight_format.as_deref(),
                Some("safetensors" | "pytorch")
            )
        {
            Some("This runtime needs full safetensors or PyTorch weights. Adapter-only and other weight formats cannot be configured here.")
        } else if self.experimental_configuration.is_none() && self.metadata.config_json.is_empty()
        {
            Some("Model configuration metadata is missing. Analyze again to retrieve it before configuring a trial.")
        } else if self
            .metadata
            .weight_size_gb
            .is_some_and(|gb| f64::from(min_cache_gb) <= gb)
        {
            Some("Required cache capacity must exceed the reported full-weight size to leave working space. Increase cache and disk capacity.")
        } else if self
            .metadata
            .pipeline_tag
            .as_deref()
            .is_some_and(|p| !["text-generation", "image-text-to-text"].contains(&p))
        {
            Some("This model's task is not supported by the text-generation runtime.")
        } else {
            None
        }
    }

    pub fn current_quotes(&self, hardware: &Hardware) -> Vec<GpuQuote> {
        match &self.experimental_configuration {
            Some(c) => gpu_quotes(std::slice::from_ref(&c.gpu_type), c.gpu_count, 0, hardware),
            None => quotes(&self.analysis, hardware),
        }
    }

    pub fn build_profile(&self, hourly_limit: f64) -> Result<Option<ResolvedProfile>> {
        let a = &self.analysis;
        let c = self.experimental_configuration.as_ref();
        if let Some(configuration) = c {
            configuration.validate()?;
        }
        let gpu_count = c.map_or(a.gpu_count, |c| c.gpu_count);
        let min_cache_gb = c.map_or(a.min_cache_gb, |c| c.min_cache_gb);
        let gpus: Vec<_> = self
            .quotes
            .iter()
            .filter(|q| {
                q.provider == self.provider
                    && q.gpu_count == gpu_count
                    && q.memory_per_gpu_gb > 0
                    && c.is_none_or(|c| q.gpu_type == c.gpu_type)
                    && q.hourly_usd
                        .is_some_and(|p| p.is_finite() && p > 0.0 && p <= hourly_limit)
            })
            .collect();
        // The runtime fetches the pinned config, including with a protected-model secret.
        // Explicit trials may lack a local config; spending and weight/task requirements remain.
        if self.provider != ProviderId::Runpod
            || (c.is_none() && (!a.runtime_compatible || a.tool_parser.is_none()))
            || gpus.is_empty()
            || self.profile_metadata_error(min_cache_gb).is_some()
        {
            return Ok(None);
        }
        let model = ModelProfile {
            id: self.id.clone(), display_name: self.metadata.repository.rsplit('/').next().unwrap_or(&self.metadata.repository).into(),
            schema_version: 1, version: self.profile_version,
            repository: self.metadata.repository.clone(), revision: self.metadata.revision.clone(), tokenizer_revision: None,
            served_model: self.id.clone(), runtime_id: self.runtime.id.clone(), access: self.metadata.access.clone(),
            gpu_types: gpus.iter().map(|g| g.gpu_type.clone()).collect(), gpu_count,
            min_gpu_memory_gb: c.map_or(a.minimum_total_vram_gb.div_ceil(gpu_count), |_| gpus[0].memory_per_gpu_gb), min_cpu_ram_gb: c.map_or(a.min_cpu_ram_gb, |c| c.min_cpu_ram_gb),
            disk_gb: c.map_or(a.disk_gb, |c| c.disk_gb), min_cache_gb, quantization: c.map_or_else(|| a.quantization.clone(), |c| c.quantization.clone()),
            context_tokens: c.map_or(a.recommended_context_tokens, |c| c.context_tokens), max_sessions: 1,
            sampling: Sampling { temperature: 0.7, top_p: 0.95 }, tool_parser: c.map_or_else(|| a.tool_parser.clone(), |c| c.tool_parser.clone()), reasoning_parser: c.map_or_else(|| a.reasoning_parser.clone(), |c| c.reasoning_parser.clone()),
            chat_template: None, model_cache_path: "/workspace/cache/huggingface".into(), compile_cache_path: "/workspace/cache/compile".into(),
            supports_responses: true, supports_messages: true, evidence: vec![],
            limitations: std::iter::once(if c.is_some() { format!("Owner-configured experimental trial for one coding session; no live compatibility or memory test has been performed. Codex assessment: {}", a.compatibility_reason) } else { "AI hardware suggestion for one coding session; no live compatibility or memory test has been performed.".into() }).chain(self.metadata.config_json.is_empty().then(|| "Model config was unavailable locally; architecture and published context remain unknown. Startup must retrieve the pinned files with the required model access.".into())).chain(a.warnings.clone()).collect(),
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
