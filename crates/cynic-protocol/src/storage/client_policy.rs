//! Storage client policy — liste ordonnée endpoints + failover, consommée par cynic-kernel
//! SSOT: infra/storage_client_policy.json (cynic.storage_client_policy.v1)
//! Kernel ne dépend que de ce type, pas de StorageTopologyDesired (découplage IaaS vs plan données)

use serde::{Deserialize, Serialize};

pub const STORAGE_CLIENT_POLICY_SCHEMA: &str = "cynic.storage_client_policy.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageClientPolicy {
    pub schema: String,
    pub client_policy: ClientPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientPolicy {
    pub endpoints: Vec<String>,
    pub failover_mode: String,
    pub write_quorum: u32,
    pub timeout_ms: u64,
}

impl Default for StorageClientPolicy {
    fn default() -> Self {
        Self {
            schema: STORAGE_CLIENT_POLICY_SCHEMA.to_string(),
            client_policy: ClientPolicy {
                endpoints: vec!["http://127.0.0.1:8000".to_string()],
                failover_mode: "ordered_fallback".to_string(),
                write_quorum: 1,
                timeout_ms: 5000,
            },
        }
    }
}
