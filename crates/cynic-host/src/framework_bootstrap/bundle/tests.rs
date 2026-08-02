#[test]
fn framework_bootstrap_bundle_writes_security_kit() {
    let out = std::env::temp_dir().join(format!(
        "cynic-framework-bootstrap-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));

    let report = super::write_framework_bootstrap_bundle(out.to_str().expect("path"))
        .expect("framework bootstrap bundle");

    assert_eq!(report.status, "written");
    assert!(out.join("host-exec-bundle/cynic-host-exec").is_file());
    assert!(out.join("install.sh").is_file());
    let install_script = std::fs::read_to_string(out.join("install.sh")).expect("read install.sh");
    assert!(install_script.contains("principal-prepare --apply"));
    assert!(install_script.contains("/usr/local/bin/cynic"));
    assert!(!install_script.contains("/home/$RUNTIME_USER"));
    assert!(!install_script.contains("CYNIC_PROJECT_ROOT"));
    assert!(!install_script.contains("organs/supervisor"));
    assert!(!install_script.contains("runtime_principals.py"));
    assert!(!install_script.contains("groupadd"));
    assert!(!install_script.contains("useradd"));
    assert!(out.join("verify.sh").is_file());
    assert!(out.join("security-maintenance.sh").is_file());
    assert!(out.join("security-policy.json").is_file());
    assert!(out.join("runtime-basket.json").is_file());
    assert!(out.join("seed-manifest.json").is_file());
    assert!(out.join("seed-artifacts.sha256").is_file());
    let manifest = std::fs::read_to_string(out.join("host-exec-bundle/manifest.json"))
        .expect("read bridge manifest");
    let security_policy =
        std::fs::read_to_string(out.join("security-policy.json")).expect("read policy");
    let runtime_basket =
        std::fs::read_to_string(out.join("runtime-basket.json")).expect("read runtime basket");
    assert!(!manifest.contains(out.to_string_lossy().as_ref()));
    assert!(!security_policy.contains(out.to_string_lossy().as_ref()));
    assert!(!manifest.contains("/home/"));
    assert!(!security_policy.contains("/home/"));
    let basket: serde_json::Value =
        serde_json::from_str(&runtime_basket).expect("parse runtime basket");
    assert_eq!(basket["schema"], "cynic.runtime_basket.v1");
    assert!(
        basket["artifacts"]
            .as_array()
            .expect("basket artifacts")
            .iter()
            .any(|artifact| artifact["binary"] == "cynic-node")
    );
    let seed: serde_json::Value = serde_json::from_slice(
        &std::fs::read(out.join("seed-manifest.json")).expect("read seed manifest"),
    )
    .expect("parse seed manifest");
    assert_eq!(seed["schema"], "cynic.framework_seed.v2");
    assert_eq!(seed["status"], "seed_only");
    assert_eq!(
        seed["enrollment"]["token_material"],
        "injected_at_apply_never_embedded"
    );
    assert!(
        seed["artifacts"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(!seed.to_string().contains(out.to_string_lossy().as_ref()));
    let checksums =
        std::fs::read_to_string(out.join("seed-artifacts.sha256")).expect("read seed checksums");
    assert!(checksums.contains("  install.sh"));
    assert!(checksums.contains("  seed-manifest.json"));
    assert!(!checksums.contains(out.to_string_lossy().as_ref()));
    std::fs::remove_dir_all(out).expect("cleanup");
}

#[test]
fn build_runner_bundle_blocks_without_role_payload_and_registry() {
    let out = std::env::temp_dir().join(format!(
        "cynic-build-runner-bootstrap-blocked-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));

    let report = super::write_framework_bootstrap_bundle_for_role(
        out.to_str().expect("path"),
        "build_runner",
        None,
        None,
    )
    .expect("blocked report");

    assert_eq!(report.status, "blocked");
    assert_eq!(report.required_role, "build_runner");
    assert!(report.missing_artifacts.contains(&"cynic-node".to_string()));
    assert!(
        report
            .missing_artifacts
            .contains(&"runtime-registry".to_string())
    );
    assert!(!out.exists());
}

#[test]
fn build_runner_bundle_projects_node_and_registry() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cynic-build-runner-bootstrap-{nonce}"));
    let artifacts = root.join("artifacts");
    let out = root.join("bundle");
    std::fs::create_dir_all(&artifacts).expect("artifact dir");
    std::fs::write(artifacts.join("cynic"), b"cli payload").expect("cli artifact");
    std::fs::write(artifacts.join("cynic-node"), b"node payload").expect("node artifact");
    let registry = root.join("registry.json");
    std::fs::write(&registry, br#"{"schema":"test"}"#).expect("registry");

    let report = super::write_framework_bootstrap_bundle_for_role(
        out.to_str().expect("path"),
        "build_runner",
        Some(artifacts.to_str().expect("artifact path")),
        Some(registry.to_str().expect("registry path")),
    )
    .expect("framework bootstrap bundle");

    assert_eq!(report.status, "written");
    assert_eq!(report.required_role, "build_runner");
    assert!(out.join("bin/cynic-node").is_file());
    assert!(out.join("etc/registry.json").is_file());
    let seed = std::fs::read_to_string(out.join("seed-manifest.json")).expect("seed");
    assert!(seed.contains(r#""target_role": "build_runner""#));
    let checksums = std::fs::read_to_string(out.join("seed-artifacts.sha256")).expect("checksums");
    assert!(checksums.contains("  bin/cynic-node"));
    assert!(checksums.contains("  etc/registry.json"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn registry_storage_placement_projects_nonsecret_storage_config() {
    let registry = serde_json::json!({"nodes": [{
        "role": "storage_primary",
        "transports": {"ssh": {"target": "storage.synapse.cynic"}},
        "services": [{"name": "surrealdb", "scheme": "http", "port": 8000}]
    }]});
    let config = super::render_storage_config(&registry, "kernel_primary").unwrap();
    assert!(config.contains("url = \"http://storage.synapse.cynic:8000\""));
    assert!(config.contains("username = \"cynic-runtime\""));
    assert!(!config.to_ascii_lowercase().contains("password ="));
}
