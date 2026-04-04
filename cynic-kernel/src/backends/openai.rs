//! OpenAiCompatBackend — universal adapter for any OpenAI-compatible inference server.
//! Implements BackendPort (shared health+name) and ChatPort (for Dogs).
//! One type, N instances (Gemini, HuggingFace, llama.cpp, vLLM, SGLang).

use crate::domain::chat::{ChatError, ChatPort, ChatResponse, InferenceProfile};
use crate::domain::inference::{BackendPort, BackendStatus};

use crate::infra::config::{AuthStyle, BackendConfig};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    /// llama-server: pass enable_thinking to the chat template.
    /// `{"enable_thinking": false}` disables Qwen3/3.5 thinking mode at the template level.
    /// Silently ignored by non-llama backends (Gemini, HuggingFace).
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_template_kwargs: Option<ChatTemplateKwargs>,
}

/// OpenAI-compatible response_format field.
/// `{"type": "json_object"}` forces the model to output valid JSON.
#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// llama-server chat template kwargs — controls thinking mode for Qwen3 family.
#[derive(Serialize)]
struct ChatTemplateKwargs {
    enable_thinking: bool,
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
    #[allow(dead_code)]
    // WHY: `model` is deserialized from the OpenAI API response (serde contract) but not yet
    // consumed by callers. Removing the field would silently break deserialization if the
    // struct is ever used where the JSON includes it with strict mode; keeping it documents
    // intent and enables future routing/logging without a breaking change.
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
    #[serde(default)]
    finish_reason: Option<String>,
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
    pub fn new(config: BackendConfig) -> Result<Self, crate::domain::inference::BackendInitError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(crate::domain::inference::BackendInitError::from_http)?;

        Ok(Self { client, config })
    }

    fn build_url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{base}{path}");

        match &self.config.auth_style {
            AuthStyle::QueryParam(param) => {
                if let Some(ref key) = self.config.api_key {
                    format!("{url}?{param}={key}")
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
                    req.header("Authorization", format!("Bearer {key}"))
                } else {
                    req
                }
            }
            // QueryParam auth is handled in build_url
            AuthStyle::QueryParam(_) | AuthStyle::None => req,
        }
    }

    async fn post_chat(
        &self,
        messages: Vec<Message>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        n: Option<u32>,
        disable_thinking: bool,
    ) -> Result<ChatCompletionResponse, ChatError> {
        let url = self.build_url("/chat/completions");
        let response_format = if self.config.json_mode {
            Some(ResponseFormat {
                format_type: "json_object".to_string(),
            })
        } else {
            None
        };
        // For thinking-capable models (Qwen3/3.5), disable thinking at the template level.
        // This is a llama-server feature — silently ignored by cloud APIs.
        let chat_template_kwargs = if disable_thinking {
            Some(ChatTemplateKwargs {
                enable_thinking: false,
            })
        } else {
            None
        };
        let body = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens,
            max_completion_tokens: None,
            n,
            response_format,
            chat_template_kwargs,
        };

        let req = self.client.post(&url).json(&body);
        let req = self.apply_auth(req);

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                ChatError::Timeout {
                    ms: self.config.timeout_secs * 1000,
                }
            } else {
                ChatError::Unreachable(format!("{}: {}", self.config.name, e))
            }
        })?;

        if resp.status().as_u16() == 429 {
            return Err(ChatError::RateLimited(format!(
                "{}: rate limited",
                self.config.name
            )));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChatError::Protocol(format!(
                "{} HTTP {}: {}",
                self.config.name, status, text
            )));
        }

        resp.json()
            .await
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
                    BackendStatus::Degraded {
                        latency_ms: latency,
                    }
                } else {
                    BackendStatus::Healthy
                }
            }
            Ok(resp) => {
                tracing::warn!(backend = %self.config.name, status = %resp.status(), "backend degraded");
                BackendStatus::Degraded {
                    latency_ms: start.elapsed().as_millis() as f64,
                }
            }
            Err(e) => {
                tracing::error!(backend = %self.config.name, error = %e, "backend unreachable — critical");
                BackendStatus::Critical
            }
        }
    }
}

// ── Inference parameter resolution (pure, testable) ─────────

/// Backend config controls thinking. Profile has no opinion — thinking requirements
/// are model-specific (R14: Qwen3.5 needs thinking for scoring).
/// Only for local backends — cloud APIs reject unknown fields.
fn should_disable(is_local: bool, config_disable: bool) -> bool {
    is_local && config_disable
}

