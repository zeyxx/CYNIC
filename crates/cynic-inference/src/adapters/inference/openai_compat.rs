//! OpenAiCompatBackend — universal adapter for any OpenAI-compatible inference server.
//! One type, N instances (Gemini, HuggingFace, llama.cpp, vLLM, SGLang).

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::domain::backend_config::{AuthStyle, InferenceBackendConfig};
use crate::domain::chat::{ChatError, ChatResponse, InferenceProfile};
use crate::domain::inference::{
    BackendError, BackendInitError, BackendStatus, InferRequest, InferResponse,
};
use crate::ports::chat_port::ChatPort;
use crate::ports::inference_port::{BackendPort, InferPort};

#[derive(Debug)]
pub struct OpenAiCompatBackend {
    client: Client,
    config: InferenceBackendConfig,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_template_kwargs: Option<ChatTemplateKwargs>,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

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
    model: String,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: String,
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

#[derive(Deserialize)]
struct ModelsListResponse {
    #[serde(default)]
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

impl OpenAiCompatBackend {
    pub fn new(config: InferenceBackendConfig) -> Result<Self, BackendInitError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(BackendInitError::from_http)?;

        Ok(Self { client, config })
    }

    fn build_url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{base}{path}");

        match &self.config.auth_style {
            AuthStyle::QueryParam { param } => {
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
            AuthStyle::QueryParam { .. } | AuthStyle::None => req,
        }
    }

