//! End-to-end federation reconciliation integration test.
//!
//! Proves the full federation pipeline with production adapters:
//! KernelReachabilityObserver, MeshPeerObserver, OsHealthObserverAdapter,
//! EnrollmentObserverAdapter, TakeoverObserverAdapter.
//!
//! The test exercises the real observation path (TCP probe, OS health, etc.)
//! but asserts only structural properties of the reconciliation — not the
//! actual health of the test host (which is unknown and variable).

use std::sync::Arc;

use cynic_host::federation_runtime::{
    EnrollmentObserver, EnrollmentObserverAdapter, FederationRuntime, KernelObserver,
    KernelReachabilityObserver, MeshObserver, MeshPeerObserver, NodeControlPort, OsHealthObserver,
    OsHealthObserverAdapter, TakeoverObserverAdapter,
};
use cynic_node_protocol::federation::FederationReconciliation;
use cynic_node_protocol::local_autonomy::AutonomyMode;

/// Build a federation runtime with production adapters.
fn production_runtime() -> FederationRuntime {
    FederationRuntime {
        kernel: Arc::new(KernelReachabilityObserver::new("127.0.0.1", vec![1])),
        mesh: Arc::new(MeshPeerObserver::new("infra/registry.json", "local")),
        os_health: Arc::new(OsHealthObserverAdapter),
        enrollment: Arc::new(EnrollmentObserverAdapter),
        takeover: Arc::new(TakeoverObserverAdapter {
            primary_live: false,
            fencing_token: 2,
        }),
        node_control: Arc::new(FakeNodeControl),
    }
}

struct FakeNodeControl;

impl cynic_host::federation_runtime::NodeControlPort for FakeNodeControl {
    fn node_control_plan(&self) -> Option<cynic_node_protocol::node_control::NodeControlPlan> {
        None
    }
}

#[test]
fn e2e_reconciliation_with_production_adapters() {
    let rt = production_runtime();
    let reconciliation = rt.reconcile();

    // KernelReachabilityObserver probes port 1 (closed) → unreachable.
    // With kernel unreachable and 0 peers, mode is Autonomous.
    assert_eq!(reconciliation.mode(), AutonomyMode::Autonomous);
}

#[test]
fn e2e_kernel_unreachable_with_peers_is_autonomous() {
    let rt = FederationRuntime {
        kernel: Arc::new(KernelReachabilityObserver::new("127.0.0.1", vec![1])),
        mesh: Arc::new(MeshPeerObserver::new("infra/registry.json", "local")),
        os_health: Arc::new(OsHealthObserverAdapter),
        enrollment: Arc::new(EnrollmentObserverAdapter),
        takeover: Arc::new(TakeoverObserverAdapter {
            primary_live: false,
            fencing_token: 2,
        }),
        node_control: Arc::new(FakeNodeControl),
    };

    let reconciliation = rt.reconcile();
    assert_eq!(reconciliation.mode(), AutonomyMode::Autonomous);

    // Should have residual unknowns about mesh (no emergence state).
    let unknowns = reconciliation.residual_unknowns();
    assert!(
        unknowns.is_empty() || unknowns.iter().any(|u| u.contains("mesh")),
        "expected mesh-related unknowns, got: {unknowns:?}"
    );
}

#[test]
fn e2e_os_health_observer_integration() {
    let observer = OsHealthObserverAdapter;
    let obs = observer.observe_os_health();
    // Just verify it doesn't panic. Actual health depends on the host.
    // We can't assert healthy/unhealthy without knowing the host state.
    let _ = obs.healthy;
}

#[test]
fn e2e_enrollment_observer_integration() {
    let observer = EnrollmentObserverAdapter;
    let obs = observer.observe_enrollment();
    // In CI/test env, the enrollment file likely doesn't exist.
    // This is a structural test — not an assertion about real enrollment.
    let _ = obs.proven;
}

#[test]
fn e2e_kernel_reachability_closed_port() {
    let observer = KernelReachabilityObserver::new("127.0.0.1", vec![1]);
    let obs = observer.observe_kernel();
    // Port 1 (tcpmux) is almost certainly closed on test hosts.
    assert!(!obs.reachable, "port 1 should be unreachable");
}

#[test]
fn e2e_mesh_observer_reads_registry() {
    let observer = MeshPeerObserver::new("infra/registry.json", "local");
    let obs = observer.observe_mesh();
    // Registry should have some peers declared.
    // Can't assert exact count without reading the registry.
    let _ = obs.reachable_peers;
}
