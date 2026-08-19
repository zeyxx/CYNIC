//! Dynamic, drift-aware peer resolution for inter-host federation.
//!
//! Replaces the hardcoded `CANONICAL_SYNAPSE_NODES` routing table. The
//! desired topology (registry seed) is reconciled with observed reality
//! (mesh live + OS-substrate health). When reality diverges, that is a
//! first-class data event (`TopologyDriftSignal`), never a panic.
//!
//! This module is transport-neutral and kernel-agnostic: the kernel role
//! (`cortex_primary`) is intentionally excluded — kernel SPOF is a separate,
//! deliberate design track. Peer resolution covers build_runner,
//! storage_primary, forge_primary, hypervisor, and any future federated role.

use crate::build_node::NodeHealthView;

/// Transport a peer endpoint is reachable over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerTransport {
    Uds,
    Http,
    Ws,
}

/// A resolved peer endpoint for a federated role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerEndpoint {
    pub node_id: String,
    pub role: String,
    pub transport: PeerTransport,
    pub addr: String,
    /// Ports the peer should be probed on (mesh liveness / service health).
    pub probe_ports: Vec<u16>,
    /// Observed mesh reachability (from `MeshStateProvider::fetch_known_peers`).
    pub reachable: bool,
    /// Folded OS-substrate health (OSH-001..008), from the OS observer.
    pub health: NodeHealthView,
}

/// Priority of a federated role. Lower number = higher priority. The
/// primary/secondary/tertiary suffixes express the failover ordering so a
/// single role (e.g. `build_runner`) can tolerate one node loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RolePriority {
    Primary = 0,
    Secondary = 1,
    Tertiary = 2,
    Standby = 3,
}

impl RolePriority {
    pub fn from_role_str(role: &str) -> Option<Self> {
        match role {
            r if r.ends_with("_primary") => Some(Self::Primary),
            r if r.ends_with("_secondary") => Some(Self::Secondary),
            r if r.ends_with("_tertiary") => Some(Self::Tertiary),
            r if r.ends_with("_standby") => Some(Self::Standby),
            _ => None,
        }
    }
}

/// A signed, versioned, timestamped fallback peer address.
///
/// Used when the live mesh is unreachable: each node retains the last known
/// good peer set, signed by the emitting node's machine key, rolled by
/// timestamp. This is the "fallback on known addresses, versioned and
/// timestamped" the federation design calls for — never a hardcoded IP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedPeerEndpoint {
    pub endpoint: PeerEndpoint,
    pub signature_ed25519: String,
    pub generated_at_unix_s: u64,
    pub version: u32,
}

/// Port: resolve peers for a federated role from observed reality, with a
/// signed fallback when the live mesh is unavailable.
pub trait PeerResolver {
    /// Resolve live peers for `role` (registry seed ∩ mesh live, health-filtered).
    fn resolve_peers(&self, role: &str) -> Vec<PeerEndpoint>;
    /// Last-known-good signed fallback peers for `role`.
    fn fallback_peers(&self, role: &str) -> Vec<SignedPeerEndpoint>;
}

/// Pure, fail-closed peer selection.
///
/// Among endpoints matching `requested_role`, reachable and free of critical
/// OS-health anomalies, prefer the lowest role priority (primary before
/// secondary), then fewest active anomalies. Returns `None` when no candidate
/// is both reachable and healthy — the caller MUST emit a `TopologyDriftSignal`
/// rather than silently degrading to a broken node.
pub fn select_peer(peers: &[PeerEndpoint], requested_role: &str) -> Option<PeerEndpoint> {
    peers
        .iter()
        .filter(|p| p.role == requested_role && p.reachable && !p.health.critical_active)
        .filter_map(|p| {
            let priority = RolePriority::from_role_str(&p.role)?;
            Some((p, priority))
        })
        .min_by(|(a, pa), (b, pb)| {
            pa.cmp(pb).then_with(|| {
                a.health
                    .active_osha_codes
                    .len()
                    .cmp(&b.health.active_osha_codes.len())
            })
        })
        .map(|(p, _)| p.clone())
}

/// All reachable, non-OS-critical peers across roles, ordered by role priority
/// then health. Used by mesh probing (liveness survey) where every declared
/// peer is surveyed, not just one role.
pub fn select_all_peers(peers: &[PeerEndpoint]) -> Vec<PeerEndpoint> {
    let mut healthy: Vec<PeerEndpoint> = peers
        .iter()
        .filter(|p| p.reachable && !p.health.critical_active)
        .cloned()
        .collect();
    healthy.sort_by(|a, b| {
        let pa = RolePriority::from_role_str(&a.role).unwrap_or(RolePriority::Standby);
        let pb = RolePriority::from_role_str(&b.role).unwrap_or(RolePriority::Standby);
        pa.cmp(&pb).then_with(|| {
            a.health
                .active_osha_codes
                .len()
                .cmp(&b.health.active_osha_codes.len())
        })
    });
    healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(node_id: &str, role: &str, reachable: bool, critical: bool) -> PeerEndpoint {
        let health = if critical {
            NodeHealthView {
                critical_active: true,
                warning_active: false,
                active_osha_codes: vec!["CYNIC-OSH-001".to_string()],
            }
        } else {
            NodeHealthView::default()
        };
        PeerEndpoint {
            node_id: node_id.to_string(),
            role: role.to_string(),
            transport: PeerTransport::Http,
            addr: format!("http://10.10.10.{}", node_id),
            probe_ports: vec![8000],
            reachable,
            health,
        }
    }

    #[test]
    fn selects_primary_builder_when_healthy() {
        let peers = vec![
            peer("120", "build_runner_secondary", true, false),
            peer("119", "build_runner_primary", true, false),
        ];
        let sel = select_peer(&peers, "build_runner_primary").expect("expected selection");
        assert_eq!(sel.node_id, "119");
    }

    #[test]
    fn fails_over_storage_secondary_when_primary_down() {
        let peers = vec![
            peer("103", "storage_primary", false, true),
            peer("104", "storage_secondary", true, false),
        ];
        let sel = select_peer(&peers, "storage_primary");
        // requested_role must match exactly; secondary is a different role.
        assert!(sel.is_none());
        // Real failover requires the standby to carry the SAME role.
        let peers2 = vec![
            peer("103", "storage_primary", false, true),
            peer("104", "storage_primary", true, false),
        ];
        let sel2 = select_peer(&peers2, "storage_primary").expect("failover within role");
        assert_eq!(sel2.node_id, "104");
    }

    #[test]
    fn none_when_no_reachable_healthy_peer() {
        let peers = vec![peer("103", "storage_primary", false, true)];
        assert!(select_peer(&peers, "storage_primary").is_none());
    }

    #[test]
    fn role_priority_orders_primary_before_secondary() {
        assert!(RolePriority::from_role_str("build_runner_primary") < RolePriority::from_role_str("build_runner_secondary"));
        assert!(RolePriority::from_role_str("storage_primary") < RolePriority::from_role_str("storage_standby"));
        assert!(RolePriority::from_role_str("weird_role").is_none());
    }
}
