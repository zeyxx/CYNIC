//! Local autonomy runtime — observe → decide → act (kernel-optional).
//!
//! Runs on every node, continuously. When the kernel is reachable, the node is
//! coordinated. When the kernel is dead, the node promotes to autonomous (local
//! decisions, retain core capabilities). This is the bottom-up federation.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cynic_node_protocol::local_autonomy::{
    AutonomyDecision, AutonomyObservation, LOCAL_AUTONOMY_STATE_SCHEMA, decide_autonomy,
};
use cynic_node_protocol::peer_resolver::PeerResolver;

use crate::os_health::{LocalOsHealthObserver, OsHealthObserver};
use crate::peer_resolver_mesh::MeshPeerResolver;

/// Local autonomy runtime — wires observation sources to the pure decision.
pub struct LocalAutonomyRuntime {
    registry_path: String,
    local_node_id: String,
    kernel_url: Option<String>,
    interval_secs: u64,
}

impl LocalAutonomyRuntime {
    pub fn new(local_node_id: impl Into<String>, registry_path: impl Into<String>) -> Self {
        Self {
            registry_path: registry_path.into(),
            local_node_id: local_node_id.into(),
            kernel_url: None,
            interval_secs: 30,
        }
    }

    pub fn with_kernel_url(mut self, url: impl Into<String>) -> Self {
        self.kernel_url = Some(url.into());
        self
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs;
        self
    }

    /// Run one observation-decision cycle. Returns the decision (caller acts on it).
    pub fn tick(&self) -> AutonomyDecision {
        let observation = self.observe();
        let decision = decide_autonomy(&observation);
        decision
    }

    /// Observe the local substrate + kernel reachability + mesh peers.
    fn observe(&self) -> AutonomyObservation {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // 1. Local OS health
        let os_observation = LocalOsHealthObserver.observe_os_health();
        let os_healthy = os_observation.dmesg_softlockup_count == 0
            && os_observation.dmesg_oom_count == 0
            && os_observation.available_mem_mib >= 2048;

        // 2. Kernel reachability
        let (kernel_reachable, kernel_lease_valid) = match &self.kernel_url {
            Some(url) => {
                let reachable = self.probe_kernel(url);
                (reachable, reachable)
            }
            None => (false, false),
        };

        // 3. Mesh peers (via PeerResolver)
        let mesh_reachable_peers = if Path::new(&self.registry_path).exists() {
            let resolver = MeshPeerResolver::new(&self.registry_path, &self.local_node_id);
            let all_peers: Vec<_> = ["build_runner", "storage", "forgejo", "kernel"]
                .iter()
                .flat_map(|role| resolver.resolve_peers(role))
                .filter(|p| p.reachable)
                .collect();
            all_peers.len()
        } else {
            0
        };

        // 4. Enrollment (simplified — in production, read from enrollment proof file)
        let enrollment_proven =
            Path::new("/var/lib/cynic/identity/enrollment-proven.json").exists();

        AutonomyObservation {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: now,
            kernel_reachable,
            kernel_lease_valid,
            local_os_healthy: os_healthy,
            mesh_reachable_peers,
            enrollment_proven,
        }
    }

    fn probe_kernel(&self, url: &str) -> bool {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();
        client.get(format!("{}/health", url)).send().is_ok()
    }

    /// Continuous loop — observe, decide, sleep.
    pub fn run_continuous<F>(&self, mut on_decision: F)
    where
        F: FnMut(&AutonomyObservation, &AutonomyDecision),
    {
        loop {
            let observation = self.observe();
            let decision = decide_autonomy(&observation);
            on_decision(&observation, &decision);
            std::thread::sleep(Duration::from_secs(self.interval_secs));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_node_protocol::local_autonomy::AutonomyMode;

    #[test]
    fn tick_returns_decision() {
        let runtime = LocalAutonomyRuntime::new("test-node", "/nonexistent/registry.json");
        let decision = runtime.tick();
        // Without registry + kernel, should be degraded or autonomous depending on OS
        assert!(!decision.blockers.is_empty() || decision.mode == AutonomyMode::Autonomous);
    }

    #[test]
    fn registry_missing_results_in_zero_peers() {
        let runtime = LocalAutonomyRuntime::new("test-node", "/nonexistent/registry.json");
        let observation = runtime.observe();
        assert_eq!(observation.mesh_reachable_peers, 0);
    }
}
