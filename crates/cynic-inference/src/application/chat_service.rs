//! Operator chat service: session history + stream + bounded tool loop.
//!
//! Talks to an OpenAI-compat endpoint (inference plane). Does **not** own
//! kernel domain storage.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::domain::chat_session::{
    ChatEvent, ChatMessage, StreamChunk, ToolSpec, messages_from_events,
};
use crate::ports::chat_session_port::{ChatSessionStore, SessionStoreError};

/// How the chat client obtains model completions (injectable for tests).
#[async_trait::async_trait]
pub trait ChatCompletionClient: Send + Sync {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        stream: bool,
    ) -> Result<CompletionOutcome, ChatServiceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOutcome {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallRequest>,
    /// Progressive chunks when stream=true (may be empty if non-stream).
    pub stream_chunks: Vec<StreamChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

/// Local tool invoker (allowlisted).
pub trait ToolInvoker: Send + Sync {
    fn invoke(&self, name: &str, arguments_json: &str) -> Result<String, ChatServiceError>;
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChatServiceError {
    #[error("not ready: {0}")]
    NotReady(String),
    #[error("session: {0}")]
    Session(String),
    #[error("model: {0}")]
    Model(String),
    #[error("tool: {0}")]
    Tool(String),
    #[error("protocol: {0}")]
    Protocol(String),
}

impl From<SessionStoreError> for ChatServiceError {
    fn from(e: SessionStoreError) -> Self {
        ChatServiceError::Session(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceReadyReport {
    pub schema: &'static str,
    pub ready: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub session_root: String,
    pub message: String,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurnReport {
    pub schema: &'static str,
    pub status: String,
    pub session_id: String,
    pub ready: bool,
    pub content: Option<String>,
    pub stream_chunks: Vec<StreamChunk>,
    pub events_appended: u64,
    pub tool_turns: u32,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub session_root: String,
    pub message: String,
    pub blocked_reason: Option<String>,
}

#[derive(Clone)]
pub struct ChatService {
    store: Arc<dyn ChatSessionStore>,
    client: Arc<dyn ChatCompletionClient>,
    tools: Arc<dyn ToolInvoker>,
    tool_specs: Vec<ToolSpec>,
    endpoint: Option<String>,
    model: Option<String>,
    max_tool_turns: u32,
    /// When false, complete() is fail-closed with NotReady.
    inference_ready: bool,
}

impl std::fmt::Debug for ChatService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatService")
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("max_tool_turns", &self.max_tool_turns)
            .field("inference_ready", &self.inference_ready)
            .field("session_root", &self.store.root_dir())
            .finish()
    }
}

impl ChatService {
    pub fn new(
        store: Arc<dyn ChatSessionStore>,
        client: Arc<dyn ChatCompletionClient>,
        tools: Arc<dyn ToolInvoker>,
        tool_specs: Vec<ToolSpec>,
        endpoint: Option<String>,
        model: Option<String>,
        max_tool_turns: u32,
        inference_ready: bool,
    ) -> Self {
        Self {
            store,
            client,
            tools,
            tool_specs,
            endpoint,
            model,
            max_tool_turns: max_tool_turns.max(1),
            inference_ready,
        }
    }

    pub fn ready_report(&self) -> InferenceReadyReport {
        if self.inference_ready {
            InferenceReadyReport {
                schema: "cynic.chat.ready.v1",
                ready: true,
                endpoint: self.endpoint.clone(),
                model: self.model.clone(),
                session_root: self.store.root_dir().display().to_string(),
                message: "inference plane endpoint available".into(),
                blocked_reason: None,
            }
        } else {
            InferenceReadyReport {
                schema: "cynic.chat.ready.v1",
                ready: false,
                endpoint: self.endpoint.clone(),
                model: self.model.clone(),
                session_root: self.store.root_dir().display().to_string(),
                message: "inference not ready".into(),
                blocked_reason: Some(
                    "no reachable OpenAI-compat endpoint; start a native profile or configure CYNIC_CHAT_ENDPOINT"
                        .into(),
                ),
            }
        }
    }

    pub fn list_sessions(&self) -> Result<Vec<crate::domain::chat_session::SessionMeta>, ChatServiceError> {
        Ok(self.store.list_sessions()?)
    }

    pub fn export_session(&self, id: &str) -> Result<String, ChatServiceError> {
        Ok(self.store.export_jsonl(id)?)
    }

    pub fn load_events(&self, id: &str) -> Result<Vec<ChatEvent>, ChatServiceError> {
        Ok(self.store.load_events(id)?)
    }

    /// One-shot or resume turn with optional streaming + tool loop.
    pub async fn turn(
        &self,
        session_id: Option<&str>,
        user_prompt: &str,
        stream: bool,
    ) -> ChatTurnReport {
        let ready = self.ready_report();
        if !ready.ready {
            return ChatTurnReport {
                schema: "cynic.chat.turn.v1",
                status: "blocked".into(),
                session_id: session_id.unwrap_or("").into(),
                ready: false,
                content: None,
                stream_chunks: vec![],
                events_appended: 0,
                tool_turns: 0,
                endpoint: ready.endpoint,
                model: ready.model,
                session_root: ready.session_root,
                message: ready.message,
                blocked_reason: ready.blocked_reason,
            };
        }

        let sid = session_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("sess-{}", now_ms()));

        let title: String = user_prompt.chars().take(48).collect();
        if let Err(e) = self.store.create_session(
            &sid,
            Some(title.as_str()),
            self.model.as_deref(),
            self.endpoint.as_deref(),
        ) {
            return self.err_report(&sid, e.to_string());
        }

        let mut appended = 0u64;
        let now = now_ms();
        let events = match self.store.load_events(&sid) {
            Ok(e) => e,
            Err(e) => return self.err_report(&sid, e.to_string()),
        };
        let next_seq = events.iter().map(|e| e.seq).max().unwrap_or(0) + 1;
        let user_ev = ChatEvent::user(next_seq, user_prompt, now);
        if let Err(e) = self.store.append(&sid, user_ev) {
            return self.err_report(&sid, e.to_string());
        }
        appended += 1;

        let mut tool_turns = 0u32;
        let mut final_content: Option<String> = None;
        let mut all_chunks: Vec<StreamChunk> = Vec::new();
        let mut seq = next_seq + 1;

        for _ in 0..self.max_tool_turns {
            let history = match self.store.load_events(&sid) {
                Ok(e) => e,
                Err(e) => return self.err_report(&sid, e.to_string()),
            };
            let messages = messages_from_events(&history);
            let outcome = match self
                .client
                .complete(&messages, &self.tool_specs, stream)
                .await
            {
                Ok(o) => o,
                Err(e) => return self.err_report(&sid, e.to_string()),
            };
            all_chunks.extend(outcome.stream_chunks);

            if !outcome.tool_calls.is_empty() {
                for tc in &outcome.tool_calls {
                    let call_ev = ChatEvent::tool_call(
                        seq,
                        &tc.id,
                        &tc.name,
                        &tc.arguments_json,
                        now_ms(),
                    );
                    seq += 1;
                    if let Err(e) = self.store.append(&sid, call_ev) {
                        return self.err_report(&sid, e.to_string());
                    }
                    appended += 1;

                    let result = match self.tools.invoke(&tc.name, &tc.arguments_json) {
                        Ok(r) => r,
                        Err(e) => format!("tool error: {e}"),
                    };
                    let res_ev =
                        ChatEvent::tool_result(seq, &tc.id, &tc.name, &result, now_ms());
                    seq += 1;
                    if let Err(e) = self.store.append(&sid, res_ev) {
                        return self.err_report(&sid, e.to_string());
                    }
                    appended += 1;
                    tool_turns += 1;
                }
                continue;
            }

            if let Some(content) = outcome.content {
                let asst = ChatEvent::assistant(seq, &content, now_ms());
                if let Err(e) = self.store.append(&sid, asst) {
                    return self.err_report(&sid, e.to_string());
                }
                appended += 1;
                final_content = Some(content);
            }
            break;
        }

        ChatTurnReport {
            schema: "cynic.chat.turn.v1",
            status: "ok".into(),
            session_id: sid,
            ready: true,
            content: final_content,
            stream_chunks: all_chunks,
            events_appended: appended,
            tool_turns,
            endpoint: self.endpoint.clone(),
            model: self.model.clone(),
            session_root: self.store.root_dir().display().to_string(),
            message: "turn complete".into(),
            blocked_reason: None,
        }
    }

    fn err_report(&self, sid: &str, msg: String) -> ChatTurnReport {
        ChatTurnReport {
            schema: "cynic.chat.turn.v1",
            status: "error".into(),
            session_id: sid.into(),
            ready: self.inference_ready,
            content: None,
            stream_chunks: vec![],
            events_appended: 0,
            tool_turns: 0,
            endpoint: self.endpoint.clone(),
            model: self.model.clone(),
            session_root: self.store.root_dir().display().to_string(),
            message: msg,
            blocked_reason: None,
        }
    }
}

fn now_ms() -> u64 {
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
        let mut n = self.calls.lock().map_err(|e| ChatServiceError::Protocol(e.to_string()))?;
        *n += 1;
        let last_user = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let last_role = messages.last().map(|m| m.role.as_str()).unwrap_or("");
        // Scripted single-shot tool (unit tests): only on first complete.
        if *n == 1 {
            if let Some(tc) = &self.tool_on_first {
                return Ok(CompletionOutcome {
                    content: None,
                    tool_calls: vec![tc.clone()],
                    stream_chunks: vec![],
                });
            }
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
                let v: serde_json::Value = serde_json::from_str(arguments_json)
                    .unwrap_or_else(|_| serde_json::json!({}));
                let text = v
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or(arguments_json);
                Ok(text.to_string())
            }
            "get_time" => Ok(format!("unix_ms={}", now_ms())),
            other => Err(ChatServiceError::Tool(format!("tool not allowlisted: {other}"))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local::session_store::FsChatSessionStore;
    use crate::domain::chat_session::ChatEventKind;
    use tempfile::tempdir;

    #[tokio::test]
    async fn stream_and_tool_turn_records_events() {
        let dir = tempdir().unwrap();
        let store = Arc::new(FsChatSessionStore::new(dir.path()).unwrap());
        let client = Arc::new(MockChatClient::with_tool(
            ToolCallRequest {
                id: "call_1".into(),
                name: "echo".into(),
                arguments_json: r#"{"text":"pong"}"#.into(),
            },
            "final answer after tool",
        ));
        let tools = Arc::new(BuiltinToolInvoker);
        let svc = ChatService::new(
            store.clone(),
            client,
            tools,
            default_tool_specs(),
            Some("mock://local".into()),
            Some("mock-model".into()),
            4,
            true,
        );

        let report = svc.turn(Some("s1"), "use the tool", true).await;
        assert_eq!(report.status, "ok");
        assert!(report.tool_turns >= 1);
        assert_eq!(
            report.content.as_deref(),
            Some("final answer after tool")
        );
        // stream chunks from final content completion
        assert!(
            !report.stream_chunks.is_empty(),
            "stream path must yield progressive chunks"
        );
        let events = store.load_events("s1").unwrap();
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&ChatEventKind::User));
        assert!(kinds.contains(&ChatEventKind::ToolCall));
        assert!(kinds.contains(&ChatEventKind::ToolResult));
        assert!(kinds.contains(&ChatEventKind::Assistant));
        let tool_res = events
            .iter()
            .find(|e| e.kind == ChatEventKind::ToolResult)
            .unwrap();
        assert_eq!(tool_res.content.as_deref(), Some("pong"));
    }

    #[tokio::test]
    async fn fail_closed_when_not_ready() {
        let dir = tempdir().unwrap();
        let store = Arc::new(FsChatSessionStore::new(dir.path()).unwrap());
        let client = Arc::new(MockChatClient::plain("should not run"));
        let tools = Arc::new(BuiltinToolInvoker);
        let svc = ChatService::new(
            store,
            client,
            tools,
            vec![],
            None,
            None,
            2,
            false,
        );
        let report = svc.turn(None, "hi", false).await;
        assert_eq!(report.status, "blocked");
        assert!(!report.ready);
        assert!(report.blocked_reason.is_some());
        assert!(report.content.is_none());
    }

    #[tokio::test]
    async fn resume_appends_to_existing_session() {
        let dir = tempdir().unwrap();
        let store = Arc::new(FsChatSessionStore::new(dir.path()).unwrap());
        let client = Arc::new(MockChatClient::plain("second"));
        let tools = Arc::new(BuiltinToolInvoker);
        let svc = ChatService::new(
            store.clone(),
            client,
            tools,
            vec![],
            Some("mock://".into()),
            Some("m".into()),
            2,
            true,
        );
        let r1 = svc.turn(Some("resume-me"), "first", false).await;
        assert_eq!(r1.status, "ok");
        let r2 = svc.turn(Some("resume-me"), "second prompt", false).await;
        assert_eq!(r2.status, "ok");
        let events = store.load_events("resume-me").unwrap();
        let users: Vec<_> = events
            .iter()
            .filter(|e| e.kind == ChatEventKind::User)
            .map(|e| e.content.clone().unwrap_or_default())
            .collect();
        assert_eq!(users, vec!["first".to_string(), "second prompt".to_string()]);
    }
}
