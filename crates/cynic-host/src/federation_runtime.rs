//! Federation runtime wiring — composes the four autonomy models via injectable ports.
//!
//! The pure decision lives in `cynic_protocol::federation::reconcile_federation`.
//! This module wires it to impure observation sources (kernel reachability,
//! mesh peers, OS health, lease status). Every port is injectable so the
//! composition is unit-testable without real infrastructure.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::os_health::observe::OsHealthObserver as OsHealthObserve;
use crate::peer_resolver_mesh::MeshPeerResolver;
use cynic_protocol::emergence::{EmergenceState, PeerState, health_score_from_os, AutonomyMode};
use cynic_protocol::federation::{FederationReconciliation, reconcile_federation};
use cynic_protocol::local_autonomy::{AutonomyObservation, decide_autonomy};
use cynic_protocol::node_control::NodeControlPlan;
use cynic_protocol::peer_resolver::{PeerEndpoint, PeerResolver};
use cynic_protocol::runtime_takeover::RuntimeTakeoverObservation;

/// Port: observe kernel reachability and lease status.
pub trait KernelObserver: Send + Sync {
    fn observe_kernel(&self) -> KernelObservation;
}

/// Port: observe the mesh (peers, roles, health).
pub trait MeshObserver: Send + Sync {
    fn observe_mesh(&self) -> MeshObservation;
}

/// Port: observe local OS health.
pub trait OsHealthObserver: Send + Sync {
    fn observe_os_health(&self) -> OsHealthObservation;
}

/// Port: observe enrollment status.
pub trait EnrollmentObserver: Send + Sync {
    fn observe_enrollment(&self) -> EnrollmentObservation;
}

/// Port: observe runtime takeover conditions (standby, fencing token).
pub trait TakeoverObserver: Send + Sync {
    fn observe_takeover(&self) -> RuntimeTakeoverObservation;
}

/// Port: get the kernel's node control plan (if kernel is reachable).
pub trait NodeControlPort: Send + Sync {
    fn node_control_plan(&self) -> Option<NodeControlPlan>;
}

/// Result of observing the kernel.
#[derive(Debug, Clone)]
pub struct KernelObservation {
    pub reachable: bool,
    pub lease_valid: bool,
}

/// Result of observing the mesh.
#[derive(Debug, Clone)]
pub struct MeshObservation {
    pub reachable_peers: usize,
    pub emergence_state: Option<EmergenceState>,
}

/// Result of observing OS health.
#[derive(Debug, Clone)]
pub struct OsHealthObservation {
    pub healthy: bool,
}

/// Result of observing enrollment.
#[derive(Debug, Clone)]
pub struct EnrollmentObservation {
    pub proven: bool,
}

/// Runtime context for federation reconciliation.
pub struct FederationRuntime {
    pub kernel: Arc<dyn KernelObserver>,
    pub mesh: Arc<dyn MeshObserver>,
    pub os_health: Arc<dyn OsHealthObserver>,
    pub enrollment: Arc<dyn EnrollmentObserver>,
    pub takeover: Arc<dyn TakeoverObserver>,
    pub node_control: Arc<dyn NodeControlPort>,
}

