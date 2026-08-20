//! Emergence runtime — gossip-based coordination without a kernel.
//!
//! Each node runs this loop when the kernel is dead. It floods its local state
//! to the mesh, receives peers' states, and runs a deterministic election to
//! decide who assumes which role. Because the election is pure and the state
//! is shared, every node converges to the same answer.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cynic_protocol::emergence::{
    EMERGENCE_STATE_SCHEMA, EmergenceDecision, EmergenceState, PeerState, elect_for_role,
    health_score_from_os, should_assume_role,
};
use cynic_protocol::local_autonomy::{AutonomyMode, RetainedCapabilities};
use cynic_protocol::peer_resolver::PeerResolver;

use crate::local_autonomy_runtime::LocalAutonomyRuntime;
use crate::os_health::{LocalOsHealthObserver, OsHealthObserver};
use crate::peer_resolver_mesh::MeshPeerResolver;

/// Emergence runtime — wires discovery + autonomy to the election rule.
pub struct EmergenceRuntime {
    local_node_id: String,
    registry_path: String,
    autonomy_runtime: LocalAutonomyRuntime,
    interval_secs: u64,
    /// Last known state from each peer (gossiped).
    peer_states: BTreeMap<String, PeerState>,
}

impl EmergenceRuntime {
    pub fn new(local_node_id: impl Into<String>, registry_path: impl Into<String>) -> Self {
        let node_id = local_node_id.into();
        let reg_path = registry_path.into();
        let autonomy = LocalAutonomyRuntime::new(&node_id, &reg_path);
        Self {
            local_node_id: node_id,
            registry_path: reg_path,
            autonomy_runtime: autonomy,
            interval_secs: 30,
            peer_states: BTreeMap::new(),
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs;
        self
    }

    /// Run one emergence cycle: observe → build state → elect → decide.
    pub fn tick(&mut self) -> Vec<EmergenceDecision> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // 1. Local autonomy observation
        let autonomy_decision = self.autonomy_runtime.tick();

        // 2. Build local peer state
        let local_state = self.build_local_state(now, &autonomy_decision.mode);

        // 3. Discover peers via MeshPeerResolver
        let peers = self.discover_peers(now);

        // 4. Merge gossiped peer states (in production, this would be real gossip;
        //    for now we use direct observation via PeerResolver)
        let state = EmergenceState {
            schema: EMERGENCE_STATE_SCHEMA.to_string(),
            observed_at_unix_s: now,
            local_node_id: self.local_node_id.clone(),
            peers,
            local: local_state,
        };

        // 5. Elect for each known role
        let roles = ["build_runner", "storage", "forgejo", "kernel"];
        let mut decisions = vec![];
        for role in roles {
            let decision = elect_for_role(&state, role);
            decisions.push(decision);
        }

        decisions
    }

    /// Build this node's peer state from local observation.
    fn build_local_state(&self, now: u64, mode: &AutonomyMode) -> PeerState {
        let os_obs = LocalOsHealthObserver.observe_os_health();
        let health_score = health_score_from_os(
            os_obs.dmesg_softlockup_count,
            os_obs.dmesg_oom_count,
            os_obs.available_mem_mib,
        );

        // Read declared role from registry
        let (role, capabilities) = self.read_local_role();

        PeerState {
            node_id: self.local_node_id.clone(),
            role,
            mode: *mode,
            health_score,
            last_seen_unix_s: now,
            capabilities,
        }
    }

    /// Read this node's declared role from the registry.
    fn read_local_role(&self) -> (String, Vec<String>) {
        if !Path::new(&self.registry_path).exists() {
            return ("unknown".to_string(), vec![]);
        }
        let bytes = match std::fs::read(&self.registry_path) {
            Ok(b) => b,
            Err(_) => return ("unknown".to_string(), vec![]),
        };
        let value: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return ("unknown".to_string(), vec![]),
        };
        if let Some(nodes) = value.get("nodes").and_then(|v| v.as_array()) {
            for node in nodes {
                let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if node_id == self.local_node_id {
                    let role = node
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let capabilities = node
                        .get("services")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| {
                                    s.get("name").and_then(|v| v.as_str()).map(String::from)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    return (role, capabilities);
                }
            }
        }
        ("unknown".to_string(), vec![])
    }

    /// Discover peers via MeshPeerResolver.
    fn discover_peers(&self, now: u64) -> Vec<PeerState> {
        if !Path::new(&self.registry_path).exists() {
            return vec![];
        }
        let resolver = MeshPeerResolver::new(&self.registry_path, &self.local_node_id);
        let mut peers = vec![];
        for role in ["build_runner", "storage", "forgejo", "kernel"] {
            for peer in resolver.resolve_peers(role) {
                let os_healthy = !peer.health.critical_active;
                let health_score = if os_healthy { 80 } else { 20 };
                peers.push(PeerState {
                    node_id: peer.node_id,
                    role: peer.role,
                    mode: if peer.reachable {
                        AutonomyMode::Autonomous
                    } else {
                        AutonomyMode::Degraded
                    },
                    health_score,
                    last_seen_unix_s: now,
                    capabilities: vec![role.to_string()],
                });
            }
        }
        peers
    }

    /// Check if this node should assume a role.
    pub fn should_assume(&mut self, role: &str) -> bool {
        let decisions = self.tick();
        decisions.iter().any(|d| {
            d.assumed_role.as_deref() == Some(role)
                && d.candidates.first() == Some(&self.local_node_id)
        })
    }

    /// Continuous loop — observe, elect, sleep.
    pub fn run_continuous<F>(&mut self, mut on_decision: F)
    where
        F: FnMut(&[EmergenceDecision]),
    {
        loop {
            let decisions = self.tick();
            on_decision(&decisions);
            std::thread::sleep(Duration::from_secs(self.interval_secs));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_returns_decisions_for_all_roles() {
        let mut runtime = EmergenceRuntime::new("test-node", "/nonexistent/registry.json");
        let decisions = runtime.tick();
        assert_eq!(decisions.len(), 4); // build_runner, storage, forgejo, kernel
    }

    #[test]
    fn should_assume_returns_false_without_registry() {
        let mut runtime = EmergenceRuntime::new("test-node", "/nonexistent/registry.json");
        assert!(!runtime.should_assume("build_runner"));
    }

    #[test]
    fn read_local_role_returns_unknown_without_registry() {
        let runtime = EmergenceRuntime::new("test-node", "/nonexistent/registry.json");
        let (role, caps) = runtime.read_local_role();
        assert_eq!(role, "unknown");
        assert!(caps.is_empty());
    }
}
