//! Scripted chat support: mock client + builtin tools for offline CLI and tests.
//!
//! Extracted from `chat_service`; the service owns session/turn logic, this
//! module owns the scripted doubles. No production backend code here.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::chat_service::{
    ChatCompletionClient, ChatServiceError, CompletionOutcome, ToolCallRequest, ToolInvoker,
};
use crate::domain::chat_session::{ChatMessage, StreamChunk, ToolSpec};

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Mock client + tools for tests and offline CLI ───────────────────────────

/// Scripted mock: first call may return tool_calls; subsequent returns content.
/// When stream=true, content is split into progressive chunks.
#[derive(Debug)]
pub struct MockChatClient {
    pub tool_on_first: Option<ToolCallRequest>,
    pub final_content: String,
    calls: std::sync::Mutex<u32>,
}

impl MockChatClient {
    pub fn plain(content: impl Into<String>) -> Self {
        Self {
            tool_on_first: None,
            final_content: content.into(),
            calls: std::sync::Mutex::new(0),
        }
    }

    pub fn with_tool(tool: ToolCallRequest, final_content: impl Into<String>) -> Self {
        Self {
            tool_on_first: Some(tool),
            final_content: final_content.into(),
            calls: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl ChatCompletionClient for MockChatClient {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        _tools: &[ToolSpec],
        stream: bool,
    ) -> Result<CompletionOutcome, ChatServiceError> {
        let mut n = self
            .calls
            .lock()
            .map_err(|e| ChatServiceError::Protocol(e.to_string()))?;
        *n += 1;
        let last_user = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let last_role = messages.last().map(|m| m.role.as_str()).unwrap_or("");
        // Scripted single-shot tool (unit tests): only on first complete.
        if *n == 1
            && let Some(tc) = &self.tool_on_first
        {
            return Ok(CompletionOutcome {
                content: None,
                tool_calls: vec![tc.clone()],
                stream_chunks: vec![],
                token_entropy_snapshots: vec![],
                token_uncertainty: None,
            });
        }
        // Interactive mock: tool keyword only when the latest message is the user turn
        // (avoids infinite tool loops after tool_result is appended).
        if last_role == "user"
            && last_user.to_lowercase().contains("tool")
            && self.tool_on_first.is_none()
        {
            return Ok(CompletionOutcome {
                content: None,
                tool_calls: vec![ToolCallRequest {
                    id: format!("call_{n}"),
                    name: "echo".into(),
                    arguments_json: format!(
                        r#"{{"text":{}}}"#,
                        serde_json::to_string(last_user).unwrap_or_else(|_| "\"ok\"".into())
                    ),
                }],
                stream_chunks: vec![],
                token_entropy_snapshots: vec![],
                token_uncertainty: None,
            });
        }
        // Interactive TUI uses placeholder "(mock ready)" so each turn echoes last user text.
        // Scripted unit tests set a concrete final_content string and keep it.
        let content = if self.final_content == "(mock ready)" || self.final_content.is_empty() {
            format!("(mock) {last_user}")
        } else {
            self.final_content.clone()
        };
        let stream_chunks = if stream {
            chunk_string(&content)
        } else {
            vec![]
        };
        Ok(CompletionOutcome {
            content: Some(content),
            tool_calls: vec![],
            stream_chunks,
            token_entropy_snapshots: vec![],
            token_uncertainty: None,
        })
    }
}

fn chunk_string(s: &str) -> Vec<StreamChunk> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let step = (chars.len() / 3).max(1);
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
    if chunks.is_empty() {
        chunks.push(StreamChunk {
            index: 0,
            content_delta: Some(String::new()),
            tool_call_delta: None,
            done: true,
        });
    }
    chunks
}

/// Built-in tools: `echo` and `get_time` only (safe default allowlist).
#[derive(Debug, Default)]
pub struct BuiltinToolInvoker;

impl ToolInvoker for BuiltinToolInvoker {
    fn invoke(&self, name: &str, arguments_json: &str) -> Result<String, ChatServiceError> {
        match name {
            "echo" => {
                let v: serde_json::Value =
                    serde_json::from_str(arguments_json).unwrap_or_else(|_| serde_json::json!({}));
                let text = v
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or(arguments_json);
                Ok(text.to_string())
            }
            "get_time" => Ok(format!("unix_ms={}", now_ms())),
            other => Err(ChatServiceError::Tool(format!(
                "tool not allowlisted: {other}"
            ))),
        }
    }
}

pub fn default_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "echo".into(),
            description: "Echo back text".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }),
        },
        ToolSpec {
            name: "get_time".into(),
            description: "Return current unix milliseconds".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
    ]
}
