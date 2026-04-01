use cynic_kernel::organ::registry::*;

#[test]
fn declared_capabilities_default_is_all_false() {
    let cap = DeclaredCapabilities::default();
    assert!(!cap.json);
    assert!(!cap.thinking);
    assert!(!cap.scoring);
}

#[test]
fn backend_starts_healthy() {
    let backend = Backend {
        id: BackendId("test".into()),
        node_id: NodeId("node".into()),
        endpoint: "/v1".into(),
        model: "test-model".into(),
        declared: DeclaredCapabilities::default(),
        measured: MeasuredCapabilities::default(),
        health: BackendHealth::Healthy,
        timeout_secs: 30,
        remediation: None,
    };
    assert!(matches!(backend.health, BackendHealth::Healthy));
}

#[test]
fn cluster_contains_backend() {
    let cluster = Cluster {
        id: ClusterId("scoring".into()),
        required_json_rate: 0.7,
        backends: vec![BackendId("b1".into())],
        strategy: ClusterStrategy::RoundRobin,
    };
    assert_eq!(cluster.backends.len(), 1);
}

#[test]
fn measured_capabilities_default_is_pessimistic() {
    let m = MeasuredCapabilities::default();
    assert_eq!(m.json_valid_rate, 0.0); // K14: unknown = worst
    assert_eq!(m.tokens_per_second, 0.0);
    assert_eq!(m.mean_latency_ms, u32::MAX); // K14: unknown = worst
}
