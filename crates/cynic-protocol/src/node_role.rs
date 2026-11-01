use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Canonical host/node capability role designating a node's declared specialization in the mesh.
///
/// Derived from `infra/registry.json` and canonicalized in `contracts/glossary.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    BuildRunner,
    EmbeddingRunner,
    Hypervisor,
    CortexHost,
    KernelPrimary,
    StoragePrimary,
    WorkstationPortable,
    AgentHost,
    RelayHost,
    ForgePrimary,
    MeshCoordinator,
    IngressEdge,
    #[serde(untagged)]
    Custom(String),
}

impl NodeRole {
    pub const BUILD_RUNNER_STR: &'static str = "build_runner";
    pub const EMBEDDING_RUNNER_STR: &'static str = "embedding_runner";
    pub const HYPERVISOR_STR: &'static str = "hypervisor";
    pub const CORTEX_HOST_STR: &'static str = "cortex_host";
    pub const KERNEL_PRIMARY_STR: &'static str = "kernel_primary";
    pub const STORAGE_PRIMARY_STR: &'static str = "storage_primary";
    pub const WORKSTATION_PORTABLE_STR: &'static str = "workstation_portable";
    pub const AGENT_HOST_STR: &'static str = "agent_host";
    pub const RELAY_HOST_STR: &'static str = "relay_host";
    pub const FORGE_PRIMARY_STR: &'static str = "forge_primary";
    pub const MESH_COORDINATOR_STR: &'static str = "mesh_coordinator";
    pub const INGRESS_EDGE_STR: &'static str = "ingress_edge";

    pub fn as_str(&self) -> &str {
        match self {
            Self::BuildRunner => Self::BUILD_RUNNER_STR,
            Self::EmbeddingRunner => Self::EMBEDDING_RUNNER_STR,
            Self::Hypervisor => Self::HYPERVISOR_STR,
            Self::CortexHost => Self::CORTEX_HOST_STR,
            Self::KernelPrimary => Self::KERNEL_PRIMARY_STR,
            Self::StoragePrimary => Self::STORAGE_PRIMARY_STR,
            Self::WorkstationPortable => Self::WORKSTATION_PORTABLE_STR,
            Self::AgentHost => Self::AGENT_HOST_STR,
            Self::RelayHost => Self::RELAY_HOST_STR,
            Self::ForgePrimary => Self::FORGE_PRIMARY_STR,
            Self::MeshCoordinator => Self::MESH_COORDINATOR_STR,
            Self::IngressEdge => Self::INGRESS_EDGE_STR,
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn is_build_runner(&self) -> bool {
        matches!(self, Self::BuildRunner)
    }

    pub fn is_embedding_runner(&self) -> bool {
        matches!(self, Self::EmbeddingRunner)
    }
}

impl AsRef<str> for NodeRole {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for NodeRole {
    fn from(s: &str) -> Self {
        match s {
            Self::BUILD_RUNNER_STR => Self::BuildRunner,
            Self::EMBEDDING_RUNNER_STR => Self::EmbeddingRunner,
            Self::HYPERVISOR_STR => Self::Hypervisor,
            Self::CORTEX_HOST_STR => Self::CortexHost,
            Self::KERNEL_PRIMARY_STR => Self::KernelPrimary,
            Self::STORAGE_PRIMARY_STR => Self::StoragePrimary,
            Self::WORKSTATION_PORTABLE_STR => Self::WorkstationPortable,
            Self::AGENT_HOST_STR => Self::AgentHost,
            Self::RELAY_HOST_STR => Self::RelayHost,
            Self::FORGE_PRIMARY_STR => Self::ForgePrimary,
            Self::MESH_COORDINATOR_STR => Self::MeshCoordinator,
            Self::INGRESS_EDGE_STR => Self::IngressEdge,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl FromStr for NodeRole {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_role_roundtrip() {
        let roles = vec![
            (NodeRole::BuildRunner, "build_runner"),
            (NodeRole::EmbeddingRunner, "embedding_runner"),
            (NodeRole::Hypervisor, "hypervisor"),
            (NodeRole::CortexHost, "cortex_host"),
            (NodeRole::KernelPrimary, "kernel_primary"),
            (NodeRole::StoragePrimary, "storage_primary"),
            (NodeRole::WorkstationPortable, "workstation_portable"),
            (NodeRole::AgentHost, "agent_host"),
            (NodeRole::RelayHost, "relay_host"),
            (NodeRole::ForgePrimary, "forge_primary"),
            (NodeRole::MeshCoordinator, "mesh_coordinator"),
            (NodeRole::IngressEdge, "ingress_edge"),
            (NodeRole::Custom("custom_node".to_string()), "custom_node"),
        ];

        for (role, expected_str) in roles {
            assert_eq!(role.as_str(), expected_str);
            assert_eq!(role.to_string(), expected_str);
            assert_eq!(NodeRole::from(expected_str), role);
            let json = serde_json::to_string(&role).unwrap();
            let deserialized: NodeRole = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }
    }
}
