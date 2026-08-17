//! Typed Federation Envelopes, Leases, and Mesh Probe Schemas.
//!
//! Provides the immutable contracts for distributing tasks across sovereign CYNIC nodes
//! without global single-point-of-failure or authority amplification.

use serde::{Deserialize, Serialize};

use crate::domain_tasks::DomainTask;

pub const FEDERATED_TASK_ENVELOPE_SCHEMA: &str = "cynic.federation.task_envelope.v1";
pub const FEDERATED_NODE_LEASE_SCHEMA: &str = "cynic.federation.node_lease.v1";
pub const FEDERATED_MESH_PROBE_SCHEMA: &str = "cynic.federation.mesh_probe.v1";

/// Transport and routing envelope for a task distributed across federated nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedTaskEnvelope {
    #[serde(default = "default_task_envelope_schema")]
    pub schema: String,
    pub envelope_id: String,
    pub origin_node_id: String,
    pub target_node_id: Option<String>,
    pub task: DomainTask,
    pub fencing_token: u64,
    pub monotonic_budget_seconds: u64,
    pub signature_ed25519: Option<String>,
}

fn default_task_envelope_schema() -> String {
    FEDERATED_TASK_ENVELOPE_SCHEMA.to_string()
}

impl FederatedTaskEnvelope {
    pub fn new(
        envelope_id: impl Into<String>,
        origin_node_id: impl Into<String>,
        target_node_id: Option<String>,
        task: DomainTask,
        fencing_token: u64,
        monotonic_budget_seconds: u64,
    ) -> Self {
        Self {
            schema: FEDERATED_TASK_ENVELOPE_SCHEMA.to_string(),
            envelope_id: envelope_id.into(),
            origin_node_id: origin_node_id.into(),
            target_node_id,
            task,
            fencing_token,
            monotonic_budget_seconds,
            signature_ed25519: None,
        }
    }
}

/// Cryptographic lease guaranteeing exclusive node-level execution rights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedNodeLease {
    #[serde(default = "default_node_lease_schema")]
    pub schema: String,
    pub lease_id: String,
    pub node_id: String,
    pub task_id: String,
    pub fencing_token: u64,
    pub acquired_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub renew_count: u32,
}

fn default_node_lease_schema() -> String {
    FEDERATED_NODE_LEASE_SCHEMA.to_string()
}

impl FederatedNodeLease {
    pub fn is_valid_at(&self, now_unix_s: u64) -> bool {
        now_unix_s >= self.acquired_at_unix_s && now_unix_s < self.expires_at_unix_s
    }
}

/// Status of a federated node observed over the Synapse WireGuard overlay mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedMeshNodeStatus {
    pub node_id: String,
    pub role: String,
    pub synapse_ip: String,
    pub wireguard_reachable: bool,
    pub active_services: Vec<String>,
    pub rtt_ms: Option<f32>,
}

/// Complete report of the federated mesh observed across all declared registry nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedMeshProbeReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub local_node_id: String,
    pub total_declared_nodes: usize,
    pub reachable_nodes_count: usize,
    pub node_statuses: Vec<FederatedMeshNodeStatus>,
}
