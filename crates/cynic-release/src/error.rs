//! Typed error codes for release decision engine.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReleaseError {
    #[error("Contract parse failure: {0}")]
    ContractParseError(String),

    #[error("Release blocked: {0}")]
    ReleaseBlocked(String),

    #[error("Relay rejected: {0}")]
    RelayRejected(String),
}