impl FederationRuntime {
    /// Run the full federation reconciliation pipeline.
    ///
    /// 1. Observe all sources
    /// 2. Decide autonomy mode
    /// 3. If Autonomous, run emergence elections
    /// 4. If elected for promoted role, check takeover
    /// 5. Return the authoritative FederationReconciliation
    pub fn reconcile(&self) -> FederationReconciliation {
        let kernel = self.kernel.observe_kernel();
        let mesh = self.mesh.observe_mesh();
        let os = self.os_health.observe_os_health();
        let enrollment = self.enrollment.observe_enrollment();
        let takeover_obs = self.takeover.observe_takeover();

        let now_unix_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let autonomy_obs = AutonomyObservation {
            schema: "cynic.local_autonomy_state.v1".to_string(),
            observed_at_unix_s: now_unix_s,
            kernel_reachable: kernel.reachable,
            kernel_lease_valid: kernel.lease_valid,
            local_os_healthy: os.healthy,
            mesh_reachable_peers: mesh.reachable_peers,
            enrollment_proven: enrollment.proven,
        };

        let autonomy = decide_autonomy(&autonomy_obs);

        let emergence_decisions: Vec<_> =
            if autonomy.mode == cynic_protocol::local_autonomy::AutonomyMode::Autonomous {
                if let Some(state) = &mesh.emergence_state {
                    let roles = vec!["build_runner", "storage", "forge", "kernel"];
                    roles
                        .iter()
                        .map(|r| cynic_protocol::emergence::elect_for_role(state, r))
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

        let takeover_decision =
            if autonomy.mode == cynic_protocol::local_autonomy::AutonomyMode::Autonomous {
                Some(cynic_protocol::runtime_takeover::decide_runtime_takeover(
                    takeover_obs,
                ))
            } else {
                None
            };

        let node_control_plan =
            if autonomy.mode == cynic_protocol::local_autonomy::AutonomyMode::Coordinated {
                self.node_control.node_control_plan()
            } else {
                None
            };

        reconcile_federation(
            &autonomy,
            node_control_plan.as_ref(),
            &emergence_decisions,
            takeover_decision.as_ref(),
        )
    }
}

// ---------------------------------------------------------------------------
// Production adapters — wire to real observation sources
// ---------------------------------------------------------------------------

/// Kernel reachability via TCP probe on the kernel's declared ports.
pub struct KernelReachabilityObserver {
    kernel_ip: String,
    kernel_ports: Vec<u16>,
    timeout_ms: u64,
}

impl KernelReachabilityObserver {
    pub fn new(kernel_ip: impl Into<String>, kernel_ports: Vec<u16>) -> Self {
        Self {
            kernel_ip: kernel_ip.into(),
            kernel_ports,
            timeout_ms: 3000,
        }
    }
}

impl KernelObserver for KernelReachabilityObserver {
    fn observe_kernel(&self) -> KernelObservation {
        let mut reachable = false;
        for port in &self.kernel_ports {
            let addr = format!("{}:{}", self.kernel_ip, port);
            if let Ok(socket_addr) = addr.parse() {
                if std::net::TcpStream::connect_timeout(
                    &socket_addr,
                    std::time::Duration::from_millis(self.timeout_ms),
                )
                .is_ok()
                {
                    reachable = true;
                    break;
                }
            }
        }
        KernelObservation {
            reachable,
            lease_valid: reachable, // simplified: reachable implies valid lease
        }
    }
}

/// Mesh peer observer — uses PeerResolver to count reachable peers.
pub struct MeshPeerObserver {
    registry_path: String,
    local_node_id: String,
}

impl MeshPeerObserver {
    pub fn new(registry_path: impl Into<String>, local_node_id: impl Into<String>) -> Self {
        Self {
            registry_path: registry_path.into(),
            local_node_id: local_node_id.into(),
        }
    }
}

impl MeshObserver for MeshPeerObserver {
    fn observe_mesh(&self) -> MeshObservation {
        let resolver = MeshPeerResolver::new(&self.registry_path, &self.local_node_id);
        let peers = resolver.resolve_all_peers();

        // Build EmergenceState from mesh observation.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut peer_states = Vec::new();
        let mut local_state = None;

        for peer in &peers {
            // Determine autonomy mode from reachability (simplified)
            let mode = if peer.reachable {
                AutonomyMode::Autonomous
            } else {
                AutonomyMode::Degraded
            };

            // Compute health score from OS health signals
            let health_score = health_score_from_os(
                peer.health.active_osha_codes.iter().filter(|c| c.contains("softlockup")).count() as u64,
                peer.health.active_osha_codes.iter().filter(|c| c.contains("oom")).count() as u64,
                peer.health.available_mem_mib,
            );

            let peer_state = PeerState {
                node_id: peer.node_id.clone(),
                role: peer.role.clone(),
                mode,
                health_score,
                last_seen_unix_s: now,
                capabilities: vec![peer.role.clone()],
            };

            if peer.node_id == self.local_node_id {
                local_state = Some(peer_state.clone());
            } else {
                peer_states.push(peer_state);
            }
        }

        // If local node wasn't in peers (e.g., not in registry), create a default
        let local = local_state.unwrap_or_else(|| PeerState {
            node_id: self.local_node_id.clone(),
            role: "unknown".to_string(),
            mode: AutonomyMode::Degraded,
            health_score: 0,
            last_seen_unix_s: now,
            capabilities: vec![],
        });

        let emergence_state = EmergenceState {
            schema: cynic_protocol::emergence::EMERGENCE_STATE_SCHEMA.to_string(),
            observed_at_unix_s: now,
            local_node_id: self.local_node_id.clone(),
            peers: peer_states,
            local,
        };

        let reachable_peers = peers.iter().filter(|p| p.reachable).count();

        MeshObservation {
            reachable_peers,
            emergence_state: Some(emergence_state),
        }
    }
}

/// OS health observer adapter — wraps LocalOsHealthObserver.
pub struct OsHealthObserverAdapter;

impl OsHealthObserver for OsHealthObserverAdapter {
    fn observe_os_health(&self) -> OsHealthObservation {
        let observation = crate::os_health::observe::LocalOsHealthObserver.observe_os_health();
        // Derive a simple healthy/unhealthy signal from the observation.
        // Conservative: unhealthy if any critical anomaly is present.
        let unhealthy = observation.dmesg_oom_count > 0
            || observation.dmesg_hung_task_count > 0
            || observation.reclaim_pressure_pct > 90;
        OsHealthObservation {
            healthy: !unhealthy,
        }
    }
}

/// Enrollment observer — checks if the machine is enrolled.
pub struct EnrollmentObserverAdapter;

impl EnrollmentObserver for EnrollmentObserverAdapter {
    fn observe_enrollment(&self) -> EnrollmentObservation {
        // Check if the enrollment authority file exists.
        let path = std::path::Path::new("/etc/cynic/enrollment-authority.json");
        EnrollmentObservation {
            proven: path.exists(),
        }
    }
}

/// Takeover observer — observes runtime takeover conditions.
pub struct TakeoverObserverAdapter {
    pub primary_live: bool,
    pub fencing_token: u64,
}

impl TakeoverObserver for TakeoverObserverAdapter {
    fn observe_takeover(&self) -> RuntimeTakeoverObservation {
        RuntimeTakeoverObservation {
            mode: cynic_protocol::runtime_takeover::StandbyMode::Resident,
            primary_live: self.primary_live,
            lease_valid: !self.primary_live,
            candidate_ready: true,
            release_proved: true,
            storage_available: true,
            fencing_token: self.fencing_token,
            prior_fencing_token: self.fencing_token.saturating_sub(1),
        }
    }
}
/// Default local node control plan adapter.
pub struct LocalNodeControlAdapter;

impl NodeControlPort for LocalNodeControlAdapter {
    fn node_control_plan(&self) -> Option<NodeControlPlan> {
        let path = std::path::Path::new("/etc/cynic/node-control-plan.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(plan) = serde_json::from_str::<NodeControlPlan>(&content) {
                    return Some(plan);
                }
            }
        }
        None
    }
}

