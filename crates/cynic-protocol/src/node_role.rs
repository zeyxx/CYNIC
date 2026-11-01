use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Canonical top-level topological role of a node in the CYNIC mesh.
///
/// Follows the agnostic distributed systems standard:
/// A node's fundamental identity in the mesh topology is one of 5 invariant roles:
/// - `Compute`: Workload execution (CPU/RAM/GPU, compilation, sandboxed tasks, tensor inference).
/// - `Storage`: Durable state and data persistence (databases, SMR, block storage).
/// - `Gateway`: Network perimeter and boundary (DMZ, bastions, VPN mesh overlay, relays).
/// - `Control`: Mesh coordination, consensus, governance, and scheduling authority.
/// - `Workstation`: Direct operator interactive access endpoint.
///
/// Specific workload specializations (e.g. "cargo_build", "tensor_inference", "wireguard_overlay")
/// are expressed via capability flags and services, not hardcoded software roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeRole {
    /// Workload execution and compute (CPU/GPU workers, compilers, sandbox runners).
    Compute,
    /// Durable persistence and state (databases, structured stores, SMR logs).
    Storage,
    /// Network perimeter and transit (DMZ, overlay VPN, SSH bastions, network relays).
    Gateway,
    /// Mesh coordination, scheduling, consensus, and orchestration authority.
    Control,
    /// Interactive operator access workstation / client terminal.
    Workstation,
}

impl NodeRole {
    pub const COMPUTE_STR: &'static str = "compute";
    pub const STORAGE_STR: &'static str = "storage";
    pub const GATEWAY_STR: &'static str = "gateway";
    pub const CONTROL_STR: &'static str = "control";
    pub const WORKSTATION_STR: &'static str = "workstation";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compute => Self::COMPUTE_STR,
            Self::Storage => Self::STORAGE_STR,
            Self::Gateway => Self::GATEWAY_STR,
            Self::Control => Self::CONTROL_STR,
            Self::Workstation => Self::WORKSTATION_STR,
        }
    }

    pub fn is_compute(self) -> bool {
        matches!(self, Self::Compute)
    }

    pub fn is_storage(self) -> bool {
        matches!(self, Self::Storage)
    }

    pub fn is_gateway(self) -> bool {
        matches!(self, Self::Gateway)
    }

    pub fn is_control(self) -> bool {
        matches!(self, Self::Control)
    }

    pub fn is_workstation(self) -> bool {
        matches!(self, Self::Workstation)
    }

    /// Parses canonical strings as well as legacy software/metaphor names for seamless backward-compatibility.
    pub fn parse_with_legacy_fallback(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            // Canonical names
            "compute" => Some(Self::Compute),
            "storage" => Some(Self::Storage),
            "gateway" => Some(Self::Gateway),
            "control" => Some(Self::Control),
            "workstation" => Some(Self::Workstation),

            // Legacy compute aliases
            "build_runner" | "builder" | "embedding_runner" | "inference_runner" | "inference_proxy"
            | "agent_host" | "sandbox_runner" | "hypervisor" => Some(Self::Compute),

            // Legacy storage aliases
            "storage_primary" | "forge_primary" | "forge" | "database_server" => Some(Self::Storage),

            // Legacy gateway aliases
            "mesh_coordinator" | "ingress_edge" | "relay_host" | "network" | "bastion" => Some(Self::Gateway),

            // Legacy control aliases
            "kernel_primary" | "orchestrator" | "kernel_host" => Some(Self::Control),

            // Legacy workstation aliases
            "cortex_host" | "dev_station" | "workstation_portable" => Some(Self::Workstation),

            _ => None,
        }
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

impl FromStr for NodeRole {
    type Err = NodeRoleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_with_legacy_fallback(s).ok_or_else(|| NodeRoleParseError(s.to_string()))
    }
}

impl Serialize for NodeRole {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NodeRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRoleParseError(pub String);

impl fmt::Display for NodeRoleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown or invalid node role: '{}'", self.0)
    }
}

impl std::error::Error for NodeRoleParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_roundtrip() {
        let canonical_cases = vec![
            (NodeRole::Compute, "compute"),
            (NodeRole::Storage, "storage"),
            (NodeRole::Gateway, "gateway"),
            (NodeRole::Control, "control"),
            (NodeRole::Workstation, "workstation"),
        ];

        for (role, s) in canonical_cases {
            assert_eq!(role.as_str(), s);
            assert_eq!(role.to_string(), s);
            assert_eq!(NodeRole::from_str(s).unwrap(), role);
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, format!("\"{}\"", s));
            let deserialized: NodeRole = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }
    }

    #[test]
    fn legacy_fallback_deserialization() {
        assert_eq!(NodeRole::from_str("build_runner").unwrap(), NodeRole::Compute);
        assert_eq!(NodeRole::from_str("embedding_runner").unwrap(), NodeRole::Compute);
        assert_eq!(NodeRole::from_str("hypervisor").unwrap(), NodeRole::Compute);
        assert_eq!(NodeRole::from_str("storage_primary").unwrap(), NodeRole::Storage);
        assert_eq!(NodeRole::from_str("forge_primary").unwrap(), NodeRole::Storage);
        assert_eq!(NodeRole::from_str("mesh_coordinator").unwrap(), NodeRole::Gateway);
        assert_eq!(NodeRole::from_str("ingress_edge").unwrap(), NodeRole::Gateway);
        assert_eq!(NodeRole::from_str("kernel_primary").unwrap(), NodeRole::Control);
        assert_eq!(NodeRole::from_str("cortex_host").unwrap(), NodeRole::Workstation);
        assert_eq!(NodeRole::from_str("workstation_portable").unwrap(), NodeRole::Workstation);
    }
}
