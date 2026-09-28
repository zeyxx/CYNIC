//! Node control store port — durable desired-state/status lifecycle.
//!
//! Canonical home since 2026-09-28. Extracted from
//! `cynic-kernel::domain::node_control` (Session 5 inversion fix) together
//! with its 9-line bootstrap shim, which had zero callers: the shim's
//! `store.map(...)` composition survives as `NodeControlRuntime::load`.
//!
//! Complements `super::node_control_plane_port` (pull/observe/publish over
//! `String` errors): this file is the durable store side with typed errors.
//! Error shape follows the node-crate convention (`code` + `detail`, no
//! `thiserror`: the previous enum had zero callers, so nothing observable
//! is lost. `#[async_trait]` (not native `async fn`) because the runtime
//! handle keeps `Arc<dyn NodeControlStorePort>`, which requires object
//! safety.

use std::sync::Arc;

use async_trait::async_trait;
use cynic_node_protocol::{NodeControlPlan, NodeDesiredState};

/// Durable failure of the node control store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlStoreError {
    pub code: String,
    pub detail: String,
}

impl NodeControlStoreError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    /// The record itself is malformed.
    pub fn invalid(detail: impl Into<String>) -> Self {
        Self::new("node_control_invalid", detail)
    }

    /// A concurrent or fencing conflict.
    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new("node_control_conflict", detail)
    }

    /// The store itself failed.
    pub fn storage(detail: impl Into<String>) -> Self {
        Self::new("node_control_storage", detail)
    }
}

impl std::fmt::Display for NodeControlStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for NodeControlStoreError {}

/// Boundary that persists per-node desired state and latest control status.
#[async_trait]
pub trait NodeControlStorePort: Send + Sync + std::fmt::Debug {
    async fn put_desired_state(
        &self,
        desired: NodeDesiredState,
    ) -> Result<(), NodeControlStoreError>;

    async fn desired_state(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeDesiredState>, NodeControlStoreError>;

    async fn put_status(&self, status: NodeControlPlan) -> Result<(), NodeControlStoreError>;

    async fn latest_status(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeControlPlan>, NodeControlStoreError>;
}

/// Loaded control runtime: the store behind node-local control supervision.
///
/// Formerly `cynic-kernel::domain::node_control::NodeControlRuntime` plus its
/// bootstrap `load_node_control_runtime` shim (`store.map(...)`); fused here
/// because the shim was pure composition with zero callers.
#[derive(Debug)]
pub struct NodeControlRuntime {
    pub store: Arc<dyn NodeControlStorePort>,
}

impl NodeControlRuntime {
    /// Compose a runtime when a store is available; `None` propagates the
    /// absent-store case exactly as the former bootstrap shim did.
    pub fn load(store: Option<Arc<dyn NodeControlStorePort>>) -> Option<Arc<Self>> {
        store.map(|store| Arc::new(Self { store }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_error_preserves_invalid_conflict_and_storage_codes() {
        let invalid = NodeControlStoreError::invalid("empty node_id");
        assert_eq!(invalid.code, "node_control_invalid");
        assert!(invalid.to_string().contains("empty node_id"));
        let conflict = NodeControlStoreError::conflict("stale generation");
        assert_eq!(conflict.code, "node_control_conflict");
        let storage = NodeControlStoreError::storage("surreal unreachable");
        assert_eq!(storage.code, "node_control_storage");
    }

    #[test]
    fn runtime_load_propagates_absent_store() {
        let loaded: Option<Arc<NodeControlRuntime>> = NodeControlRuntime::load(None);
        assert!(loaded.is_none());
    }
}
