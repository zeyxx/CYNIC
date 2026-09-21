use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DISCOVERY_CAPABILITIES_SCHEMA: &str = "cynic.discovery.capabilities.v1";
pub const DISCOVERY_NODE_SCHEMA: &str = "cynic.discovery.node.v1";
pub const DISCOVERY_EVENT_SCHEMA: &str = "cynic.discovery.event.v1";

/// Canonical roles in the CYNIC fleet. Distinct from `FleetRole` in
/// `federation_authz` — that one is RBAC (what a role *may* do), this one
/// is observation (what a node *is*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// Orchestrator — kernel primary, coordination bus.
    Orchestrator,
    /// Build runner — accepts workloads, produces proofs.
    Builder,
    /// Dev station — human operator, local development.
    DevStation,
    /// Inference proxy — serves LLM traffic.
    InferenceProxy,
    /// Storage primary — SurrealDB, ACL decisions.
    Storage,
    /// Forge host — webhooks, release triggers.
    Forge,
    /// Network observer — mesh, DNS, provider fallback.
    Network,
    /// Unknown — unclassified node.
    Unknown,
}

impl NodeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Orchestrator => "orchestrator",
            Self::Builder => "builder",
            Self::DevStation => "dev_station",
            Self::InferenceProxy => "inference_proxy",
            Self::Storage => "storage",
            Self::Forge => "forge",
            Self::Network => "network",
            Self::Unknown => "unknown",
        }
    }

    /// True if this role can execute build workloads.
    pub fn can_build(self) -> bool {
        matches!(self, Self::Builder)
    }

    /// True if this role serves inference traffic.
    pub fn can_inference(self) -> bool {
        matches!(self, Self::InferenceProxy)
    }
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single capability declared by a node. Typed, not a free string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Accept and execute build workloads.
    BuildExecute,
    /// Observe fleet topology (read-only).
    FleetObserve,
    /// Serve LLM inference traffic.
    InferenceServe,
    /// Host storage ACL and credential sync.
    StorageAcl,
    /// Administer forge repositories.
    ForgeAdmin,
    /// Supervise host OS substrate.
    HostSupervision,
    /// Generate cryptographic proofs.
    ProofGenerate,
    /// Expose `/capabilities` REST endpoint.
    CapabilityDeclare,
    /// Unclassified — unknown capability.
    Other(String),
}

impl Capability {
    pub fn as_str(&self) -> &str {
        match self {
            Self::BuildExecute => "build_execute",
            Self::FleetObserve => "fleet_observe",
            Self::InferenceServe => "inference_serve",
            Self::StorageAcl => "storage_acl",
            Self::ForgeAdmin => "forge_admin",
            Self::HostSupervision => "host_supervision",
            Self::ProofGenerate => "proof_generate",
            Self::CapabilityDeclare => "capability_declare",
            Self::Other(s) => s.as_str(),
        }
    }

    /// True if this capability requires a network endpoint.
    pub fn requires_endpoint(&self) -> bool {
        matches!(
            self,
            Self::BuildExecute
                | Self::InferenceServe
                | Self::StorageAcl
                | Self::ForgeAdmin
                | Self::CapabilityDeclare
        )
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub(crate) fn timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
