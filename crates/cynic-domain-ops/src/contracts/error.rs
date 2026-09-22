//! Error types for domain operations contracts.
//!
//! `DomainOpsError` covers all failure modes of desired-state parsing,
//! proof construction, and validation. Uses `thiserror` for ergonomic
//! error chains.

use cynic_protocol::domain_ops::{DOMAIN_OPS_DESIRED_STATE_SCHEMA, DOMAIN_OPS_PROOF_SCHEMA};

/// Error type for domain operations contract parsing and validation.
#[derive(Debug, thiserror::Error)]
pub enum DomainOpsError {
    #[error("JSON deserialization error: {context}")]
    JsonParse {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("schema mismatch: expected '{expected}', got '{got}'")]
    SchemaMismatch {
        expected: &'static str,
        got: String,
    },

    #[error("invalid domain name '{domain}': {reason}")]
    InvalidDomain {
        domain: String,
        reason: String,
    },

    #[error("provider config error for '{provider}': {reason}")]
    ProviderConfig {
        provider: String,
        reason: String,
    },

    #[error("proxy policy error: {0}")]
    ProxyPolicy(String),

    #[error("edge probe error: {0}")]
    EdgeProbe(String),

    #[error("unexpected proof schema '{0}'")]
    UnexpectedProof(String),

    #[error("missing idempotency key for operation '{operation:?}' on domain '{domain}'")]
    MissingIdempotencyKey {
        domain: String,
        operation: cynic_protocol::domain_ops::DomainOperation,
    },
}

impl DomainOpsError {
    /// Convenience constructor for JSON parse errors with context.
    pub fn json_parse(context: &str, source: serde_json::Error) -> Self {
        Self::JsonParse {
            context: context.to_string(),
            source,
        }
    }

    /// Convenience constructor for schema mismatch on desired state.
    pub fn schema_mismatch(got: &str) -> Self {
        Self::SchemaMismatch {
            expected: DOMAIN_OPS_DESIRED_STATE_SCHEMA,
            got: got.to_string(),
        }
    }

    /// Convenience constructor for unexpected proof schema.
    pub fn unexpected_proof_schema(got: &str) -> Self {
        Self::UnexpectedProof(got.to_string())
    }

    /// True if this error is a schema mismatch (can be used for retry logic).
    pub fn is_schema_mismatch(&self) -> bool {
        matches!(self, Self::SchemaMismatch { .. })
    }

    /// True if this error is a JSON parse error (malformed input).
    pub fn is_json_parse_error(&self) -> bool {
        matches!(self, Self::JsonParse { .. })
    }
}

// Silence unused warning for DOMAIN_OPS_PROOF_SCHEMA if only used in
// error messages in some configurations.
#[allow(dead_code)]
const _PROOF_SCHEMA: &str = DOMAIN_OPS_PROOF_SCHEMA;
