use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant};

use crate::application::agent_harness::{HarnessExecutor, InferenceReply};
use crate::application::agent_harness::{ScenarioSpec, TaskSpec};

#[derive(Debug, Clone)]
pub struct HttpHarnessExecutor {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl HttpHarnessExecutor {
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        timeout: Duration,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.or_else(|| crate::resolve_chat_api_key(None)),
        })
    }
}

#[derive(Serialize)]
struct RequestBody<'a> {
    model: &'a str,
    messages: [Message<'a>; 1],
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
    stream_options: StreamOptions,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'static str,
    content: &'a str,
}

#[async_trait]
impl HarnessExecutor for HttpHarnessExecutor {
    async fn execute(&self, scenario: &ScenarioSpec, task: &TaskSpec) -> InferenceReply {
        let started = Instant::now();
        let url = format!("{}/chat/completions", self.base_url);
        let body = RequestBody {
            model: &scenario.model.parameters.model_id,
            messages: [Message {
                role: "user",
                content: &task.prompt,
            }],
            max_tokens: task.max_output_tokens,
            temperature: scenario.model.parameters.temperature,
            top_p: scenario.model.parameters.top_p,
            stream: true,
            stream_options: StreamOptions { include_usage: true },
        };
        let mut request = self.client.post(url).json(&body);
        if let Some(correlation_id) = &scenario.correlation_id {
            request = request.header("X-CYNIC-Correlation-ID", correlation_id);
        }
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                return failed(0, error.to_string());
            }
        };

        let status = response.status().as_u16();
        if status >= 400 {
            let err_text = response.text().await.unwrap_or_default();
            return failed(status, err_text);
        }

        let mut stream = response.bytes_stream();
        let mut first_token_time: Option<Instant> = None;
        let mut chunk_count = 0u32;
        let mut prompt_tokens = None;
        let mut completion_tokens = None;
        let mut event_kinds = Vec::new();
        let mut buffer = String::new();

        while let Some(chunk_res) = stream.next().await {
            let bytes = match chunk_res {
                Ok(b) => b,
                Err(err) => {
                    return InferenceReply {
                        runtime_run_id: None,
                        http_status: Some(status),
                        prompt_tokens,
                        output_tokens: completion_tokens,
                        prompt_tokens_per_second: None,
                        output_tokens_per_second: None,
                        wall_time_ms: Some(started.elapsed().as_millis() as u64),
                        event_count: Some(chunk_count),
                        event_kinds,
                        event_stream_error: Some(err.to_string()),
                        error: Some(format!("stream_read_error: {err}")),
                    };
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(pos) = buffer.find("\n\n") {
                let sse_line = buffer[..pos].to_string();
                buffer.drain(..pos + 2);

                for line in sse_line.lines() {
                    let line = line.trim();
                    if let Some(data_str) = line.strip_prefix("data: ") {
                        let data_str = data_str.trim();
                        if data_str == "[DONE]" {
                            continue;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(data_str) {
                            chunk_count += 1;
                            if first_token_time.is_none() {
                                first_token_time = Some(Instant::now());
                            }

                            if let Some(choices) = val["choices"].as_array() {
                                for choice in choices {
                                    if let Some(delta) = choice.get("delta") {
                                        if delta.get("content").is_some() {
                                            event_kinds.push("content_delta".to_string());
                                        }
                                        if delta.get("reasoning_content").is_some() {
                                            event_kinds.push("thinking_delta".to_string());
                                        }
                                    }
                                }
                            }

                            if let Some(usage) = val.get("usage") {
                                if let Some(pt) = usage["prompt_tokens"].as_u64() {
                                    prompt_tokens = Some(pt);
                                }
                                if let Some(ct) = usage["completion_tokens"].as_u64() {
                                    completion_tokens = Some(ct);
                                }
                            }
                        }
                    }
                }
            }
        }

        let wall_time_ms = started.elapsed().as_millis() as u64;
        let ttft_ms = first_token_time
            .map(|t| t.duration_since(started).as_millis() as u64)
            .unwrap_or(wall_time_ms);
        let gen_duration_ms = wall_time_ms.saturating_sub(ttft_ms);

        let prompt_tokens_per_second = prompt_tokens
            .filter(|_| ttft_ms > 0)
            .map(|pt| (pt as f64 * 1000.0 / ttft_ms as f64) as f32);

        let output_tokens_per_second = completion_tokens
            .filter(|_| gen_duration_ms > 0)
            .map(|ct| (ct as f64 * 1000.0 / gen_duration_ms as f64) as f32)
            .or_else(|| {
                completion_tokens
                    .filter(|_| wall_time_ms > 0)
                    .map(|ct| (ct as f64 * 1000.0 / wall_time_ms as f64) as f32)
            });

        InferenceReply {
            runtime_run_id: None,
            http_status: Some(status),
            prompt_tokens,
            output_tokens: completion_tokens,
            prompt_tokens_per_second,
            output_tokens_per_second,
            wall_time_ms: Some(wall_time_ms),
            event_count: Some(chunk_count),
            event_kinds,
            event_stream_error: None,
            error: None,
        }
    }
}

fn failed(status: u16, error: String) -> InferenceReply {
    InferenceReply {
        runtime_run_id: None,
        http_status: if status > 0 { Some(status) } else { None },
        prompt_tokens: None,
        output_tokens: None,
        prompt_tokens_per_second: None,
        output_tokens_per_second: None,
        wall_time_ms: None,
        event_count: None,
        event_kinds: Vec::new(),
        event_stream_error: None,
        error: Some(error),
    }
}
