//! Chat domain values for text generation over inference backends.

/// What kind of work the caller is doing. Determines baseline parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceProfileKind {
    Scoring,
    Agent,
    Summary,
    Infer,
}

/// Inference profile = intent (kind) + dynamic budget.
#[derive(Debug, Clone, Copy)]
pub struct InferenceProfile {
    pub kind: InferenceProfileKind,
    max_tokens: Option<u32>,
}

impl InferenceProfile {
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

    pub fn scoring_with_budget(max_tokens: u32) -> Self {
        Self {
            kind: InferenceProfileKind::Scoring,
            max_tokens: Some(max_tokens),
        }
    }

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

    pub fn temperature(&self) -> Option<f32> {
        match self.kind {
            InferenceProfileKind::Scoring => Some(0.1),
            InferenceProfileKind::Summary => Some(0.2),
            InferenceProfileKind::Agent | InferenceProfileKind::Infer => None,
        }
    }
}

/// Response from a chat completion — text plus whatever the backend reported.
///
/// Usage is `Option`, not `u32`. A plain integer cannot express "the backend reported no
/// usage block", so every producer filled in a zero and every consumer read that zero as a
/// measured zero. That is the fabrication `references/llms-101-mapping.md` records as a
/// direct violation of "unknown is data": a matrix sweep ranking models by zero-filled
/// token counts produces a decision-relevant ordering out of numbers nobody measured.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    pub reasoning: Option<String>,
    /// Reported by the backend. `None` means unreported, which is not the same as zero.
    pub prompt_tokens: Option<u32>,
    /// Reported by the backend. `None` means unreported, which is not the same as zero.
    pub completion_tokens: Option<u32>,
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
