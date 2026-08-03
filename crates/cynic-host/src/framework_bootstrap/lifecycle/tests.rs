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
    assert_eq!(audit.dimensions.len(), 10);
    for expected in [
        "artifact_packaging",
        "release_signing",
        "artifact_store",
        "artifact_publication",
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
