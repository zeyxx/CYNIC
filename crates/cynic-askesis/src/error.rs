//! Error types for cynic-askesis.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AskesisError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("serde_json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("audit engine unavailable: {0}")]
    AuditUnavailable(String),

    #[error("anchor provider error: {0}")]
    AnchorProvider(String),

    #[error("invalid log entry: {0}")]
    InvalidObservationEvent(String),

    #[error("gemini subprocess failed: {0}")]
    GeminiSubprocess(String),

    #[error("google calendar api error: {0}")]
    GoogleCalendar(String),

    #[error("oauth2 error: {0}")]
    OAuth(String),

    #[error("agent eval dataset error: {0}")]
    EvalDataset(String),

    #[error("agent eval policy error: {0}")]
    EvalPolicy(String),

    #[error("agent eval backend error: {0}")]
    EvalBackend(String),

    #[error("kyon scenario error: {0}")]
    KyonScenario(String),
}

pub type Result<T> = std::result::Result<T, AskesisError>;