/// When thinking is active, the model spends ~2000-6000 tokens on chain-of-thought
/// before writing the JSON response (~300 tokens). Profile's max_tokens (1024 for
/// Scoring) is too small — thinking overflows it. Use the backend's configured
/// max_tokens as ceiling when thinking is active.
fn effective_max_tokens(
    should_disable_thinking: bool,
    config_max_tokens: u32,
    profile: InferenceProfile,
) -> u32 {
    if !should_disable_thinking && config_max_tokens > profile.max_tokens() {
        config_max_tokens
    } else {
        profile.max_tokens()
    }
}

// ── ChatPort implementation (for Dogs) ──────────────────────

#[async_trait]
impl ChatPort for OpenAiCompatBackend {
    async fn chat(
        &self,
        system: &str,
        user: &str,
        profile: InferenceProfile,
    ) -> Result<ChatResponse, ChatError> {
        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(Message {
                role: "system".to_string(),
                content: system.to_string(),
            });
        }
        messages.push(Message {
            role: "user".to_string(),
            content: user.to_string(),
        });

        let is_local_backend = self.config.health_url.is_some();
        let should_disable_thinking =
            should_disable(is_local_backend, self.config.disable_thinking);
        let temperature = profile.temperature().unwrap_or(self.config.temperature);
        let max_tokens =
            effective_max_tokens(should_disable_thinking, self.config.max_tokens, profile);

        let resp = self
            .post_chat(
                messages,
                Some(temperature),
                Some(max_tokens),
                None,
                should_disable_thinking,
            )
            .await?;

        let (prompt_tokens, completion_tokens) = resp
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        let text = resp
            .choices
            .into_iter()
            .next()
            .map(|c| {
                let thinking_chars = c
                    .message
                    .reasoning_content
                    .as_deref()
                    .map_or(0, |r| r.len());
                let content_chars = c.message.content.len();
                let truncated = c.finish_reason.as_deref() == Some("length");

                if thinking_chars > 0 || truncated {
                    tracing::info!(
                        backend = %self.config.name,
                        thinking_chars,
                        content_chars,
                        completion_tokens,
                        truncated,
                        "token budget split"
                    );
                }
                if truncated && content_chars == 0 {
                    tracing::warn!(
                        backend = %self.config.name,
                        thinking_chars,
                        max_tokens,
                        "thinking overflow — content empty, increase max_tokens"
                    );
                }

                let content = c.message.content;
                // If content is empty but reasoning_content has JSON (thinking models like Qwen3.5),
                // extract the JSON object from reasoning_content as fallback.
                if content.trim().is_empty()
                    && let Some((start, end)) = c
                        .message
                        .reasoning_content
                        .as_deref()
                        .and_then(|r| Some((r.find('{')?, r.rfind('}')?)))
                    && let Some(r) = c.message.reasoning_content.as_deref()
                {
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

        Ok(ChatResponse {
            text,
            prompt_tokens,
            completion_tokens,
        })
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
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None,
            remediation: None,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
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
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None,
            remediation: None,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "https://api.example.com/v1/chat/completions?key=key123"
        );
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
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None,
            remediation: None,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "http://localhost:8080/v1/chat/completions"
        );
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
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None,
            remediation: None,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    // ── should_disable tests ────────────────────────────────

    #[test]
    fn thinking_disabled_when_config_says_so() {
        assert!(should_disable(true, true));
    }

    #[test]
    fn thinking_not_disabled_for_cloud_backend() {
        // Cloud backends never send chat_template_kwargs
        assert!(!should_disable(false, true));
    }

    #[test]
    fn thinking_active_when_config_says_so() {
        // Qwen 3.5: config says thinking required (R14)
        assert!(!should_disable(true, false));
    }

    // ── effective_max_tokens tests ──────────��───────────────

    #[test]
    fn thinking_active_uses_config_max_tokens() {
        // Qwen 3.5 with thinking ON: config says 4096, profile says 1024
        assert_eq!(
            effective_max_tokens(false, 4096, InferenceProfile::Scoring),
            4096
        );
    }

    #[test]
    fn thinking_disabled_uses_profile_max_tokens() {
        // Thinking OFF: profile controls the budget
        assert_eq!(
            effective_max_tokens(true, 4096, InferenceProfile::Scoring),
            1024
        );
    }

    #[test]
    fn thinking_active_but_config_smaller_uses_profile() {
        // Config max_tokens < profile max_tokens: profile wins
        assert_eq!(
            effective_max_tokens(false, 512, InferenceProfile::Scoring),
            1024
        );
    }

    #[test]
    fn agent_profile_unaffected_by_thinking() {
        // Agent already has 8192 — config 4096 is smaller, profile wins
        assert_eq!(
            effective_max_tokens(false, 4096, InferenceProfile::Agent),
            8192
        );
    }
}
