//! Error types for Cloudflare provider operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CloudflareError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Cloudflare API error (code {code}): {message}")]
    ApiError { code: u16, message: String },

    #[error("zone not found for '{0}'")]
    ZoneNotFound(String),

    #[error("credential not found for reference: {0}")]
    CredentialNotFound(String),

    #[error("invalid IP address in resolve_to: {0}")]
    InvalidIp(String),
}
