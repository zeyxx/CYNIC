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
        declared: DeclaredCapabilities::default(),
        measured: MeasuredCapabilities::default(),
        health: BackendHealth::Healthy,
    };
    assert!(matches!(backend.health, BackendHealth::Healthy));
}

#[test]
fn measured_capabilities_default_is_pessimistic() {
    let m = MeasuredCapabilities::default();
    assert_eq!(m.json_valid_rate, 0.0); // K14: unknown = worst
}
