use crate::{
    error::{AppError, Result},
    model_catalog::{identifier, ModelAccess},
    runpod_client::{bounded_body, http_client},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSource {
    pub repository: String,
    pub reference: String,
}
impl ModelSource {
    pub fn parse(input: &str) -> Result<Self> {
        let invalid = || {
            AppError::invalid(
                "Paste a Hugging Face model URL, optionally ending in /tree/revision.",
            )
        };
        let input = input.trim();
        if input.len() > 500 || input.contains(['%', '\\', '?', '#']) {
            return Err(invalid());
        }
        let url = reqwest::Url::parse(input).map_err(|_| invalid())?;
        if url.scheme() != "https"
            || url.host_str() != Some("huggingface.co")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
        {
            return Err(invalid());
        }
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        if !matches!(parts.len(), 2 | 4) || !parts.iter().all(|p| identifier(p))
            || ["datasets", "spaces", "collections"].contains(&parts[0])
            || (parts.len() == 4 && parts[2] != "tree")
            // Reject normalized path traversal as well as malformed URLs.
            || input.split('/').any(|p| matches!(p, "." | ".."))
        {
            return Err(invalid());
        }
        Ok(Self {
            repository: format!("{}/{}", parts[0], parts[1]),
            reference: parts.get(3).unwrap_or(&"main").to_string(),
        })
    }
    pub fn url(&self) -> String {
        format!(
            "https://huggingface.co/{}/tree/{}",
            self.repository, self.reference
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HubModel {
    pub repository: String,
    pub revision: String,
    pub access: ModelAccess,
    pub pipeline_tag: Option<String>,
    pub library_name: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub parameter_count: Option<u64>,
    #[serde(default)]
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub weight_size_gb: Option<f64>,
    pub weight_format: Option<String>,
    pub is_adapter: bool,
    pub config_json: String,
    pub card_excerpt: String,
    pub notes: Vec<String>,
}

#[async_trait]
pub trait HuggingFaceApi: Send + Sync {
    async fn model(&self, source: &ModelSource) -> Result<HubModel>;
}
pub struct HuggingFaceClient;
#[async_trait]
impl HuggingFaceApi for HuggingFaceClient {
    async fn model(&self, source: &ModelSource) -> Result<HubModel> {
        let http = http_client(30)?;
        let response = http
            .get(format!(
                "https://huggingface.co/api/models/{}/revision/{}?blobs=true",
                source.repository, source.reference
            ))
            .send()
            .await
            .map_err(|_| hub_error())?;
        if !response.status().is_success() {
            return Err(hub_error());
        }
        let bytes = bounded_body(response, 2 * 1024 * 1024).await?;
        let data: Value = serde_json::from_slice(&bytes).map_err(|_| hub_error())?;
        let mut model = parse_metadata(source, &data)?;
        for (name, limit) in [("config.json", 64 * 1024), ("README.md", 48 * 1024)] {
            let result = async {
                let response = http
                    .get(format!(
                        "https://huggingface.co/{}/raw/{}/{}",
                        model.repository, model.revision, name
                    ))
                    .send()
                    .await
                    .map_err(|_| hub_error())?;
                if !response.status().is_success() {
                    return Err(hub_error());
                }
                let bytes = bounded_body(response, limit).await?;
                String::from_utf8(bytes).map_err(|_| hub_error())
            }
            .await;
            match (name, result) {
                ("config.json", Ok(value))
                    if serde_json::from_str::<Value>(&value).is_ok_and(|v| v.is_object()) =>
                {
                    model.config_json = value
                }
                ("README.md", Ok(value)) => model.card_excerpt = value,
                _ => model.notes.push(format!(
                    "{name} was unavailable or exceeded the metadata size limit."
                )),
            }
        }
        Ok(model)
    }
}
pub fn parse_metadata(source: &ModelSource, data: &Value) -> Result<HubModel> {
    let revision = data["sha"]
        .as_str()
        .filter(|v| v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(hub_error)?;
    let repository = data["id"].as_str().unwrap_or(&source.repository);
    if repository.split('/').count() != 2 || !repository.split('/').all(identifier) {
        return Err(hub_error());
    }
    let files = data["siblings"].as_array();
    // Prefer safetensors; do not double-count a second serialization of the same weights.
    let has_safe = files.is_some_and(|files| {
        files.iter().any(|f| {
            f["rfilename"]
                .as_str()
                .is_some_and(|s| s.ends_with(".safetensors"))
        })
    });
    let weights: Vec<_> = files
        .into_iter()
        .flatten()
        .filter(|f| {
            f["rfilename"].as_str().is_some_and(|s| {
                if has_safe {
                    s.ends_with(".safetensors")
                } else {
                    s.ends_with(".bin")
                }
            })
        })
        .collect();
    let bytes: Option<u64> = if weights.is_empty() {
        None
    } else {
        weights.iter().try_fold(0_u64, |total, f| {
            total.checked_add(f["size"].as_u64().or_else(|| f["lfs"]["size"].as_u64())?)
        })
    };
    Ok(HubModel {
        repository: repository.into(),
        revision: revision.to_ascii_lowercase(),
        access: if data["private"] == true {
            ModelAccess::Private
        } else if data["gated"] == true || data["gated"].is_string() {
            ModelAccess::Gated
        } else {
            ModelAccess::Public
        },
        pipeline_tag: data["pipeline_tag"].as_str().map(str::to_owned),
        library_name: data["library_name"].as_str().map(str::to_owned),
        parameter_count: data["safetensors"]["total"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= 9_007_199_254_740_991),
        license: data["cardData"]["license"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.chars().take(200).collect()),
        tags: data["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .take(100)
            .map(|s| s.chars().take(200).collect())
            .collect(),
        weight_size_gb: bytes.map(|v| v as f64 / 1_000_000_000.0),
        weight_format: if weights.is_empty() {
            None
        } else {
            Some(if has_safe { "safetensors" } else { "pytorch" }.into())
        },
        is_adapter: files.is_some_and(|files| {
            files
                .iter()
                .any(|f| f["rfilename"] == "adapter_config.json")
        }),
        config_json: String::new(),
        card_excerpt: String::new(),
        notes: if bytes.is_none() {
            vec!["Weight sizes are incomplete; memory and disk sizing need extra caution.".into()]
        } else {
            vec![]
        },
    })
}
fn hub_error() -> AppError {
    AppError::new("hugging_face_metadata", "Hugging Face model metadata could not be read.", "Check the model URL and connection. URL import currently needs publicly readable metadata; private repositories require a manual catalog profile.")
}
