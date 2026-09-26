use std::sync::Arc;

use async_trait::async_trait;

pub use cynic_protocol::{AgentSessionAuthority, AgentSessionRecord};

#[derive(Debug, thiserror::Error)]
pub enum AgentSessionStoreError {
    #[error("agent session record is invalid: {0}")]
    Invalid(String),
    #[error("agent session store conflict: {0}")]
    Conflict(String),
    #[error("agent session store failed: {0}")]
    Storage(String),
}

#[async_trait]
pub trait AgentSessionStorePort: Send + Sync + std::fmt::Debug {
    async fn insert(&self, record: AgentSessionRecord) -> Result<(), AgentSessionStoreError>;

    async fn resolve_active(
        &self,
        session_id: &str,
        now_unix_ms: u64,
    ) -> Result<Option<AgentSessionRecord>, AgentSessionStoreError>;

    async fn resolve_active_credential(
        &self,
        credential_digest: &str,
        now_unix_ms: u64,
    ) -> Result<Option<AgentSessionRecord>, AgentSessionStoreError>;

    async fn revoke(
        &self,
        session_id: &str,
        expected_generation: u64,
        revoked_at_unix_ms: u64,
        reason: &str,
        evidence_ref: &str,
    ) -> Result<AgentSessionRecord, AgentSessionStoreError>;

    async fn rotate(
        &self,
        previous_session_id: &str,
        expected_generation: u64,
        replacement: AgentSessionRecord,
        rotated_at_unix_ms: u64,
        evidence_ref: &str,
    ) -> Result<AgentSessionRecord, AgentSessionStoreError>;

    async fn expire(&self, now_unix_ms: u64) -> Result<u64, AgentSessionStoreError>;
}

pub struct AgentSessionRuntime {
    pub authority: AgentSessionAuthority,
    pub store: Arc<dyn AgentSessionStorePort>,
    pub node_id: String,
    pub boot_id: String,
}

impl std::fmt::Debug for AgentSessionRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentSessionRuntime")
            .field("authority_revision", &self.authority.revision)
            .field("authority_digest", &self.authority.authority_digest)
            .field("node_id", &self.node_id)
            .field("has_boot_identity", &!self.boot_id.is_empty())
            .finish_non_exhaustive()
    }
}

pub fn local_transport_binding_sha256(
    node_id: &str,
    boot_id: &str,
    process_id: Option<u32>,
    process_start_time_ticks: Option<u64>,
    user_id: u32,
    group_id: u32,
) -> Result<String, serde_json::Error> {
    cynic_node_protocol::digest_json(&serde_json::json!({
        "schema": "cynic.local_peer_transport_binding.v1",
        "node_id": node_id,
        "boot_id": boot_id,
        "process_id": process_id,
        "process_start_time_ticks": process_start_time_ticks,
        "user_id": user_id,
        "group_id": group_id,
    }))
}
