//! Live OpenAI-compat chat client for the operator chat service.
//!
//! Implements [`ChatCompletionClient`] against `POST {base}/chat/completions`.
//! Does not depend on cynic-kernel.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::application::chat_service::{
    ChatCompletionClient, ChatServiceError, CompletionOutcome, ToolCallRequest,
};
use crate::domain::chat_session::{ChatMessage, StreamChunk, ToolSpec};

/// HTTP client talking to an OpenAI-compatible inference endpoint.
#[derive(Debug, Clone)]
pub struct HttpOpenAiChatClient {
    client: Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    timeout_secs: u64,
}

impl HttpOpenAiChatClient {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, ChatServiceError> {
        Self::with_api_key(base_url, model, None)
    }

    pub fn with_api_key(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Result<Self, ChatServiceError> {
        let timeout_secs = 120u64;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| ChatServiceError::Model(format!("http client: {e}")))?;
        let api_key = api_key
            .filter(|k| !k.trim().is_empty())
            .or_else(|| std::env::var("CYNIC_CHAT_API_KEY").ok().filter(|k| !k.is_empty()));
        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            api_key,
            timeout_secs,
        })
    }

    fn url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    stream: bool,
}

#[derive(Deserialize)]
struct ResponseBody {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Option<ChoiceMessage>,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(default)]
    function: Option<ApiFunction>,
}

#[derive(Deserialize)]
struct ApiFunction {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[async_trait]
impl ChatCompletionClient for HttpOpenAiChatClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        stream: bool,
    ) -> Result<CompletionOutcome, ChatServiceError> {
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                let mut v = json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(id) = &m.tool_call_id {
                    v["tool_call_id"] = json!(id);
                }
                if let Some(name) = &m.name {
                    v["name"] = json!(name);
                }
                if let Some(tc) = &m.tool_calls {
                    v["tool_calls"] = tc.clone();
                }
                v
            })
            .collect();

        let tool_defs = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters,
                            }
                        })
                    })
                    .collect(),
            )
        };

        // Streamed SSE is parsed as buffered non-stream request for v1 wire
        // reliability; progressive chunks are synthesized from final content
        // when stream=true (same contract as MockChatClient).
        let body = RequestBody {
            model: self.model.clone(),
            messages: msgs,
            tools: tool_defs,
            stream: false,
        };

        let mut req = self.client.post(self.url()).json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                ChatServiceError::Model(format!("timeout after {}s", self.timeout_secs))
            } else {
                ChatServiceError::Model(format!("unreachable: {e}"))
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChatServiceError::Model(format!("HTTP {status}: {text}")));
        }

        let parsed: ResponseBody = resp
            .json()
            .await
            .map_err(|e| ChatServiceError::Protocol(format!("parse: {e}")))?;

        let msg = parsed
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .ok_or_else(|| ChatServiceError::Protocol("empty choices".into()))?;

        let tool_calls: Vec<ToolCallRequest> = msg
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|c| {
                        let f = c.function.as_ref()?;
                        Some(ToolCallRequest {
                            id: c.id.clone(),
                            name: f.name.clone(),
                            arguments_json: f.arguments.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let content = msg.content.clone().filter(|s| !s.is_empty());
        let stream_chunks = if stream {
            if let Some(ref c) = content {
                synthesize_chunks(c)
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        Ok(CompletionOutcome {
            content,
            tool_calls,
            stream_chunks,
        })
    }
}

fn synthesize_chunks(s: &str) -> Vec<StreamChunk> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return vec![StreamChunk {
            index: 0,
            content_delta: Some(String::new()),
            tool_call_delta: None,
            done: true,
        }];
    }
    let step = (chars.len() / 3).max(1);
    let mut chunks = Vec::new();
    let mut i = 0;
    let mut index = 0u32;
    while i < chars.len() {
        let end = (i + step).min(chars.len());
        let part: String = chars[i..end].iter().collect();
        let done = end >= chars.len();
        chunks.push(StreamChunk {
            index,
            content_delta: Some(part),
            tool_call_delta: None,
            done,
        });
        index += 1;
        i = end;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builds_from_endpoint() {
        let c = HttpOpenAiChatClient::new("http://127.0.0.1:8080/v1", "test-model").unwrap();
        assert!(c.url().ends_with("/chat/completions"));
        assert_eq!(c.model, "test-model");
    }
}
