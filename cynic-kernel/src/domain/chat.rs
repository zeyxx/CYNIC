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

#[derive(Debug, Clone)]
pub enum ChatError {
    Unreachable(String),
    Timeout { ms: u64 },
    RateLimited(String),
    Protocol(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(msg) => write!(f, "Chat unreachable: {}", msg),
            Self::Timeout { ms } => write!(f, "Chat timed out after {}ms", ms),
            Self::RateLimited(msg) => write!(f, "Chat rate limited: {}", msg),
            Self::Protocol(msg) => write!(f, "Chat protocol error: {}", msg),
        }
    }
}

impl std::error::Error for ChatError {}

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