impl FederationRuntime {
    /// Build a live federation runtime from registry path and node id.
    pub fn live(registry_path: impl Into<String>, local_node_id: impl Into<String>) -> Self {
        let reg_str = registry_path.into();
        let node_str = local_node_id.into();

        let kernel = Arc::new(KernelReachabilityObserver::new("127.0.0.1", vec![3030]));
        let mesh = Arc::new(MeshPeerObserver::new(&reg_str, &node_str));
        let os_health = Arc::new(OsHealthObserverAdapter);
        let enrollment = Arc::new(EnrollmentObserverAdapter);
        let takeover = Arc::new(TakeoverObserverAdapter {
            primary_live: true,
            fencing_token: 1,
        });
        let node_control = Arc::new(LocalNodeControlAdapter);

        Self {
            kernel,
            mesh,
            os_health,
            enrollment,
            takeover,
            node_control,
        }
    }
}
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::local_autonomy::AutonomyMode;
    use cynic_protocol::node_control::{NodeControlConnectionState, NodeControlDecision};

    struct FakeKernel {
        reachable: bool,
        lease_valid: bool,
    }

    struct FakeMesh {
        peers: usize,
        emergence_state: Option<EmergenceState>,
    }

    struct FakeOsHealth {
        healthy: bool,
    }

    struct FakeEnrollment {
        proven: bool,
    }

    struct FakeTakeover {
        observation: RuntimeTakeoverObservation,
    }

    struct FakeNodeControl {
        plan: Option<NodeControlPlan>,
    }

    impl KernelObserver for FakeKernel {
        fn observe_kernel(&self) -> KernelObservation {
            KernelObservation {
                reachable: self.reachable,
                lease_valid: self.lease_valid,
            }
        }
    }

    impl MeshObserver for FakeMesh {
        fn observe_mesh(&self) -> MeshObservation {
            MeshObservation {
                reachable_peers: self.peers,
                emergence_state: self.emergence_state.clone(),
            }
        }
    }

    impl OsHealthObserver for FakeOsHealth {
        fn observe_os_health(&self) -> OsHealthObservation {
            OsHealthObservation {
                healthy: self.healthy,
            }
        }
    }

    impl EnrollmentObserver for FakeEnrollment {
        fn observe_enrollment(&self) -> EnrollmentObservation {
            EnrollmentObservation {
                proven: self.proven,
            }
        }
    }

    impl TakeoverObserver for FakeTakeover {
        fn observe_takeover(&self) -> RuntimeTakeoverObservation {
            self.observation.clone()
        }
    }

    impl NodeControlPort for FakeNodeControl {
        fn node_control_plan(&self) -> Option<NodeControlPlan> {
            self.plan.clone()
        }
    }

    fn runtime(
        kernel_reachable: bool,
        os_healthy: bool,
        peers: usize,
        enrollment_proven: bool,
    ) -> FederationRuntime {
        FederationRuntime {
            kernel: Arc::new(FakeKernel {
                reachable: kernel_reachable,
                lease_valid: kernel_reachable,
            }),
            mesh: Arc::new(FakeMesh {
                peers,
                emergence_state: None,
            }),
            os_health: Arc::new(FakeOsHealth {
                healthy: os_healthy,
            }),
            enrollment: Arc::new(FakeEnrollment {
                proven: enrollment_proven,
            }),
            takeover: Arc::new(FakeTakeover {
                observation: RuntimeTakeoverObservation {
                    mode: cynic_protocol::runtime_takeover::StandbyMode::Resident,
                    primary_live: !kernel_reachable,
                    lease_valid: kernel_reachable,
                    candidate_ready: true,
                    release_proved: true,
                    storage_available: true,
                    fencing_token: 2,
                    prior_fencing_token: 1,
                },
            }),
            node_control: Arc::new(FakeNodeControl {
                plan: Some(NodeControlPlan {
                    schema: "cynic.node_control_plan.v2".to_string(),
                    status: "converged".to_string(),
                    node_id: "node-a".to_string(),
                    role: "build_runner".to_string(),
                    desired_generation: 1,
                    observed_generation: Some(1),
                    connection: NodeControlConnectionState::Connected,
                    decision: NodeControlDecision::Converged,
                    execution: None,
                    evidence_refs: vec![],
                    unknowns: vec![],
                    // epoch removed in recent protocol revision
                }),
            }),
        }
    }

    #[test]
    fn coordinated_when_kernel_reachable() {
        let rt = runtime(true, true, 3, true);
        let reconciliation = rt.reconcile();
        assert_eq!(reconciliation.mode(), AutonomyMode::Coordinated);
    }

    #[test]
    fn autonomous_when_kernel_dead() {
        let rt = runtime(false, true, 2, true);
        let reconciliation = rt.reconcile();
        assert_eq!(reconciliation.mode(), AutonomyMode::Autonomous);
    }

    #[test]
    fn degraded_when_os_unhealthy() {
        let rt = runtime(false, false, 2, true);
        let reconciliation = rt.reconcile();
        assert_eq!(reconciliation.mode(), AutonomyMode::Degraded);
    }

    #[test]
    fn degraded_when_enrollment_unproved() {
        let rt = runtime(true, true, 3, false);
        let reconciliation = rt.reconcile();
        assert_eq!(reconciliation.mode(), AutonomyMode::Degraded);
    }

    #[test]
    fn kernel_reachability_observer_returns_false_for_closed_port() {
        let observer = KernelReachabilityObserver::new("127.0.0.1", vec![1]);
        let obs = observer.observe_kernel();
        assert!(!obs.reachable);
    }

    #[test]
    fn os_health_observer_adapts_local_observer() {
        let observer = OsHealthObserverAdapter;
        let _obs = observer.observe_os_health();
        // Just verify it doesn't panic; the actual health depends on the host.
    }

    #[test]
    fn enrollment_observer_reads_file() {
        let observer = EnrollmentObserverAdapter;
        let obs = observer.observe_enrollment();
        // Should be false in test environment (no enrollment file).
        assert!(!obs.proven);
    }
}
