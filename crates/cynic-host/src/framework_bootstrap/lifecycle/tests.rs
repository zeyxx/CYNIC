use super::audit_framework_lifecycle;

#[test]
fn current_registry_exposes_missing_builder_reconstruction_identity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json");
    let audit = audit_framework_lifecycle(root.to_str(), "build_runner").expect("audit");

    assert_eq!(audit.status, "blocked");
    assert_eq!(
        audit.blockers,
        vec!["node_substrate_provisioning_spec_missing"]
    );
    assert_eq!(audit.dimensions.len(), 15);
    let reconstruction = audit
        .dimensions
        .iter()
        .find(|dimension| dimension.name == "substrate_reconstruction")
        .expect("reconstruction dimension");
    assert_eq!(reconstruction.convergence_status, "blocked_desired_state");
    assert_eq!(
        audit.next_actions,
        vec!["declare_content_addressed_substrate_reconstruction"]
    );
    for expected in [
        "artifact_packaging",
        "release_signing",
        "artifact_store",
        "artifact_publication",
        "source_snapshot_recovery",
        "workload_delivery_recovery",
        "bootstrap_containment_retirement",
        "provider_control_identity",
    ] {
        assert!(
            audit
                .dimensions
                .iter()
                .any(|dimension| dimension.name == expected)
        );
    }
}

#[test]
fn artifact_store_topology_is_a_distinct_blocking_dimension() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json");
    let mut registry: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root).expect("registry")).expect("json");
    registry["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["role"] == "storage_primary")
        .expect("storage node")["services"]
        .as_array_mut()
        .expect("services")
        .retain(|service| service["name"] != "runtime-artifact-store");
    let temp =
        std::env::temp_dir().join(format!("cynic-lifecycle-audit-{}.json", std::process::id()));
    std::fs::write(&temp, serde_json::to_vec(&registry).expect("encode")).expect("write");

    let audit = audit_framework_lifecycle(temp.to_str(), "build_runner").expect("audit");
    let _ = std::fs::remove_file(temp);

    let store = audit
        .dimensions
        .iter()
        .find(|dimension| dimension.name == "artifact_store")
        .expect("store dimension");
    assert_eq!(store.convergence_status, "blocked_desired_state");
    assert_eq!(store.blockers, vec!["artifact_store_service_cardinality:0"]);
}

#[test]
fn provider_control_identity_requires_typed_non_secret_desired_state() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json");
    let mut registry: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root).expect("registry")).expect("json");
    registry
        .as_object_mut()
        .expect("registry object")
        .remove("provider_control");
    let temp = std::env::temp_dir().join(format!(
        "cynic-provider-control-audit-{}.json",
        std::process::id()
    ));
    std::fs::write(&temp, serde_json::to_vec(&registry).expect("encode")).expect("write");

    let audit = audit_framework_lifecycle(temp.to_str(), "build_runner").expect("audit");
    let _ = std::fs::remove_file(temp);

    let identity = audit
        .dimensions
        .iter()
        .find(|dimension| dimension.name == "provider_control_identity")
        .expect("provider identity dimension");
    assert_eq!(identity.convergence_status, "blocked_desired_state");
    assert_eq!(
        identity.blockers,
        vec!["provider_control_desired_state_missing"]
    );
}
