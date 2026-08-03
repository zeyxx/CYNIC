use super::audit_framework_lifecycle;

#[test]
fn current_registry_declares_every_dimension_but_requires_runtime_observation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json");
    let audit = audit_framework_lifecycle(root.to_str(), "build_runner").expect("audit");

    assert_eq!(audit.status, "observation_required");
    assert!(audit.blockers.is_empty());
    assert!(
        audit
            .dimensions
            .iter()
            .all(|dimension| dimension.convergence_status == "observation_required")
    );
}
