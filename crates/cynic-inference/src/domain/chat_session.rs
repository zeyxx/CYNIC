//! Chat session domain: replaying event log (not semantic long-term memory).

use serde::{Deserialize, Serialize};

/// Kind of durable session event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatEventKind {
    User,
    Assistant,
    ToolCall,
    ToolResult,
    System,
    Error,
    /// Metadata about a stream (not every token).
    StreamMeta,
}

/// One append-only event in a session transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatEvent {
    pub seq: u64,
    pub kind: ChatEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_args_json: Option<String>,
    pub created_at_unix_ms: u64,
}

/// Session index metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub event_count: u64,
}

/// OpenAI-compat style message for request assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
}

/// Tool definition exposed to the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

/// Progressive stream chunk (buffered stream abstraction for v1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamChunk {
    pub index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_delta: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_delta: Option<serde_json::Value>,
    pub done: bool,
}

impl ChatEvent {
    pub fn user(seq: u64, content: impl Into<String>, now_ms: u64) -> Self {
        Self {
            seq,
            kind: ChatEventKind::User,
            role: Some("user".into()),
            content: Some(content.into()),
            tool_name: None,
            tool_call_id: None,
            tool_args_json: None,
            created_at_unix_ms: now_ms,
        }
    }

    pub fn assistant(seq: u64, content: impl Into<String>, now_ms: u64) -> Self {
        Self {
            seq,
            kind: ChatEventKind::Assistant,
            role: Some("assistant".into()),
            content: Some(content.into()),
            tool_name: None,
            tool_call_id: None,
            tool_args_json: None,
            created_at_unix_ms: now_ms,
        }
    }

    pub fn tool_call(
        seq: u64,
        call_id: impl Into<String>,
        name: impl Into<String>,
        args_json: impl Into<String>,
        now_ms: u64,
    ) -> Self {
        Self {
            seq,
            kind: ChatEventKind::ToolCall,
            role: Some("assistant".into()),
            content: None,
            tool_name: Some(name.into()),
            tool_call_id: Some(call_id.into()),
            tool_args_json: Some(args_json.into()),
            created_at_unix_ms: now_ms,
        }
    }

    pub fn tool_result(
        seq: u64,
        call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        now_ms: u64,
    ) -> Self {
        Self {
            seq,
            kind: ChatEventKind::ToolResult,
            role: Some("tool".into()),
            content: Some(content.into()),
            tool_name: Some(name.into()),
            tool_call_id: Some(call_id.into()),
            tool_args_json: None,
            created_at_unix_ms: now_ms,
        }
    }
}

/// Build OpenAI-style messages from durable events (for resume).
pub fn messages_from_events(events: &[ChatEvent]) -> Vec<ChatMessage> {
    let mut out = Vec::new();
    for e in events {
        match e.kind {
            ChatEventKind::User => {
                out.push(ChatMessage {
                    role: "user".into(),
                    content: e.content.clone().unwrap_or_default(),
                    tool_call_id: None,
                    name: None,
                    tool_calls: None,
                });
            }
            ChatEventKind::Assistant => {
                if let Some(c) = &e.content {
                    out.push(ChatMessage {
                        role: "assistant".into(),
                        content: c.clone(),
                        tool_call_id: None,
                        name: None,
                        tool_calls: None,
                    });
                }
            }
            ChatEventKind::ToolCall => {
                let id = e.tool_call_id.clone().unwrap_or_default();
                let name = e.tool_name.clone().unwrap_or_default();
                let args = e.tool_args_json.clone().unwrap_or_else(|| "{}".into());
                let tool_calls = serde_json::json!([{
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": args }
                }]);
                out.push(ChatMessage {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    name: None,
                    tool_calls: Some(tool_calls),
                });
            }
            ChatEventKind::ToolResult => {
                out.push(ChatMessage {
                    role: "tool".into(),
                    content: e.content.clone().unwrap_or_default(),
                    tool_call_id: e.tool_call_id.clone(),
                    name: e.tool_name.clone(),
                    tool_calls: None,
                });
            }
            ChatEventKind::System => {
                out.push(ChatMessage {
                    role: "system".into(),
                    content: e.content.clone().unwrap_or_default(),
                    tool_call_id: None,
                    name: None,
                    tool_calls: None,
                });
            }
            ChatEventKind::Error | ChatEventKind::StreamMeta => {}
        }
    }
    out
}
