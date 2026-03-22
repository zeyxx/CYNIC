//! Embedding adapter — calls any OpenAI-compatible /v1/embeddings endpoint.
//! Default target: local llama-server on :8081 with Qwen3-Embedding.

use crate::domain::embedding::{Embedding, EmbeddingError, EmbeddingPort};
use crate::domain::inference::{BackendPort, BackendStatus};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct EmbeddingBackend {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl EmbeddingBackend {
    pub fn new(base_url: &str, api_key: Option<String>, model: &str) -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model: model.to_string(),
        })
    }

    /// Build from environment variables.
    /// CYNIC_EMBED_URL overrides, else derives from CYNIC_REST_ADDR host + port 8081.
    pub fn from_env() -> Result<Self, reqwest::Error> {
        let base_url = std::env::var("CYNIC_EMBED_URL")
            .unwrap_or_else(|_| {
                let rest = std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "127.0.0.1:3030".into());
                let host = rest.split(':').next().unwrap_or("127.0.0.1");
                format!("http://{}:8081/v1", host)
            });
        let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
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
                    BackendStatus::Degraded { latency_ms: latency }
                } else {
                    BackendStatus::Healthy
                }
            }
            Ok(resp) => {
                tracing::warn!(status = %resp.status(), "embedding backend degraded");
                BackendStatus::Degraded { latency_ms: start.elapsed().as_millis() as f64 }
            }
            Err(e) => {
                tracing::error!(error = %e, "embedding backend unreachable — critical");
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

        let mut req = self.client.post(&url)
            .json(&req_body);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                EmbeddingError::Timeout { ms: start.elapsed().as_millis() as u64 }
            } else {
                EmbeddingError::Unreachable(e.to_string())
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(EmbeddingError::Protocol(format!("HTTP {}: {}", status, body)));
        }

        let parsed: EmbeddingResponse = resp.json().await
            .map_err(|e| EmbeddingError::Protocol(format!("JSON parse error: {}", e)))?;

        let data = parsed.data.into_iter().next()
            .ok_or_else(|| EmbeddingError::Protocol("empty response".into()))?;

        let dimensions = data.embedding.len();
        let prompt_tokens = parsed.usage.map(|u| u.prompt_tokens).unwrap_or(0);

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
        assert!(backend.base_url.contains("8081"));
        assert_eq!(backend.model, "qwen3-embed");
    }
}
