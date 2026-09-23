//! Storage topology desired state — IaaS target, not client policy.
//! SSOT: infra/storage_topology.json (cynic.storage_topology.v1), scellé contracts/foundation.json
//! Cynic: policy portable, substrate injecté — kernel ne dépend pas de ce type, seulement de StorageClientPolicy

use serde::{Deserialize, Serialize};

pub const STORAGE_TOPOLOGY_SCHEMA: &str = "cynic.storage_topology.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageTopologyDesired {
    pub schema: String,
    pub engine: StorageEngine,
    pub target_topology: TargetTopology,
    pub budget: StorageBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageEngine {
    pub backend: String,
    pub cluster_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetTopology {
    pub replicas: u32,
    pub residual_risk: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageBudget {
    pub wall_time_ms: u64,
    pub cpu_ms: Option<u64>,
}

impl Default for StorageTopologyDesired {
    fn default() -> Self {
        Self {
            schema: STORAGE_TOPOLOGY_SCHEMA.to_string(),
            engine: StorageEngine {
                backend: "standalone_process".to_string(),
                cluster_mode: "none".to_string(),
            },
            target_topology: TargetTopology {
                replicas: 1,
                residual_risk: "single_endpoint_no_replication".to_string(),
            },
            budget: StorageBudget {
                wall_time_ms: 5000,
                cpu_ms: Some(5000),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageTopologyObserved {
    pub schema: String,
    pub observed_at_ms: u64,
    pub endpoints: Vec<String>,
    pub leader: Option<String>,
    pub poll_cost_ms: Option<u64>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageTopologyReport {
    pub schema: String,
    pub desired_digest: String,
    pub observed_digest: String,
    #[serde(default)]
    pub pre_observed_digest: Option<String>,
    #[serde(default)]
    pub post_observed_digest: Option<String>,
    #[serde(default)]
    pub actions_applied: Vec<Vec<String>>,
    #[serde(default)]
    pub status: String,
    pub residual_unknowns: Vec<String>,
    pub proof_ref: Option<String>,
}

pub const STORAGE_TOPOLOGY_REPORT_SCHEMA: &str = "cynic.storage_topology_report.v1";
