//! OpenAiCompatBackend — universal adapter for any OpenAI-compatible inference server.
//! Implements BackendPort (shared health+name) and ChatPort (for Dogs).
//! One type, N instances (Gemini, HuggingFace, llama.cpp, vLLM, SGLang).

use crate::domain::inference::{BackendPort, BackendStatus};
use crate::domain::chat::{ChatPort, ChatError, ChatResponse};

use crate::infra::config::{BackendConfig, AuthStyle};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct OpenAiCompatBackend {
    client: Client,
    config: BackendConfig,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    #[allow(dead_code)] // deserialized from API, may be used for routing/logging
    model: String,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: String,
    /// Thinking/reasoning content from models like Qwen3.5 with thinking mode.
    /// When reasoning_format != "none", the model puts thinking here and the
    /// actual response in `content`. If content is empty, we extract JSON from this.
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

impl OpenAiCompatBackend {
    /// Create a new backend from config. Does NOT health-check — call health() after.
    pub fn new(config: BackendConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self { client, config }
    }

    fn build_url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{}{}", base, path);

        match &self.config.auth_style {
            AuthStyle::QueryParam(param) => {
                if let Some(ref key) = self.config.api_key {
                    format!("{}?{}={}", url, param, key)
                } else {
                    url
                }
            }
            _ => url,
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.auth_style {
            AuthStyle::Bearer => {
                if let Some(ref key) = self.config.api_key {
                    req.header("Authorization", format!("Bearer {}", key))
                } else {
                    req
                }
            }
            // QueryParam auth is handled in build_url
            AuthStyle::QueryParam(_) | AuthStyle::None => req,
        }
    }

    async fn post_chat(&self, messages: Vec<Message>, temperature: Option<f32>, max_tokens: Option<u32>, n: Option<u32>) -> Result<ChatCompletionResponse, ChatError> {
        let url = self.build_url("/chat/completions");
        let body = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens,
            max_completion_tokens: None,
            n,
        };

        let req = self.client.post(&url).json(&body);
        let req = self.apply_auth(req);

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                ChatError::Timeout { ms: 30_000 }
            } else {
                ChatError::Unreachable(format!("{}: {}", self.config.name, e))
            }
        })?;

        if resp.status().as_u16() == 429 {
            return Err(ChatError::RateLimited(format!("{}: rate limited", self.config.name)));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChatError::Protocol(format!("{} HTTP {}: {}", self.config.name, status, text)));
        }

        resp.json().await
            .map_err(|e| ChatError::Protocol(format!("{}: parse error: {}", self.config.name, e)))
    }
}

// ── BackendPort implementation (shared health + name) ───────

#[async_trait]
impl BackendPort for OpenAiCompatBackend {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn health(&self) -> BackendStatus {
        let start = Instant::now();
        let url = self.build_url("/models");
        let req = self.client.get(&url);
        let req = self.apply_auth(req);

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as f64;
                if latency > 2000.0 {
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

// ── ChatPort implementation (for Dogs) ──────────────────────

#[async_trait]
impl ChatPort for OpenAiCompatBackend {
    async fn chat(&self, system: &str, user: &str) -> Result<ChatResponse, ChatError> {
        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(Message { role: "system".to_string(), content: system.to_string() });
        }
        messages.push(Message { role: "user".to_string(), content: user.to_string() });

        let resp = self.post_chat(messages, Some(0.3), Some(4096), None).await?;

        let (prompt_tokens, completion_tokens) = resp.usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        let text = resp.choices.into_iter().next()
            .map(|c| {
                let content = c.message.content;
                // If content is empty but reasoning_content has JSON (thinking models like Qwen3.5),
                // extract the JSON object from reasoning_content as fallback.
                if content.trim().is_empty()
                    && let Some((start, end)) = c.message.reasoning_content
                        .as_deref()
                        .and_then(|r| Some((r.find('{')?, r.rfind('}')?)))
                {
                    let r = c.message.reasoning_content.as_deref().unwrap();
                    return r[start..=end].to_string();
                }
                // If content has a </think> preamble (reasoning-format=none), strip it.
                if let Some(pos) = content.find("</think>") {
                    let after = content[pos + 8..].trim_start();
                    if !after.is_empty() {
                        return after.to_string();
                    }
                }
                content
            })
            .ok_or_else(|| ChatError::Protocol(format!("{}: empty response", self.config.name)))?;

        Ok(ChatResponse { text, prompt_tokens, completion_tokens })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_bearer_no_query() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "test".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key: Some("sk-123".into()),
            model: "gpt-4".into(),
            auth_style: AuthStyle::Bearer,
            context_size: 0,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: String::new(),
            remediation: None,
        });
        assert_eq!(backend.build_url("/chat/completions"), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn build_url_query_param() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "test".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key: Some("key123".into()),
            model: "gemini".into(),
            auth_style: AuthStyle::QueryParam("key".into()),
            context_size: 0,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: String::new(),
            remediation: None,
        });
        assert_eq!(backend.build_url("/chat/completions"), "https://api.example.com/v1/chat/completions?key=key123");
    }

    #[test]
    fn build_url_no_auth() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "local".into(),
            base_url: "http://localhost:8080/v1".into(),
            api_key: None,
            model: "phi-3".into(),
            auth_style: AuthStyle::None,
            context_size: 0,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: String::new(),
            remediation: None,
        });
        assert_eq!(backend.build_url("/chat/completions"), "http://localhost:8080/v1/chat/completions");
    }

    #[test]
    fn trailing_slash_handled() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "test".into(),
            base_url: "https://api.example.com/v1/".into(),
            api_key: None,
            model: "m".into(),
            auth_style: AuthStyle::None,
            context_size: 0,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: String::new(),
            remediation: None,
        });
        assert_eq!(backend.build_url("/chat/completions"), "https://api.example.com/v1/chat/completions");
    }
}
