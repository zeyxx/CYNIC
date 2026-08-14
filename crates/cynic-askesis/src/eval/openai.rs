use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::AskesisError;
use crate::eval::port::CompletionPort;
use crate::eval::types::{CompletionRequest, ModelCompletion, TokenUsage, ToolCall};

#[derive(Debug)]
pub struct OpenAiCompletionClient {
    client: Client,
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAiCompletionClient {
    pub fn new(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> crate::Result<Self> {
        let endpoint = endpoint.into().trim_end_matches('/').to_string();
        let model = model.into();
        if endpoint.is_empty() || model.trim().is_empty() {
            return Err(AskesisError::EvalBackend(
                "endpoint and model must be non-empty".into(),
            ));
        }
        let client = Client::builder()
            .build()
            .map_err(|error| AskesisError::EvalBackend(error.to_string()))?;
        Ok(Self {
            client,
            endpoint,
            model,
            api_key,
        })
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<serde_json::Value>,
    stream: bool,
    max_tokens: u32,
    temperature: f32,
    top_p: f32,
    top_k: u32,
    min_p: f32,
    repeat_penalty: f32,
    presence_penalty: f32,
    frequency_penalty: f32,
    seed: i64,
    timings_per_token: bool,
    chat_template_kwargs: ChatTemplateKwargs,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct ChatTemplateKwargs {
    enable_thinking: bool,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    #[serde(rename = "type")]
    kind: &'static str,
    function: ApiFunction,
}

#[derive(Debug, Serialize)]
struct ApiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    timings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    finish_reason: Option<String>,
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ApiToolCall>,
}

#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: String,
    function: ApiCalledFunction,
}

#[derive(Debug, Deserialize)]
struct ApiCalledFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

#[async_trait]
impl CompletionPort for OpenAiCompletionClient {
    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: &CompletionRequest) -> crate::Result<ModelCompletion> {
        let policy = &request.policy;
        let tools: Vec<ApiTool> = request
            .tools
            .iter()
            .map(|tool| ApiTool {
                kind: "function",
                function: ApiFunction {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.parameters.clone(),
                },
            })
            .collect();
        let tool_choice = (!tools.is_empty()).then_some("auto");
        let body = ChatRequest {
            model: &self.model,
            messages: request.messages.clone(),
            stream: false,
            max_tokens: policy.max_tokens,
            temperature: policy.temperature,
            top_p: policy.top_p,
            top_k: policy.top_k,
            min_p: policy.min_p,
            repeat_penalty: policy.repeat_penalty,
            presence_penalty: policy.presence_penalty,
            frequency_penalty: policy.frequency_penalty,
            seed: policy.seed,
            timings_per_token: true,
            chat_template_kwargs: ChatTemplateKwargs {
                enable_thinking: policy.enable_thinking,
            },
            response_format: policy.json_mode.then_some(ResponseFormat {
                kind: "json_object",
            }),
            stop: policy.stop.clone(),
            tools,
            tool_choice,
        };
        let mut builder = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .timeout(Duration::from_secs(policy.timeout_secs))
            .json(&body);
        if let Some(api_key) = &self.api_key {
            builder = builder.bearer_auth(api_key);
        }
        let started = Instant::now();
        let response = builder
            .send()
            .await
            .map_err(|error| AskesisError::EvalBackend(error.to_string()))?;
        let status = response.status();
        let payload = response
            .text()
            .await
            .map_err(|error| AskesisError::EvalBackend(error.to_string()))?;
        if !status.is_success() {
            return Err(AskesisError::EvalBackend(format!(
                "HTTP {status}: {}",
                payload.chars().take(500).collect::<String>()
            )));
        }
        let decoded: ChatResponse = serde_json::from_str(&payload)
            .map_err(|error| AskesisError::EvalBackend(format!("decode response: {error}")))?;
        let choice = decoded
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AskesisError::EvalBackend("response has no choices".into()))?;
        let usage = decoded.usage.unwrap_or(Usage {
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        });
        Ok(ModelCompletion {
            content: choice.message.content.unwrap_or_default(),
            finish_reason: choice.finish_reason,
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            },
            latency_ms: started.elapsed().as_millis() as u64,
            timings: decoded.timings,
            tool_calls: choice
                .message
                .tool_calls
                .into_iter()
                .map(|call| ToolCall {
                    id: call.id,
                    name: call.function.name,
                    arguments_json: call.function.arguments,
                })
                .collect(),
        })
    }
}
