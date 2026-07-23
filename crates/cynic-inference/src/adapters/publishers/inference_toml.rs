//! InferenceTomlPublisher — persists discovered models into
//! /etc/cynic/inference.toml (the installed runtime SSOT for routing).
//!
//! Idempotent: skips backends already present by (endpoint, model_id).

use async_trait::async_trait;
use serde::Serialize;

use crate::config_loader::{LoadedInferenceConfig, load_inference_config, resolve_config_path};
use crate::domain::backend_config::{AuthStyle, BackendType, InferenceBackendConfig};
use crate::ports::model_publisher_port::{ModelPublisherPort, NormalisedModel, PublishResult};

#[derive(Debug)]
pub struct InferenceTomlPublisher;

#[async_trait]
impl ModelPublisherPort for InferenceTomlPublisher {
    fn name(&self) -> &str {
        "inference.toml"
    }

    async fn publish(&self, model: &NormalisedModel) -> PublishResult {
        let result = Self::upsert(model);
        PublishResult {
            publisher: self.name().to_string(),
            model_id: model.model_id.clone(),
            created: result.as_ref().map(|r| r.created).unwrap_or(false),
            error: result.err(),
        }
    }

    async fn list_registered(&self) -> Result<Vec<String>, String> {
        let loaded = load_inference_config().map_err(|e| e.to_string())?;
        Ok(loaded.backends.into_iter().map(|b| b.model).collect())
    }
}

struct UpsertOutcome {
    created: bool,
}

impl InferenceTomlPublisher {
    fn upsert(model: &NormalisedModel) -> Result<UpsertOutcome, String> {
        let config_path = resolve_config_path();

        let mut current_backends = match load_inference_config() {
            Ok(LoadedInferenceConfig { backends, .. }) => backends,
            _ => Vec::new(),
        };

        // Idempotency check — skip if (endpoint, model_id) already present
        let exists = current_backends
            .iter()
            .any(|b| b.base_url == model.endpoint && b.model == model.model_id);

        if exists {
            return Ok(UpsertOutcome { created: false });
        }

        let name = format!(
            "{}-{}",
            model.engine,
            model.model_id.replace([':', '.', '/'], "-")
        );

        current_backends.push(InferenceBackendConfig {
            name,
            backend_type: BackendType::OpenAi,
            base_url: model.endpoint.clone(),
            api_key: None,
            model: model.model_id.clone(),
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
            sovereign: model.sovereign,
            fleet_node: None,
        });

        #[derive(Serialize)]
        struct OutputConfig {
            backend: Vec<InferenceBackendConfig>,
        }

        let output = OutputConfig {
            backend: current_backends,
        };
        let toml_str =
            toml::to_string(&output).map_err(|e| format!("TOML serialization failed: {e}"))?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }

        std::fs::write(&config_path, toml_str)
            .map_err(|e| format!("Failed to write inference.toml: {e}"))?;

        Ok(UpsertOutcome { created: true })
    }
}
