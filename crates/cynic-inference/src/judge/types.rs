//! Judge failure and error types.
//! Pure domain types — no I/O, no async, no state.

use crate::domain::dog::DogError;

/// A single Dog's failure — preserves the error kind for programmatic classification.
#[derive(Debug)]
pub struct DogFailure {
    pub dog_id: String,
    pub kind: DogFailureKind,
    pub detail: String,
}

impl std::fmt::Display for DogFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.dog_id, self.detail)
    }
}

/// Typed failure kind — mirrors DogError variants for downstream classification.
/// Surfaced in the verdict response as `failed_dog_error_kinds` for programmatic handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DogFailureKind {
    /// HTTP/network layer error from the inference backend (non-connectivity).
    ApiError,
    /// Dog produced invalid/unparseable output — model hallucinated or ignored schema.
    ParseError,
    /// Backend returned HTTP 429 or similar quota signal.
    RateLimited,
    /// Request exceeded the per-dog wall-clock timeout.
    Timeout,
    /// Inference slot unavailable (Soma L2 — all slots busy, non-blocking priority skipped).
    SlotUnavailable,
    /// Backend unreachable at network layer (connection refused, DNS failure, Tailscale down).
    NetworkUnreachable,
    /// Dog excluded by circuit breaker — too many recent failures, cooling down.
    CircuitOpen,
    /// Dog excluded by routing quality gate (json_valid_rate, failure_rate below threshold).
    QualityGateExcluded,
}

impl DogFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiError => "api_error",
            Self::ParseError => "parse_error",
            Self::RateLimited => "rate_limited",
            Self::Timeout => "timeout",
            Self::SlotUnavailable => "slot_unavailable",
            Self::NetworkUnreachable => "network_unreachable",
            Self::CircuitOpen => "circuit_open",
            Self::QualityGateExcluded => "quality_gate_excluded",
        }
    }
}

impl From<&DogError> for DogFailureKind {
    fn from(e: &DogError) -> Self {
        match e {
            DogError::ApiError(msg)
                if msg.contains("tcp connect")
                    || msg.contains("connection refused")
                    || msg.contains("unreachable") =>
            {
                Self::NetworkUnreachable
            }
            DogError::ApiError(_) => Self::ApiError,
            DogError::ParseError(_)
            | DogError::ZeroFlood(_)
            | DogError::DegenerateScores { .. } => Self::ParseError,
            DogError::RateLimited(_) => Self::RateLimited,
            DogError::Timeout => Self::Timeout,
            DogError::ContextOverflow { .. } => Self::ApiError,
            DogError::SlotUnavailable => Self::SlotUnavailable,
        }
    }
}

#[derive(Debug)]
pub enum JudgeError {
    NoDogs,
    AllDogsFailed(Vec<DogFailure>),
    InvalidInput(String),
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDogs => write!(f, "No Dogs configured"),
            Self::AllDogsFailed(failures) => {
                let msgs: Vec<String> = failures.iter().map(|f| f.to_string()).collect();
                write!(f, "All Dogs failed: {}", msgs.join("; "))
            }
            Self::InvalidInput(reason) => write!(f, "Invalid input: {reason}"),
        }
    }
}

impl std::error::Error for JudgeError {}
