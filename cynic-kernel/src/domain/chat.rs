//! ChatPort — minimal text-in/text-out contract for LLM inference.
//! Dogs use this via BackendPort (shared health + name).
//!
//! InferenceProfile lets the kernel declare INTENT per call — the same backend
//! serves scoring (fast, no thinking) and agent work (full reasoning, big context).

use crate::domain::inference::{BackendPort, BackendStatus};
use async_trait::async_trait;

// ── Inference Profiles ───────────────────────────────────────

/// What kind of work the kernel is doing. Determines baseline parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceProfileKind {
    /// Dog axiom scoring: structured JSON output.
    Scoring,
    /// Hermes agent / deep analysis: full reasoning, large context window.
    Agent,
    /// Text summarization: moderate output, no thinking overhead.
    Summary,
    /// Free-form inference (/infer MCP tool): full reasoning capacity.
    Infer,
}

/// Inference profile = intent (kind) + dynamic budget.
///
/// The Dog computes `max_tokens` per request from:
///   `min(context_size - estimated_prompt_tokens, completion_budget_from_calibration)`
/// instead of a hardcoded value. When `max_tokens` is None, falls back to kind default.
#[derive(Debug, Clone, Copy)]
pub struct InferenceProfile {
    pub kind: InferenceProfileKind,
    /// Dynamic completion budget computed by the Dog for THIS specific request.
    /// When Some, overrides the kind's default max_tokens.
    /// When None, the kind's default applies (backwards compatible).
    max_tokens: Option<u32>,
}

impl InferenceProfile {
    /// Backwards-compatible constructors matching the old enum variants.
    pub const SCORING: Self = Self {
        kind: InferenceProfileKind::Scoring,
        max_tokens: None,
    };
    pub const AGENT: Self = Self {
        kind: InferenceProfileKind::Agent,
        max_tokens: None,
    };
    pub const SUMMARY: Self = Self {
        kind: InferenceProfileKind::Summary,
        max_tokens: None,
    };
    pub const INFER: Self = Self {
        kind: InferenceProfileKind::Infer,
        max_tokens: None,
    };

    /// Create a Scoring profile with a dynamic max_tokens budget.
    /// The Dog calls this after computing `context_size - estimated_prompt_tokens`.
    pub fn scoring_with_budget(max_tokens: u32) -> Self {
        Self {
            kind: InferenceProfileKind::Scoring,
            max_tokens: Some(max_tokens),
        }
    }

    /// Max completion tokens — dynamic budget if set, otherwise kind default.
    pub fn max_tokens(&self) -> u32 {
        if let Some(override_val) = self.max_tokens {
            return override_val;
        }
        match self.kind {
            InferenceProfileKind::Scoring => 1024,
            InferenceProfileKind::Agent => 8192,
            InferenceProfileKind::Summary => 1024,
            InferenceProfileKind::Infer => 4096,
        }
    }

    /// Temperature override. None = use backend default.
    pub fn temperature(&self) -> Option<f32> {
        match self.kind {
            InferenceProfileKind::Scoring => Some(0.1),
            InferenceProfileKind::Summary => Some(0.2),
            InferenceProfileKind::Agent | InferenceProfileKind::Infer => None,
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
        request_id: Option<&str>,
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
        _request_id: Option<&str>,
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
