#[cfg(not(feature = "std"))]
use alloc::string::String;

use thiserror::Error;

/// Errors that can occur when working with CYNIC identity.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// Hex decoding failed.
    #[error("hex decode error: {0}")]
    HexDecode(String),

    /// Invalid key length.
    #[error("invalid key length: expected 16, got {0}")]
    InvalidLength(usize),

    /// ed25519 signature verification failed.
    #[error("signature verification failed: {0}")]
    Verification(String),

    /// ed25519 signing failed.
    #[error("signing failed: {0}")]
    Signing(String),
}
