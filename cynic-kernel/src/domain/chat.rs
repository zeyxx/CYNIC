//! ChatPort — minimal text-in/text-out contract for LLM inference.
//! Dogs use this via BackendPort (shared health + name).

use async_trait::async_trait;
use crate::domain::inference::{BackendPort, BackendStatus};

/// Response from a chat completion — text + token usage.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChatError {
    #[error("Chat unreachable: {0}")]
    Unreachable(String),
    #[error("Chat timed out after {ms}ms")]
    Timeout { ms: u64 },
    #[error("Chat rate limited: {0}")]
    RateLimited(String),
    #[error("Chat protocol error: {0}")]
    Protocol(String),
}

/// Chat-specific extension of BackendPort. Dogs use this for axiom evaluation.
/// `name()` and `health()` come from BackendPort — no duplication.
#[async_trait]
pub trait ChatPort: BackendPort {
    /// Send a system+user prompt, get back the assistant's response with token usage.
    async fn chat(&self, system: &str, user: &str) -> Result<ChatResponse, ChatError>;
}

/// Mock implementation for tests.
pub struct MockChatBackend {
    pub response: String,
    pub name: String,
    pub force_error: Option<ChatError>,
}

impl MockChatBackend {
    pub fn new(name: &str, response: &str) -> Self {
        Self {
            response: response.to_string(),
            name: name.to_string(),
            force_error: None,
        }
    }
}

#[async_trait]
impl BackendPort for MockChatBackend {
    fn name(&self) -> &str {
        &self.name
    }

    async fn health(&self) -> BackendStatus {
        if self.force_error.is_some() {
            BackendStatus::Critical
        } else {
            BackendStatus::Healthy
        }
    }
}

#[async_trait]
impl ChatPort for MockChatBackend {
    async fn chat(&self, _system: &str, _user: &str) -> Result<ChatResponse, ChatError> {
        if let Some(ref err) = self.force_error {
            return Err(err.clone());
        }
        Ok(ChatResponse {
            text: self.response.clone(),
            prompt_tokens: 0,
            completion_tokens: 0,
        })
    }
}
