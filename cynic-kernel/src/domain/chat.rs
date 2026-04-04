//! ChatPort — minimal text-in/text-out contract for LLM inference.
//! Dogs use this via BackendPort (shared health + name).
//!
//! InferenceProfile lets the kernel declare INTENT per call — the same backend
//! serves scoring (fast, no thinking) and agent work (full reasoning, big context).

use crate::domain::inference::{BackendPort, BackendStatus};
use async_trait::async_trait;

// ── Inference Profiles ───────────────────────────────────────

/// The kernel knows what kind of work it's doing. Profiles tune inference
/// parameters per call without duplicating backend config.
///
/// Hardware is fixed (288 GB/s on RTX 4060 Ti). The only software lever
/// that matters is how many tokens we ask the model to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceProfile {
    /// Dog axiom scoring: structured JSON output. Thinking controlled by backend config.
    Scoring,
    /// Hermes agent / deep analysis: full reasoning, large context window.
    Agent,
    /// Text summarization: moderate output, no thinking overhead.
    Summary,
    /// Free-form inference (/infer MCP tool): full reasoning capacity.
    Infer,
}

impl InferenceProfile {
    /// Max completion tokens for this profile.
    /// Scoring uses 1024 (not 512) because cloud API tokenizers (Gemini) have
    /// different token counts than llama.cpp — 512 truncated Gemini's JSON response.
    pub fn max_tokens(&self) -> u32 {
        match self {
            Self::Scoring => 1024,
            Self::Agent => 8192,
            Self::Summary => 1024,
            Self::Infer => 4096,
        }
    }

    /// Temperature override. None = use backend default.
    pub fn temperature(&self) -> Option<f32> {
        match self {
            Self::Scoring => Some(0.3),
            Self::Summary => Some(0.2),
            Self::Agent | Self::Infer => None,
        }
    }
}

// ── ChatPort ─────────────────────────────────────────────────

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
    /// Send a system+user prompt with an inference profile that tunes
    /// max_tokens, thinking, and temperature per use-case.
    async fn chat(
        &self,
        system: &str,
        user: &str,
        profile: InferenceProfile,
    ) -> Result<ChatResponse, ChatError>;
}

/// Mock implementation for tests.
#[derive(Debug)]
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
    async fn chat(
        &self,
        _system: &str,
        _user: &str,
        _profile: InferenceProfile,
    ) -> Result<ChatResponse, ChatError> {
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
