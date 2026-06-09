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
    InvalidLogEntry(String),

    #[error("gemini subprocess failed: {0}")]
    GeminiSubprocess(String),

    #[error("google calendar api error: {0}")]
    GoogleCalendar(String),

    #[error("oauth2 error: {0}")]
    OAuth(String),
}

pub type Result<T> = std::result::Result<T, AskesisError>;
