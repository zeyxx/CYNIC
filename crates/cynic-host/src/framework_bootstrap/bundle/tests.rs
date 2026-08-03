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
    assert!(out.join("enroll.sh").is_file());
    let enroll_script = std::fs::read_to_string(out.join("enroll.sh")).expect("read enroll.sh");
    assert!(enroll_script.contains("\"$ROOT/install.sh\""));
    assert!(enroll_script.contains("\"$ROOT/verify.sh\""));
    assert!(enroll_script.contains("one-time HumanAdmin boundary"));
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
    assert!(checksums.contains("  enroll.sh"));
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
    std::fs::write(
        &registry,
        serde_json::json!({
            "schema": "test",
            "release_publication": {
                "schema": "cynic.release_publication.desired_state.v1",
                "storage_role": "storage_primary",
                "service_name": "runtime-artifact-store",
                "protocol": "generic_https_put_get",
                "authentication_kind": "bearer_file",
                "credential_path": "/var/lib/cynic/identity/artifact-store-token",
                "signing_key_path": "/var/lib/cynic/identity/release-signing-key",
                "signer_identity": "cynic-release",
                "signature_namespace": "cynic-runtime-seed"
            },
            "nodes": [
                {"id": "cynic-builder", "role": "build_runner", "services": []},
                {
                    "id": "storage-a",
                    "role": "storage_primary",
                    "proxmox_id": "lxc/103",
                    "services": [{
                        "name": "runtime-artifact-store",
                        "scheme": "https",
                        "endpoint_host": "storage.synapse.cynic",
                        "port": 8443,
                        "base_path": "/runtime-releases",
                        "protocol": "generic_https_put_get",
                        "authentication_kind": "bearer_file"
                    }]
                }
            ]
        })
        .to_string(),
    )
    .expect("registry");

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
    assert!(out.join("etc/node-control.toml").is_file());
    assert!(out.join("etc/systemd/cynic-node.service").is_file());
    let control = std::fs::read_to_string(out.join("etc/node-control.toml")).expect("control");
    assert!(control.contains("node_id = \"cynic-builder\""));
    assert!(control.contains("enrollment_role = \"build_runner\""));
    assert!(control.contains("kernel_url = \"${CYNIC_KERNEL_URL}\""));
    assert!(control.contains("agent_workload_state_root = \"${CYNIC_STATE_ROOT}\""));
    assert!(
        control.contains(
            "runtime_artifact_inbox_root = \"${CYNIC_STATE_ROOT}/runtime-artifacts/inbox\""
        )
    );
    assert!(control.contains("runtime_artifact_max_bytes = 4294967296"));
    assert!(control.contains("[node_control.build_scheduler]"));
    assert!(control.contains("[node_control.build_scheduler.activation]"));
    assert!(control.contains("status = \"gated\""));
    assert!(control.contains("mode = \"durable_node_owned\""));
    assert!(control.contains("activation_requires_read_back = true"));
    assert!(control.contains("registry_path = \"/etc/cynic/registry.json\""));
    assert!(control.contains("[node_control.build_scheduler.release_publication]"));
    assert!(
        control.contains(
            "artifact_base_url = \"https://storage.synapse.cynic:8443/runtime-releases\""
        )
    );
    assert!(control.contains("authentication_kind = \"bearer_file\""));
    assert!(control.contains("credential_path = \"/var/lib/cynic/identity/artifact-store-token\""));
    assert!(!control.contains("cynic-host-cortex"));
    assert!(!control.contains("[dog]"));
    let service =
        std::fs::read_to_string(out.join("etc/systemd/cynic-node.service")).expect("service");
    assert!(service.contains(
        "ExecStart=/usr/local/bin/cynic-node control --config /etc/cynic/node-control.toml"
    ));
    let seed = std::fs::read_to_string(out.join("seed-manifest.json")).expect("seed");
    assert!(seed.contains(r#""target_role": "build_runner""#));
    let checksums = std::fs::read_to_string(out.join("seed-artifacts.sha256")).expect("checksums");
    assert!(checksums.contains("  bin/cynic-node"));
    assert!(checksums.contains("  etc/registry.json"));
    let install = std::fs::read_to_string(out.join("install.sh")).expect("install script");
    assert!(install.contains("RUNTIME_INSTALL_ROOT"));
    assert!(install.contains("RUNTIME_ARTIFACT_INBOX"));
    assert!(install.contains("stage_runtime_artifact 'cynic-node' '/usr/local/bin/cynic-node'"));
    assert!(
        install.contains("prepare_runtime_entrypoint 'cynic-node' '/usr/local/bin/cynic-node'")
    );
    assert!(
        install.contains("activate_runtime_entrypoint 'cynic-node' '/usr/local/bin/cynic-node'")
    );
    assert!(
        install.contains("read_back_runtime_entrypoint 'cynic-node' '/usr/local/bin/cynic-node'")
    );
    assert!(install.contains("$RUNTIME_INSTALL_ROOT/current/$binary"));
    assert!(
        !install
            .contains("install -o root -g root -m 0755 \"$ROOT/bin/cynic-node\" \"$NODE_TARGET\"")
    );
    let verify = std::fs::read_to_string(out.join("verify.sh")).expect("verify script");
    assert!(
        verify.contains("read_back_runtime_entrypoint 'cynic-node' '/usr/local/bin/cynic-node'")
    );
    assert!(checksums.contains("  etc/node-control.toml"));
    assert!(checksums.contains("  etc/systemd/cynic-node.service"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn build_runner_projection_requires_proxmox_placed_artifact_storage_desired_state() {
    let registry = serde_json::json!({
        "nodes": [
            {"id": "builder-a", "role": "build_runner", "services": []},
            {
                "id": "storage-a",
                "role": "storage_primary",
                "services": [{"name": "runtime-artifact-store"}]
            }
        ]
    });

    let error = super::render_node_control_config(&registry, "build_runner").unwrap_err();

    assert!(error.contains("release publication desired state is absent"));
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