    async fn post_chat(
        &self,
        messages: Vec<Message>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        n: Option<u32>,
        disable_thinking: bool,
        request_id: Option<&str>,
    ) -> Result<ChatCompletionResponse, ChatError> {
        let url = self.build_url("/chat/completions");
        let response_format = if self.config.json_mode {
            Some(ResponseFormat {
                format_type: "json_object".to_string(),
            })
        } else {
            None
        };
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

        let mut req = self.client.post(&url).json(&body);
        req = self.apply_auth(req);

        if let Some(rid) = request_id {
            req = req.header("X-Request-Id", rid);
        }

        let resp = req.send().await.map_err(|error| {
            if error.is_timeout() {
                ChatError::Timeout {
                    ms: self.config.timeout_secs * 1000,
                }
            } else {
                ChatError::Unreachable(format!("{}: {}", self.config.name, error))
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

        resp.json().await.map_err(|error| {
            ChatError::Protocol(format!("{}: parse error: {}", self.config.name, error))
        })
    }
}

#[async_trait]
impl BackendPort for OpenAiCompatBackend {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn health(&self) -> BackendStatus {
        let start = Instant::now();
        let url = self.build_url("/models");
        let req = self.apply_auth(self.client.get(&url));

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as f64;

                if let Ok(models_resp) = resp.json::<ModelsListResponse>().await
                    && let Some(first_model) = models_resp.data.first()
                {
                    let discovered_model = &first_model.id;
                    if discovered_model != &self.config.model {
                        tracing::warn!(
                            backend = %self.config.name,
                            config_model = %self.config.model,
                            discovered_model = %discovered_model,
                            "model mismatch: config != reality"
                        );
                    } else {
                        tracing::info!(
                            backend = %self.config.name,
                            model = %discovered_model,
                            "model discovery verified"
                        );
                    }
                }

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
            Err(error) => {
                tracing::error!(backend = %self.config.name, error = %error, "backend unreachable — critical");
                BackendStatus::Critical
            }
        }
    }
}

fn should_disable(is_local: bool, config_disable: bool) -> bool {
    is_local && config_disable
}

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

#[async_trait]
impl ChatPort for OpenAiCompatBackend {
    async fn chat(
        &self,
        system: &str,
        user: &str,
        profile: InferenceProfile,
        request_id: Option<&str>,
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
                request_id,
            )
            .await?;

        let (prompt_tokens, completion_tokens) = resp
            .usage
            .map(|usage| (usage.prompt_tokens, usage.completion_tokens))
            .unwrap_or((0, 0));

        let (text, reasoning, thinking_chars) = resp
            .choices
            .into_iter()
            .next()
            .map(|choice| {
                let thinking_chars = choice
                    .message
                    .reasoning_content
                    .as_deref()
                    .map_or(0, |reasoning| reasoning.len());
                let content_chars = choice.message.content.len();
                let truncated = choice.finish_reason.as_deref() == Some("length");

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

                let content = choice.message.content;
                let reasoning = choice.message.reasoning_content.clone();

                if content.trim().is_empty()
                    && let Some((start, end)) = choice
                        .message
                        .reasoning_content
                        .as_deref()
                        .and_then(|reasoning| Some((reasoning.find('{')?, reasoning.rfind('}')?)))
                    && let Some(reasoning) = choice.message.reasoning_content.as_deref()
                {
                    return (
                        reasoning[start..=end].to_string(),
                        reasoning.to_string().into(),
                        thinking_chars,
                    );
                }
                if let Some(pos) = content.find("</think>") {
                    let pre = &content[..pos].trim_start_matches("<think>").trim();
                    let after = content[pos + 8..].trim_start();
                    if !after.is_empty() {
                        return (after.to_string(), Some(pre.to_string()), thinking_chars);
                    }
                }
                (content, reasoning, thinking_chars)
            })
            .ok_or_else(|| ChatError::Protocol(format!("{}: empty response", self.config.name)))?;

        let thinking_tokens = (thinking_chars as u32) / 4;

        Ok(ChatResponse {
            text,
            reasoning,
            prompt_tokens,
            completion_tokens,
            thinking_tokens,
        })
    }
}

#[async_trait]
impl InferPort for OpenAiCompatBackend {
    async fn infer(
        &self,
        request: &InferRequest,
        request_id: Option<&str>,
    ) -> Result<InferResponse, BackendError> {
        let mut messages = Vec::new();
        if let Some(system) = request
            .system
            .as_deref()
            .filter(|system| !system.is_empty())
        {
            messages.push(Message {
                role: "system".to_string(),
                content: system.to_string(),
            });
        }
        messages.push(Message {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let is_local_backend = self.config.health_url.is_some();
        let response = self
            .post_chat(
                messages,
                Some(request.temperature as f32),
                Some(request.max_tokens),
                None,
                should_disable(is_local_backend, self.config.disable_thinking),
                request_id,
            )
            .await
            .map_err(|error| chat_error_to_backend_error(&self.config.name, error))?;

        let (prompt_tokens, completion_tokens) = response
            .usage
            .map(|usage| (usage.prompt_tokens.into(), usage.completion_tokens.into()))
            .unwrap_or((0, 0));
        let text = response
            .choices
            .into_iter()
            .next()
            .map(|choice| {
                choice
                    .message
                    .content
                    .trim()
                    .to_string()
                    .if_empty_then(choice.message.reasoning_content.unwrap_or_default())
            })
            .unwrap_or_default();

        if text.is_empty() {
            return Err(BackendError::Protocol(format!(
                "{}: empty response",
                self.config.name
            )));
        }

        Ok(InferResponse {
            text,
            model: response.model,
            prompt_tokens,
            completion_tokens,
        })
    }
}

fn chat_error_to_backend_error(backend_id: &str, error: ChatError) -> BackendError {
    match error {
        ChatError::Timeout { ms } => BackendError::Timeout {
            backend_id: backend_id.to_string(),
            ms,
        },
        ChatError::Unreachable(message) => BackendError::Unreachable(message),
        ChatError::Protocol(message) | ChatError::RateLimited(message) => {
            BackendError::Protocol(message)
        }
    }
}

trait EmptyStringFallback {
    fn if_empty_then(self, fallback: String) -> String;
}

impl EmptyStringFallback for String {
    fn if_empty_then(self, fallback: String) -> String {
        if self.is_empty() {
            fallback.trim().to_string()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::backend_config::BackendType;

    #[test]
    fn build_url_bearer_no_query() {
        let backend = OpenAiCompatBackend::new(InferenceBackendConfig {
            name: "test".into(),
            backend_type: BackendType::OpenAi,
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
            supports_tool_use: false,
            health_url: None,
            cli_extra_args: vec![],
            sovereign: false,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn build_url_query_param() {
        let backend = OpenAiCompatBackend::new(InferenceBackendConfig {
            name: "test".into(),
            backend_type: BackendType::OpenAi,
            base_url: "https://api.example.com/v1".into(),
            api_key: Some("key123".into()),
            model: "gemini".into(),
            auth_style: AuthStyle::QueryParam {
                param: "key".into(),
            },
            context_size: 0,
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            supports_tool_use: false,
            health_url: None,
            cli_extra_args: vec![],
            sovereign: false,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "https://api.example.com/v1/chat/completions?key=key123"
        );
    }

    #[test]
    fn build_url_no_auth() {
        let backend = OpenAiCompatBackend::new(InferenceBackendConfig {
            name: "local".into(),
            backend_type: BackendType::OpenAi,
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
            supports_tool_use: false,
            health_url: None,
            cli_extra_args: vec![],
            sovereign: true,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "http://localhost:8080/v1/chat/completions"
        );
    }

    #[test]
    fn trailing_slash_handled() {
        let backend = OpenAiCompatBackend::new(InferenceBackendConfig {
            name: "test".into(),
            backend_type: BackendType::OpenAi,
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
            supports_tool_use: false,
            health_url: None,
            cli_extra_args: vec![],
            sovereign: false,
        })
        .unwrap();
        assert_eq!(
            backend.build_url("/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn thinking_disabled_when_config_says_so() {
        assert!(should_disable(true, true));
    }

    #[test]
    fn thinking_not_disabled_for_cloud_backend() {
        assert!(!should_disable(false, true));
    }

    #[test]
    fn thinking_active_when_config_says_so() {
        assert!(!should_disable(true, false));
    }

    #[test]
    fn thinking_active_uses_config_max_tokens() {
        assert_eq!(
            effective_max_tokens(false, 4096, InferenceProfile::SCORING),
            4096
        );
    }

    #[test]
    fn thinking_disabled_uses_profile_max_tokens() {
        assert_eq!(
            effective_max_tokens(true, 4096, InferenceProfile::SCORING),
            1024
        );
    }

    #[test]
    fn thinking_active_but_config_smaller_uses_profile() {
        assert_eq!(
            effective_max_tokens(false, 512, InferenceProfile::SCORING),
            1024
        );
    }

    #[test]
    fn agent_profile_unaffected_by_thinking() {
        assert_eq!(
            effective_max_tokens(false, 4096, InferenceProfile::AGENT),
            8192
        );
    }
}
