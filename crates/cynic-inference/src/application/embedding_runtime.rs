use std::sync::Arc;

use crate::adapters::embedding::http::EmbeddingBackend;
use crate::adapters::embedding::null::NullEmbedding;
use crate::ports::embedding_port::EmbeddingPort;
use crate::ports::inference_port::BackendPort;

const DEFAULT_REST_ADDR: &str = "127.0.0.1:3030";

#[derive(Debug, Clone)]
pub struct EmbeddingRuntimeConfig {
    pub explicit_url: Option<String>,
    pub rest_host: String,
    pub api_key: Option<String>,
    pub model: String,
}

impl EmbeddingRuntimeConfig {
    pub fn from_env() -> Self {
        let explicit_url = std::env::var("CYNIC_EMBED_URL").ok();
        let rest_host = std::env::var("CYNIC_REST_ADDR")
            .unwrap_or_else(|_| DEFAULT_REST_ADDR.into())
            .split(':')
            .next()
            .unwrap_or("127.0.0.1")
            .to_string();
        let api_key = cynic_secrets::env_var("sovereign-api-key", "SOVEREIGN_API_KEY");
        let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());

        Self {
            explicit_url,
            rest_host,
            api_key,
            model,
        }
    }
}

pub async fn build_initial_embedding_port(
    config: &EmbeddingRuntimeConfig,
) -> Arc<dyn EmbeddingPort> {
    if let Some(url) = &config.explicit_url {
        match EmbeddingBackend::new(url, config.api_key.clone(), &config.model) {
            Ok(backend) if backend.health().await.is_available() => Arc::new(backend),
            _ => Arc::new(NullEmbedding),
        }
    } else if let Some(backend) =
        EmbeddingBackend::discover(&config.rest_host, config.api_key.clone(), &config.model).await
    {
        Arc::new(backend)
    } else {
        Arc::new(NullEmbedding)
    }
}

pub async fn discover_embedding_backend(
    config: &EmbeddingRuntimeConfig,
) -> Option<EmbeddingBackend> {
    EmbeddingBackend::discover(&config.rest_host, config.api_key.clone(), &config.model).await
}
