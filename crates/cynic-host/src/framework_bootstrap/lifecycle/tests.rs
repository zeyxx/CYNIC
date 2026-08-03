use super::audit_framework_lifecycle;

#[test]
fn current_registry_exposes_missing_runtime_and_enrollment_dimensions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json");
    let audit = audit_framework_lifecycle(root.to_str(), "build_runner").expect("audit");

    assert_eq!(audit.status, "blocked");
    assert!(
        audit
            .blockers
            .contains(&"kernel_runtime_endpoint_scheme_missing".to_string())
    );
    assert!(
        audit
            .blockers
            .contains(&"kernel_runtime_endpoint_host_missing".to_string())
    );
    assert!(
        audit
            .blockers
            .contains(&"node_enrollment_desired_state_missing".to_string())
    );
    assert!(
        audit
            .dimensions
            .iter()
            .any(|dimension| dimension.name == "terminal_read_back"
                && dimension.convergence_status == "blocked_desired_state")
    );
}
