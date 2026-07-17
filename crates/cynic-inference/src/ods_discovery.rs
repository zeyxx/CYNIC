//! Probe local OpenAI-compatible engines for **CYNIC ↔ ODS integration**.
//!
//! [Osmantic ODS](https://github.com/Osmantic/ODS) is an external, maintained
//! framework used to bootstrap a full local AI environment. CYNIC does not own
//! ODS: this module only discovers endpoints ODS (or peers) already expose and
//! can merge them into CYNIC routing SSOT (`inference.toml`).
//!
//! Prefer `crate::adapters::ods` at call sites. Native vault lifecycle lives in
//! `adapters::local` — separate surface.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config_loader::{LoadedInferenceConfig, load_inference_config, resolve_config_path};
use crate::domain::backend_config::{AuthStyle, BackendType, InferenceBackendConfig};
use crate::ports::model_publisher_port::PublishResult;

/// Status report of a discovered ODS service.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub endpoint: String,
    pub status: String,
    pub models: Vec<String>,
}

/// Discovered backends view for the CLI.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub services: Vec<DiscoveredService>,
    pub backends_created: usize,
    pub config_updated: bool,
    pub config_path: PathBuf,
}

/// Scan host-facing ODS / OpenAI-compat endpoints and return a report.
///
/// Defaults match [Osmantic ODS](https://github.com/Osmantic/ODS) host API:
/// Linux Docker often exposes llama-server on **:11434**; native paths use **:8080**.
/// Override with `CYNIC_ODS_PROBE_URLS` (comma-separated base URLs).
pub async fn discover_ods_services() -> Vec<DiscoveredService> {
    let mut services = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    let mut seen_endpoints = std::collections::BTreeSet::<String>::new();

    for base in crate::domain::model_layout::default_ods_probe_bases() {
        let base = base.trim_end_matches('/').to_string();
        // Prefer OpenAI-compat /v1/models (ODS llama-server, many peers)
        if let Some(s) = probe_openai_compat(&client, &base).await {
            if seen_endpoints.insert(s.endpoint.clone()) {
                services.push(s);
            }
            continue;
        }
        // Ollama native tags API (legacy / non-ODS peers on 11434)
        if let Some(s) = probe_ollama(&client, &base).await {
            if seen_endpoints.insert(s.endpoint.clone()) {
                services.push(s);
            }
        }
    }

    services
}

async fn probe_ollama(client: &reqwest::Client, base_url: &str) -> Option<DiscoveredService> {
    let tags_url = format!("{base_url}/api/tags");
    let resp = client.get(&tags_url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    #[derive(Deserialize)]
    struct OllamaModel {
        name: String,
    }
    #[derive(Deserialize)]
    struct OllamaTags {
        models: Vec<OllamaModel>,
    }

    let tags = resp.json::<OllamaTags>().await.ok()?;
    let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();

    Some(DiscoveredService {
        name: "ollama-compatible".to_string(),
        endpoint: format!("{base_url}/v1"),
        status: "running".to_string(),
        models,
    })
}

/// Probe OpenAI-compatible `/v1/models` (ODS llama-server host API, native llama-server).
async fn probe_openai_compat(
    client: &reqwest::Client,
    base_url: &str,
) -> Option<DiscoveredService> {
    // Accept base with or without /v1
    let models_url = if base_url.ends_with("/v1") {
        format!("{base_url}/models")
    } else {
        format!("{base_url}/v1/models")
    };
    let resp = client.get(&models_url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    #[derive(Deserialize)]
    struct LlamaModel {
        id: String,
    }
    #[derive(Deserialize)]
    struct LlamaModelsList {
        data: Vec<LlamaModel>,
    }

    let list = resp.json::<LlamaModelsList>().await.ok()?;
    let models: Vec<String> = list.data.into_iter().map(|m| m.id).collect();

    let endpoint = if base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{base_url}/v1")
    };

    // Label by port heuristics for operator clarity
    let name = if endpoint.contains(":11434") {
        "ods-llama-server".to_string()
    } else if endpoint.contains(":8080") {
        "llama-server".to_string()
    } else {
        "openai-compat".to_string()
    };

    Some(DiscoveredService {
        name,
        endpoint,
        status: "running".to_string(),
        models,
    })
}

/// Merge discovered ODS services into the canonical inference.toml config file.
/// If `inference.toml` does not exist, it will be created.
pub async fn sync_discovered_backends() -> Result<DiscoveryReport, String> {
    let services = discover_ods_services().await;
    let config_path = resolve_config_path();

    // Load existing backends if possible
    let mut current_backends = match load_inference_config() {
        Ok(LoadedInferenceConfig { backends, .. }) => backends,
        _ => Vec::new(),
    };

    let mut backends_created = 0;
    let mut updated = false;

    for service in &services {
        for model in &service.models {
            let backend_name = format!(
                "{}-{}",
                service.name.to_lowercase(),
                model.replace(':', "-").replace('.', "-")
            );

            // Avoid duplicate name or model on the same endpoint
            let exists = current_backends.iter().any(|b| {
                b.name == backend_name || (b.base_url == service.endpoint && b.model == *model)
            });

            if !exists {
                current_backends.push(InferenceBackendConfig {
                    name: backend_name,
                    backend_type: BackendType::OpenAi,
                    base_url: service.endpoint.clone(),
                    api_key: None,
                    model: model.clone(),
                    auth_style: AuthStyle::None,
                    context_size: 8192,
                    timeout_secs: 120,
                    max_tokens: 4096,
                    temperature: 0.7,
                    disable_thinking: false,
                    json_mode: false,
                    supports_tool_use: false,
                    health_url: None,
                    cli_extra_args: vec![],
                    sovereign: true,
                });
                backends_created += 1;
                updated = true;
            }
        }
    }

    if updated {
        // Build the TOML document manually or serialise it
        #[derive(Serialize)]
        struct OutputConfig {
            backend: Vec<InferenceBackendConfig>,
        }

        let output = OutputConfig {
            backend: current_backends,
        };

        let toml_str =
            toml::to_string(&output).map_err(|e| format!("Failed to serialize TOML: {e}"))?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }

        std::fs::write(&config_path, toml_str)
            .map_err(|e| format!("Failed to write config file: {e}"))?;
    }

    Ok(DiscoveryReport {
        services,
        backends_created,
        config_updated: updated,
        config_path,
    })
}

/// Helper that instantiates the agnostic ModelSyncPipeline with TOML and Open WebUI targets,
/// and returns the collective results.
pub async fn run_sync_pipeline(node_id: &str) -> Vec<PublishResult> {
    use crate::application::model_sync_pipeline::ModelSyncPipeline;
    use crate::adapters::publishers::inference_toml::InferenceTomlPublisher;
    use crate::adapters::publishers::open_webui::OpenWebUiPublisher;

    let pipeline = ModelSyncPipeline::new(node_id.to_string())
        .with_publisher(std::sync::Arc::new(InferenceTomlPublisher))
        .with_publisher(std::sync::Arc::new(OpenWebUiPublisher::new()));

    pipeline.run_sync().await
}

