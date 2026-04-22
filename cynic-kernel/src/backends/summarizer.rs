//! Sovereign Summarizer — calls the local LLM's /v1/chat/completions
//! endpoint for session compression and direct inference. Implements both
//! SummarizationPort (background session summaries) and InferPort (cynic_infer).
//! Single adapter for all sovereign LLM interactions.
//!
//! HTTP logic lives in the inherent impl (call_completions). Both trait
//! implementations delegate to it — zero duplication.

use crate::domain::inference::{BackendError, InferPort, InferRequest, InferResponse};
use crate::domain::summarization::{SummarizationError, SummarizationPort};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

#[derive(Debug)]
pub struct SovereignSummarizer {
    client: Client,
    backend_id: String,
    base_url: String,
    api_key: Option<String>,
    model: String,
    timeout_ms: u64,
}

/// Raw response from the LLM completions endpoint.
struct CompletionResult {
    text: String,
    model: String,
    prompt_tokens: u64,
    completion_tokens: u64,
}

impl SovereignSummarizer {
    fn new(
        backend_id: &str,
        base_url: &str,
        api_key: Option<String>,
        model: &str,
        timeout_secs: u64,
    ) -> Result<Self, crate::domain::inference::BackendInitError> {
        let timeout = Duration::from_secs(timeout_secs);
        Ok(Self {
            client: Client::builder()
                .timeout(timeout)
                .build()
                .map_err(crate::domain::inference::BackendInitError::from_http)?,
            backend_id: backend_id.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model: model.to_string(),
            timeout_ms: timeout.as_millis() as u64,
        })
    }

    /// Construct from env vars. Uses the sovereign LLM endpoint.
    /// Config sources: CYNIC_SUMMARIZER_URL > derived from CYNIC_REST_ADDR host + :8080.
    pub fn from_env() -> Result<Self, crate::domain::inference::BackendInitError> {
        let host = std::env::var("CYNIC_REST_ADDR")
            .unwrap_or_else(|_| crate::domain::constants::DEFAULT_REST_ADDR.into())
            .split(':')
            .next()
            .unwrap_or("127.0.0.1")
            .to_string();
        let base_url = std::env::var("CYNIC_SUMMARIZER_URL")
            .unwrap_or_else(|_| format!("http://{host}:8080/v1"));
        let api_key = std::env::var("SOVEREIGN_API_KEY").ok()
            .or_else(|| {
                let path = dirs::config_dir()
                    .unwrap_or_default()
                    .join("cynic/llama-api-key");
                match std::fs::read_to_string(&path) {
                    Ok(s) => Some(s.trim().to_string()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "llama API key file unreadable (permissions?)");
                        None
                    }
                }
            });
        let model = std::env::var("CYNIC_SUMMARIZER_MODEL").unwrap_or_else(|_| "local".into());
        let backend_id =
            std::env::var("CYNIC_SUMMARIZER_BACKEND").unwrap_or_else(|_| "sovereign".into());
        let timeout_secs = std::env::var("CYNIC_SUMMARIZER_TIMEOUT_SECS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .filter(|secs| *secs > 0)
            .unwrap_or(60);

        Self::new(&backend_id, &base_url, api_key, &model, timeout_secs)
    }

    /// Construct from the canonical backend config loaded from backends.toml.
    pub fn from_backend_config(
        cfg: &crate::infra::config::BackendConfig,
    ) -> Result<Self, crate::domain::inference::BackendInitError> {
        Self::new(
            &cfg.name,
            &cfg.base_url,
            cfg.api_key.clone(),
            &cfg.model,
            cfg.timeout_secs,
        )
    }

    /// Health check — can the LLM be reached?
    /// Concrete method on the struct, NOT on a port trait.
    /// Called by the composition root (main.rs) for operational readiness.
    /// The domain port (SummarizationPort) stays pure — no health concerns.
    pub async fn is_available(&self) -> bool {
        let health_url = self.base_url.replace("/v1", "/health");
        self.client
            .get(&health_url)
            .send()
            .await
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
        request_id: Option<&str>,
    ) -> Result<CompletionResult, BackendError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        if let Some(rid) = request_id {
            req = req.header("X-Request-Id", rid);
        }

        let resp = req.json(&body).send().await.map_err(|e| {
            if e.is_timeout() {
                BackendError::Timeout {
                    backend_id: self.backend_id.clone(),
                    ms: self.timeout_ms,
                }
            } else {
                BackendError::Unreachable(e.to_string())
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BackendError::Protocol(format!("HTTP {status}: {text}")));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let message = &data["choices"][0]["message"];
        // Qwen3 reasoning models put output in reasoning_content with empty content.
        // Try content first, then fall back to reasoning_content.
        let text = message["content"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                message["reasoning_content"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
            })
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

        let result = self
            .call_completions(messages, 0.3, 512, None)
            .await
            .map_err(|e| match e {
                BackendError::Timeout { .. } => SummarizationError::Timeout,
                BackendError::Unreachable(msg) => SummarizationError::Unreachable(msg),
                other => SummarizationError::Protocol(other.to_string()),
            })?;

        if result.text.is_empty() {
            return Err(SummarizationError::Protocol(
                "Empty response from LLM".into(),
            ));
        }

        Ok(result.text)
    }
}

#[async_trait]
impl InferPort for SovereignSummarizer {
    async fn infer(
        &self,
        request: &InferRequest,
        request_id: Option<&str>,
    ) -> Result<InferResponse, BackendError> {
        let mut messages = vec![serde_json::json!({"role": "user", "content": request.prompt})];
        if let Some(ref sys) = request.system {
            messages.insert(0, serde_json::json!({"role": "system", "content": sys}));
        }

        let result = self
            .call_completions(
                messages,
                request.temperature,
                request.max_tokens,
                request_id,
            )
            .await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::{AuthStyle, BackendConfig, BackendType};

    #[test]
    fn backend_config_constructor_uses_runtime_timeout_and_backend_id() {
        let cfg = BackendConfig {
            name: "qwen35-9b-gpu".into(),
            backend_type: BackendType::OpenAi,
            base_url: "http://127.0.0.1:8080/v1".into(),
            api_key: Some("secret".into()),
            model: "Qwen3.5-9B".into(),
            auth_style: AuthStyle::Bearer,
            context_size: 32768,
            timeout_secs: 120,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            prompt_tier: crate::infra::config::PromptTier::Full,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: Some("http://127.0.0.1:8080/health".into()),
            remediation: None,
        };

        let summarizer = SovereignSummarizer::from_backend_config(&cfg).expect("init summarizer");
        assert_eq!(summarizer.backend_id, "qwen35-9b-gpu");
        assert_eq!(summarizer.model, "Qwen3.5-9B");
        assert_eq!(summarizer.timeout_ms, 120_000);
    }
}
