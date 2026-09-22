//! Error types for Porkbun provider operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PorkbunError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Porkbun API returned error status {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("credential not found for reference: {0}")]
    CredentialNotFound(String),
}
