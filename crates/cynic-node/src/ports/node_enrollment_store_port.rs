//! Node enrollment store port — durable grant lifecycle behind an injected boundary.
//!
//! Canonical home since 2026-09-28. Extracted from
//! `cynic-kernel::domain::node_enrollment` (Session 5 inversion fix): the
//! kernel consumed node-domain storage through a concrete module instead of a
//! port. Error shape follows the node-crate convention (`code` + `detail`,
//! no `thiserror`: `cynic-node` does not depend on it and the previous
//! thiserror enum had zero callers, so no caller-visible behavior is lost).

use async_trait::async_trait;
use cynic_node_protocol::SignedNodeEnrollmentGrant;

use super::node_enrollment_signer_port::NodeEnrollmentSignerPort;

/// Durable failure of the enrollment grant store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeEnrollmentStoreError {
    pub code: String,
    pub detail: String,
}

impl NodeEnrollmentStoreError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    /// A concurrent or fencing conflict (e.g. stale generation on replace).
    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new("node_enrollment_conflict", detail)
    }

    /// The store itself failed.
    pub fn storage(detail: impl Into<String>) -> Self {
        Self::new("node_enrollment_storage", detail)
    }
}

impl std::fmt::Display for NodeEnrollmentStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for NodeEnrollmentStoreError {}

/// Boundary that persists signed enrollment grants with single-use token
/// consumption and generation-fenced replacement.
#[async_trait]
pub trait NodeEnrollmentStorePort: Send + Sync + std::fmt::Debug {
    async fn consume_token_and_store_grant(
        &self,
        token_digest: &str,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError>;

    async fn grant(
        &self,
        node_id: &str,
    ) -> Result<Option<SignedNodeEnrollmentGrant>, NodeEnrollmentStoreError>;

    async fn replace_grant(
        &self,
        expected_grant_id: &str,
        expected_generation: u64,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError>;
}

/// Loaded authority-side enrollment runtime: the authority that may grant,
/// the store that persists grants, and the signer that seals them.
///
/// Constructed by `crate::adapters::local_node_enrollment_signer`; consumed
/// by node-local supervision. Kept next to the store port because the store
/// is the durable side of the runtime triple.
#[derive(Debug)]
pub struct NodeEnrollmentRuntime {
    pub authority: cynic_node_protocol::MachineEnrollmentAuthority,
    pub store: std::sync::Arc<dyn NodeEnrollmentStorePort>,
    pub signer: std::sync::Arc<dyn NodeEnrollmentSignerPort>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_error_preserves_conflict_and_storage_codes() {
        let conflict = NodeEnrollmentStoreError::conflict("stale generation 3 < 4");
        assert_eq!(conflict.code, "node_enrollment_conflict");
        assert!(conflict.to_string().contains("stale generation"));
        let storage = NodeEnrollmentStoreError::storage("surreal unreachable");
        assert_eq!(storage.code, "node_enrollment_storage");
        assert!(storage.to_string().contains("surreal unreachable"));
    }
}
