//! Discovery — typed capability declaration and node observation.
//!
//! When multiple surfaces (contracts, protocol, CLI, MCP, skills, runtime)
//! need to know what the fleet *can* do (not what it *may* do — that's
//! `federation_authz`), this module provides the types to declare, observe,
//! and aggregate capabilities.
//!
//! Phase A: types only. Phase B: adapters in `cynic-discovery-host`.
//! Phase C: `/capabilities` endpoint in `cynic-kernel`.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Schemas ──────────────────────────────────────────────────────────────────

pub const DISCOVERY_CAPABILITIES_SCHEMA: &str = "cynic.discovery.capabilities.v1";
pub const DISCOVERY_NODE_SCHEMA: &str = "cynic.discovery.node.v1";
pub const DISCOVERY_EVENT_SCHEMA: &str = "cynic.discovery.event.v1";

// ── Node role ────────────────────────────────────────────────────────────────

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

// ── Capability ───────────────────────────────────────────────────────────────

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

// ── Node capabilities ────────────────────────────────────────────────────────

/// Complete capability declaration for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Stable node identifier (matches registry).
    pub node_id: String,
    /// Observed role.
    pub role: NodeRole,
    /// Network endpoint for this node (IP or hostname).
    pub endpoint: String,
    /// Declared capabilities.
    pub capabilities: Vec<Capability>,
    /// Unix timestamp of last observation.
    pub observed_at_unix_s: u64,
}

impl NodeCapabilities {
    pub fn new(
        node_id: &str,
        role: NodeRole,
        endpoint: &str,
        capabilities: Vec<Capability>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            role,
            endpoint: endpoint.to_string(),
            capabilities,
            observed_at_unix_s: timestamp_now(),
        }
    }

    /// True if this node declares a specific capability.
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

// ── Fleet capabilities ───────────────────────────────────────────────────────

/// Aggregated capabilities across the entire fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetCapabilities {
    /// Schema version.
    pub schema: String,
    /// Unix timestamp of aggregation.
    pub observed_at_unix_s: u64,
    /// All nodes observed.
    pub nodes: Vec<NodeCapabilities>,
    /// Nodes that failed to respond (residual unknowns).
    pub failed_nodes: Vec<(String, String)>,
}

impl FleetCapabilities {
    pub fn new(nodes: Vec<NodeCapabilities>) -> Self {
        Self {
            schema: DISCOVERY_CAPABILITIES_SCHEMA.to_string(),
            observed_at_unix_s: timestamp_now(),
            nodes,
            failed_nodes: Vec::new(),
        }
    }

    /// Find all nodes with a specific capability.
    pub fn find_by_capability(&self, cap: &Capability) -> Vec<&NodeCapabilities> {
        self.nodes
            .iter()
            .filter(|n| n.has_capability(cap))
            .collect()
    }

    /// Find all nodes with a specific role.
    pub fn find_by_role(&self, role: NodeRole) -> Vec<&NodeCapabilities> {
        self.nodes.iter().filter(|n| n.role == role).collect()
    }

    /// Count of live (responding) nodes.
    pub fn live_count(&self) -> usize {
        self.nodes.len()
    }

    /// Count of failed nodes.
    pub fn failed_count(&self) -> usize {
        self.failed_nodes.len()
    }

    /// True if the fleet has at least one build executor.
    pub fn has_builder(&self) -> bool {
        !self
            .find_by_capability(&Capability::BuildExecute)
            .is_empty()
    }
}

// ── Discovery event ──────────────────────────────────────────────────────────

/// Events emitted when fleet topology changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiscoveryEvent {
    /// A new node joined and declared its capabilities.
    NodeJoined {
        node_id: String,
        role: NodeRole,
        capabilities: Vec<Capability>,
    },
    /// A node departed the fleet.
    NodeDeparted { node_id: String },
    /// A node's capabilities changed.
    CapabilityChanged {
        node_id: String,
        added: Vec<Capability>,
        removed: Vec<Capability>,
    },
}

// ── Port ─────────────────────────────────────────────────────────────────────

/// Pure port for observing fleet capabilities. Adapters implement this.
pub trait DiscoveryObservationPort {
    fn observe(&self) -> Result<FleetCapabilities, String>;
}

/// Pure port for observing a single node's capabilities.
pub trait NodeDiscoveryObservationPort {
    fn observe_node(&self, node_id: &str) -> Result<NodeCapabilities, String>;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_role_roundtrip() {
        for role in [
            NodeRole::Orchestrator,
            NodeRole::Builder,
            NodeRole::DevStation,
            NodeRole::InferenceProxy,
            NodeRole::Storage,
            NodeRole::Forge,
            NodeRole::Network,
            NodeRole::Unknown,
        ] {
            let s = role.as_str();
            let parsed: NodeRole = serde_json::from_str(&format!("\"{s}\"")).unwrap();
            assert_eq!(role, parsed);
        }
    }

    #[test]
    fn capability_requires_endpoint() {
        assert!(Capability::BuildExecute.requires_endpoint());
        assert!(Capability::InferenceServe.requires_endpoint());
        assert!(!Capability::FleetObserve.requires_endpoint());
        assert!(!Capability::HostSupervision.requires_endpoint());
    }

    #[test]
    fn node_capabilities_has() {
        let node = NodeCapabilities::new(
            "test-node",
            NodeRole::Builder,
            "10.0.0.1:8080",
            vec![Capability::BuildExecute, Capability::FleetObserve],
        );
        assert!(node.has_capability(&Capability::BuildExecute));
        assert!(node.has_capability(&Capability::FleetObserve));
        assert!(!node.has_capability(&Capability::InferenceServe));
    }

    #[test]
    fn fleet_capabilities_find() {
        let fleet = FleetCapabilities::new(vec![
            NodeCapabilities::new(
                "builder-1",
                NodeRole::Builder,
                "10.0.0.1:8080",
                vec![Capability::BuildExecute],
            ),
            NodeCapabilities::new(
                "orch-1",
                NodeRole::Orchestrator,
                "10.0.0.2:8080",
                vec![Capability::FleetObserve],
            ),
        ]);
        let builders = fleet.find_by_role(NodeRole::Builder);
        assert_eq!(builders.len(), 1);
        assert_eq!(builders[0].node_id, "builder-1");
        assert!(fleet.has_builder());
    }

    #[test]
    fn discovery_event_serializes() {
        let event = DiscoveryEvent::NodeJoined {
            node_id: "node-1".to_string(),
            role: NodeRole::Builder,
            capabilities: vec![Capability::BuildExecute],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"node_joined\""));
        assert!(json.contains("\"node_id\":\"node-1\""));
    }

    #[test]
    fn schema_constants_unique() {
        assert_ne!(DISCOVERY_CAPABILITIES_SCHEMA, DISCOVERY_NODE_SCHEMA);
        assert_ne!(DISCOVERY_CAPABILITIES_SCHEMA, DISCOVERY_EVENT_SCHEMA);
    }
}
