//! LlamaCppBackend — InferencePort implementation for llama.cpp's OpenAI-compatible HTTP API.

use crate::domain::inference::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Calls llama.cpp's /v1/chat/completions endpoint.
pub struct LlamaCppBackend {
    endpoint: String,
    client: Client,
    capability: BackendCapability,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    n: u32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    model: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

impl LlamaCppBackend {
    /// Connect to a llama-server endpoint. Polls /health with backoff until ready.
    pub async fn connect(endpoint: &str, backend_id: &str) -> Result<Self, BackendError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        // Cold-start polling: retry /health with exponential backoff
        let mut delay = std::time::Duration::from_secs(1);
        let deadline = Instant::now() + std::time::Duration::from_secs(60);
        loop {
            match client.get(format!("{}/health", endpoint)).send().await {
                Ok(resp) if resp.status().is_success() => break,
                _ if Instant::now() + delay < deadline => {
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, std::time::Duration::from_secs(16));
                }
                _ => return Err(BackendError::Unreachable(backend_id.to_string())),
            }
        }

        // Discover loaded models
        let models: ModelsResponse = client
            .get(format!("{}/v1/models", endpoint))
            .send()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?
            .json()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let loaded_models: Vec<String> = models.data.iter().map(|m| m.id.clone()).collect();

        Ok(Self {
            endpoint: endpoint.to_string(),
            client,
            capability: BackendCapability {
                id: backend_id.to_string(),
                kind: BackendKind::Local,
                device_name: "llama.cpp".to_string(),
                vram_total_gb: 0.0,
                vram_available_gb: 0.0,
                latency_ms: 0.0,
                loaded_models,
            },
        })
    }
}

#[async_trait]
impl InferencePort for LlamaCppBackend {
    fn capability(&self) -> &BackendCapability {
        &self.capability
    }

    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        let start = Instant::now();

        let mut messages = Vec::new();
        if !req.system_prompt.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: req.system_prompt.clone(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: req.context.clone(),
        });

        let model = req.model_hint.clone()
            .unwrap_or_else(|| {
                self.capability.loaded_models.first()
                    .cloned()
                    .unwrap_or_default()
            });

        let chat_req = ChatRequest {
            model: model.clone(),
            messages,
            temperature: req.temperature,
            n: req.num_branches.max(1),
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.endpoint))
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BackendError::Timeout {
                        backend_id: self.capability.id.clone(),
                        ms: start.elapsed().as_millis() as u64,
                    }
                } else {
                    BackendError::Unreachable(format!("{}: {}", self.capability.id, e))
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BackendError::Protocol(format!("HTTP {}: {}", status, body)));
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let hypotheses: Vec<String> = chat_resp
            .choices
            .into_iter()
            .map(|c| c.message.content)
            .collect();

        Ok(InferenceResponse {
            trace_id: req.trace_id,
            hypotheses,
            latency_ms: start.elapsed().as_millis() as f64,
            model_used: chat_resp.model,
            backend_id: self.capability.id.clone(),
        })
    }

    async fn health(&self) -> BackendStatus {
        let start = Instant::now();
        match self.client
            .get(format!("{}/health", self.endpoint))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as f64;
                if latency > 1000.0 {
                    BackendStatus::Degraded { latency_ms: latency }
                } else {
                    BackendStatus::Healthy
                }
            }
            Ok(_) => BackendStatus::Degraded { latency_ms: start.elapsed().as_millis() as f64 },
            Err(_) => BackendStatus::Critical,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_to_unreachable_fails() {
        let result = LlamaCppBackend::connect("http://127.0.0.1:99999", "bad").await;
        assert!(result.is_err());
    }
}
