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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DogFailureKind {
    ApiError,
    ParseError,
    RateLimited,
    Timeout,
}

impl DogFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiError => "api_error",
            Self::ParseError => "parse_error",
            Self::RateLimited => "rate_limited",
            Self::Timeout => "timeout",
        }
    }
}

impl From<&DogError> for DogFailureKind {
    fn from(e: &DogError) -> Self {
        match e {
            DogError::ApiError(_) => Self::ApiError,
            DogError::ParseError(_)
            | DogError::ZeroFlood(_)
            | DogError::DegenerateScores { .. } => Self::ParseError,
            DogError::RateLimited(_) => Self::RateLimited,
            DogError::Timeout => Self::Timeout,
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
