//! Embedding adapter — calls any OpenAI-compatible /v1/embeddings endpoint.
//! Default target: local llama-server on :8081 with Qwen3-Embedding.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::domain::embedding::{Embedding, EmbeddingError};
use crate::domain::inference::{BackendInitError, BackendStatus};
use crate::ports::embedding_port::EmbeddingPort;
use crate::ports::inference_port::BackendPort;

const DEFAULT_REST_ADDR: &str = "127.0.0.1:3030";

#[derive(Debug)]
pub struct EmbeddingBackend {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl EmbeddingBackend {
    pub fn new(
        base_url: &str,
        api_key: Option<String>,
        model: &str,
    ) -> Result<Self, BackendInitError> {
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(BackendInitError::from_http)?,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model: model.to_string(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn discover(host: &str, api_key: Option<String>, model: &str) -> Option<Self> {
        let ports = [
            std::env::var("CYNIC_EMBED_PORT").unwrap_or_else(|_| "8081".into()),
            "8080".into(),
        ];
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .ok()?;

        for port in &ports {
            let base_url = format!("http://{host}:{port}/v1");
            let health_url = format!("http://{host}:{port}/health");
            let mut req = client.get(&health_url);
            if let Some(ref key) = api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }
            match tokio::time::timeout(std::time::Duration::from_secs(3), req.send()).await {
                Ok(Ok(resp)) if resp.status().is_success() => {
                    tracing::info!(host, port, "embedding backend discovered");
                    return Self::new(&base_url, api_key, model)
                        .map_err(|error| {
                            tracing::warn!(host, port, error = %error, "embedding init failed")
                        })
                        .ok();
                }
                _ => {
                    tracing::debug!(host, port, "embedding port unreachable");
                    continue;
                }
            }
        }
        None
    }

    pub fn from_env() -> Result<Self, BackendInitError> {
        let base_url = std::env::var("CYNIC_EMBED_URL").unwrap_or_else(|_| {
            let rest =
                std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| DEFAULT_REST_ADDR.into());
            let host = rest.split(':').next().unwrap_or("127.0.0.1");
            let port = std::env::var("CYNIC_EMBED_PORT").unwrap_or_else(|_| "8081".into());
            format!("http://{host}:{port}/v1")
        });
<<<<<<< HEAD
        let api_key = cynic_secrets::env_var("sovereign-api-key", "SOVEREIGN_API_KEY");
=======
        let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
>>>>>>> github/main
        let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
        Self::new(&base_url, api_key, &model)
    }
}

#[async_trait]
impl BackendPort for EmbeddingBackend {
    fn name(&self) -> &str {
        &self.model
    }

    async fn health(&self) -> BackendStatus {
        let start = Instant::now();
        let url = self.base_url.replace("/v1", "/health");
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as f64;
                if latency > 2000.0 {
                    BackendStatus::Degraded {
                        latency_ms: latency,
                    }
                } else {
                    BackendStatus::Healthy
                }
            }
            Ok(resp) => {
                tracing::warn!(status = %resp.status(), "embedding backend degraded");
                BackendStatus::Degraded {
                    latency_ms: start.elapsed().as_millis() as f64,
                }
            }
            Err(error) => {
                tracing::error!(%error, "embedding backend unreachable");
                BackendStatus::Critical
            }
        }
    }
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    #[serde(default)]
    prompt_tokens: u32,
}

#[async_trait]
impl EmbeddingPort for EmbeddingBackend {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let url = format!("{}/embeddings", self.base_url);
        let start = Instant::now();

        let req_body = EmbeddingRequest {
            input: text.to_string(),
            model: self.model.clone(),
        };

        let mut req = self.client.post(&url).json(&req_body);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req.send().await.map_err(|error| {
            if error.is_timeout() {
                EmbeddingError::Timeout {
                    ms: start.elapsed().as_millis() as u64,
                }
            } else {
                EmbeddingError::Unreachable(error.to_string())
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(EmbeddingError::Protocol(format!("HTTP {status}: {body}")));
        }

        let parsed: EmbeddingResponse = resp
            .json()
            .await
            .map_err(|error| EmbeddingError::Protocol(format!("JSON parse error: {error}")))?;

        let data = parsed
            .data
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::Protocol("empty response".into()))?;

        let dimensions = data.embedding.len();
        let prompt_tokens = parsed.usage.map(|usage| usage.prompt_tokens).unwrap_or(0);

        Ok(Embedding {
            vector: data.embedding,
            dimensions,
            prompt_tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_defaults() {
        let backend = EmbeddingBackend::from_env().unwrap();
        assert!(backend.base_url().contains("8081"));
    }
}
