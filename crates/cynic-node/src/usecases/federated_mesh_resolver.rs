//! Local registry-backed mesh resolver.
//!
//! Replaces direct reads of the hardcoded `CANONICAL_SYNAPSE_NODES` const with
//! a `PeerResolver` port implementation. The desired topology (registry seed)
//! is reconciled with observed reality (mesh live + OS-substrate health) via
//! the pure `select_peer` / `select_all_peers` helpers in `cynic-protocol`.
//!
//! Today the seed is supplied from the declared registry (the const, pending a
//! versioned/signed `infra/synapse-nodes.seed.json`). Mesh-live observation and
//! the signed `known-peers.vN` fallback are wired as the observation layer
//! matures; the resolver already exposes `fallback_peers` so callers can
//! degrade to last-known-good addresses without a hardcoded IP.

use cynic_protocol::peer_resolver::{
    PeerEndpoint, PeerResolver, PeerTransport, SignedPeerEndpoint,
};
use cynic_protocol::build_node::NodeHealthView;

/// A declared synapse node (mirrors the prior `DeclaredSynapseNode` shape so
/// the existing seed const can be the initial source without churn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedSynapseNode {
    pub id: String,
    pub role: String,
    pub ip: String,
    pub probe_ports: Vec<u16>,
}

impl SeedSynapseNode {
    pub fn from_static(nodes: &[crate::usecases::federated_mesh_probe_uc::DeclaredSynapseNode]) -> Vec<Self> {
        nodes
            .iter()
            .map(|n| SeedSynapseNode {
                id: n.id.to_string(),
                role: n.role.to_string(),
                ip: n.ip.to_string(),
                probe_ports: n.probe_ports.to_vec(),
            })
            .collect()
    }
}

fn transport_for(role: &str, ip: &str) -> PeerTransport {
    if role == "cortex_primary" || ip.starts_with("127.") {
        PeerTransport::Uds
    } else if role == "storage_primary" {
        PeerTransport::Ws
    } else {
        PeerTransport::Http
    }
}

/// Resolver backed by a local seed (registry / declared nodes).
pub struct LocalRegistryMeshResolver {
    seed: Vec<SeedSynapseNode>,
}

impl LocalRegistryMeshResolver {
    pub fn from_seed(seed: Vec<SeedSynapseNode>) -> Self {
        Self { seed }
    }

    pub fn from_declared(
        nodes: &[crate::usecases::federated_mesh_probe_uc::DeclaredSynapseNode],
    ) -> Self {
        Self::from_seed(SeedSynapseNode::from_static(nodes))
    }

    fn to_endpoint(&self, node: &SeedSynapseNode) -> PeerEndpoint {
        PeerEndpoint {
            node_id: node.id.clone(),
            role: node.role.clone(),
            transport: transport_for(&node.role, &node.ip),
            // Seed is the *desired* topology; addr carries the declared IP so
            // the probe/observer can confirm reachability.
            addr: node.ip.clone(),
            probe_ports: node.probe_ports.clone(),
            // Desired-until-observed: a seed node is assumed reachable until a
            // mesh probe or OS-health observation proves otherwise.
            reachable: true,
            health: NodeHealthView::default(),
        }
    }
}

impl PeerResolver for LocalRegistryMeshResolver {
    fn resolve_peers(&self, role: &str) -> Vec<PeerEndpoint> {
        self.seed
            .iter()
            .filter(|n| n.role == role)
            .map(|n| self.to_endpoint(n))
            .collect()
    }

    fn fallback_peers(&self, _role: &str) -> Vec<SignedPeerEndpoint> {
        // Wired to the signed `known-peers.vN.<timestamp>.json` once the
        // observer layer produces it. Until then, no hardcoded fallback.
        vec![]
    }
}

impl LocalRegistryMeshResolver {
    /// All seed peers (every declared role), for mesh-wide liveness probing.
    pub fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        self.seed.iter().map(|n| self.to_endpoint(n)).collect()
    }
}

/// Helper: pick the highest-priority reachable, healthy peer for `role`.
/// Returns `None` when no candidate is dispatchable — caller must then emit a
/// `TopologyDriftSignal` (drift is data, not a crash).
pub fn select_peer_for_role(
    resolver: &LocalRegistryMeshResolver,
    role: &str,
) -> Option<PeerEndpoint> {
    cynic_protocol::peer_resolver::select_peer(&resolver.resolve_peers(role), role)
}

/// Helper: all reachable, healthy seed peers, ordered by role priority.
pub fn select_all_healthy(resolver: &LocalRegistryMeshResolver) -> Vec<PeerEndpoint> {
    cynic_protocol::peer_resolver::select_all_peers(&resolver.resolve_all_peers())
}
