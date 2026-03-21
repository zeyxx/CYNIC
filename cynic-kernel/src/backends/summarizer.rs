//! Sovereign Summarizer — calls the local LLM's /v1/chat/completions
//! endpoint for session compression and direct inference. Implements both
//! SummarizationPort (background session summaries) and InferPort (cynic_infer).
//! Single adapter for all sovereign LLM interactions.
//!
//! HTTP logic lives in the inherent impl (call_completions). Both trait
//! implementations delegate to it — zero duplication.

use async_trait::async_trait;
use reqwest::Client;
use crate::domain::inference::{BackendError, InferPort, InferRequest, InferResponse};
use crate::domain::summarization::{SummarizationError, SummarizationPort};

pub struct SovereignSummarizer {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

/// Raw response from the LLM completions endpoint.
struct CompletionResult {
    text: String,
    model: String,
    prompt_tokens: u64,
    completion_tokens: u64,
}

impl SovereignSummarizer {
    /// Construct from env vars. Uses the sovereign LLM endpoint.
    /// Config sources: CYNIC_SUMMARIZER_URL > derived from CYNIC_REST_ADDR host + :8080.
    pub fn from_env() -> Self {
        let host = std::env::var("CYNIC_REST_ADDR")
            .unwrap_or_else(|_| "localhost:3030".into())
            .split(':').next().unwrap_or("localhost").to_string();
        let base_url = std::env::var("CYNIC_SUMMARIZER_URL")
            .unwrap_or_else(|_| format!("http://{}:8080/v1", host));
        let api_key = std::env::var("SOVEREIGN_API_KEY").ok()
            .or_else(|| {
                std::fs::read_to_string(
                    dirs::config_dir()
                        .unwrap_or_default()
                        .join("cynic/llama-api-key")
                ).ok().map(|s| s.trim().to_string())
            });
        let model = std::env::var("CYNIC_SUMMARIZER_MODEL")
            .unwrap_or_else(|_| "local".into());

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to build summarizer HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    /// Health check — can the LLM be reached?
    /// Concrete method on the struct, NOT on a port trait.
    /// Called by the composition root (main.rs) for operational readiness.
    /// The domain port (SummarizationPort) stays pure — no health concerns.
    pub async fn is_available(&self) -> bool {
        let health_url = self.base_url.replace("/v1", "/health");
        self.client.get(&health_url).send().await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Shared HTTP call to /v1/chat/completions — all trait impls delegate here.
    /// Handles auth, Qwen3 reasoning_content fallback, and response parsing.
    async fn call_completions(
        &self,
        messages: Vec<serde_json::Value>,
        temperature: f64,
        max_tokens: u32,
    ) -> Result<CompletionResult, BackendError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let mut req = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.json(&body).send().await
            .map_err(|e| {
                if e.is_timeout() { BackendError::Timeout { backend_id: "sovereign".into(), ms: 60_000 } }
                else { BackendError::Unreachable(e.to_string()) }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BackendError::Protocol(format!("HTTP {}: {}", status, text)));
        }

        let data: serde_json::Value = resp.json().await
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let message = &data["choices"][0]["message"];
        // Qwen3 reasoning models put output in reasoning_content with empty content.
        // Try content first, then fall back to reasoning_content.
        let text = message["content"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| message["reasoning_content"]
                .as_str()
                .filter(|s| !s.trim().is_empty()))
            .unwrap_or("")
            .trim()
            .to_string();

        let usage = &data["usage"];
        Ok(CompletionResult {
            text,
            model: data["model"].as_str().unwrap_or("local").to_string(),
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
        })
    }
}

#[async_trait]
impl SummarizationPort for SovereignSummarizer {
    async fn summarize(&self, prompt: &str) -> Result<String, SummarizationError> {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "You are a concise session summarizer. Output 2-3 sentences, no preamble."}),
            serde_json::json!({"role": "user", "content": prompt}),
        ];

        let result = self.call_completions(messages, 0.3, 512).await
            .map_err(|e| match e {
                BackendError::Timeout { .. } => SummarizationError::Timeout,
                BackendError::Unreachable(msg) => SummarizationError::Unreachable(msg),
                other => SummarizationError::Protocol(other.to_string()),
            })?;

        if result.text.is_empty() {
            return Err(SummarizationError::Protocol("Empty response from LLM".into()));
        }

        Ok(result.text)
    }
}

#[async_trait]
impl InferPort for SovereignSummarizer {
    async fn infer(&self, request: &InferRequest) -> Result<InferResponse, BackendError> {
        let mut messages = vec![
            serde_json::json!({"role": "user", "content": request.prompt}),
        ];
        if let Some(ref sys) = request.system {
            messages.insert(0, serde_json::json!({"role": "system", "content": sys}));
        }

        let result = self.call_completions(messages, request.temperature, request.max_tokens).await?;

        if result.text.is_empty() {
            return Err(BackendError::Protocol("Empty response from LLM".into()));
        }

        Ok(InferResponse {
            text: result.text,
            model: result.model,
            prompt_tokens: result.prompt_tokens,
            completion_tokens: result.completion_tokens,
        })
    }
}
