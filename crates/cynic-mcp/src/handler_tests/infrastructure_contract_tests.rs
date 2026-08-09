#[cfg(test)]
mod infrastructure_contract_tests {
    use super::*;

    fn registry_fixture() -> serde_json::Value {
        serde_json::json!({
            "network_policy": {
                "canonical_service_plane": "synapse",
                "planes": {
                    "synapse": {"cidr": "10.20.0.0/24"}
                },
                "reservations": [
                    {
                        "name": "cynic-lxc-surrealdb",
                        "synapse": "10.20.0.3",
                        "dns": [
                            "cynic-lxc-surrealdb.synapse.cynic",
                            "organ-surrealdb.synapse.cynic"
                        ]
                    },
                    {
                        "name": "cynic-vm-v2",
                        "synapse": "10.20.0.5",
                        "dns": ["cynic-vm-v2.synapse.cynic"]
                    }
                ]
            },
            "nodes": [
                {
                    "name": "cynic-lxc-surrealdb",
                    "aliases": ["organ-surrealdb"],
                    "role": "storage_primary",
                    "status": "planned",
                    "network": {"synapse": "10.20.0.3"}
                },
                {
                    "name": "cynic-vm-v2",
                    "role": "kernel_primary",
                    "status": "active",
                    "network": {"synapse": "10.20.0.5"}
                }
            ]
        })
    }

    #[test]
    fn network_policy_validation_accepts_reserved_synapse_ips() {
        let registry = registry_fixture();

        assert!(validate_network_policy(&registry).is_empty());
    }

    #[test]
    fn network_policy_validation_rejects_duplicate_synapse_ips() {
        let mut registry = registry_fixture();
        registry["nodes"][1]["network"]["synapse"] = serde_json::json!("10.20.0.3");

        let violations = validate_network_policy(&registry);

        assert!(violations.iter().any(|violation| {
            violation.get("kind").and_then(|value| value.as_str()) == Some("duplicate_synapse_ip")
        }));
    }

    #[test]
    fn find_registry_node_matches_alias() {
        let registry = registry_fixture();
        let node = find_registry_node(&registry, "organ-surrealdb").expect("alias should match");

        assert_eq!(node_name(node), Some("cynic-lxc-surrealdb"));
    }

    #[test]
    fn render_synapse_dns_records_uses_reservation_aliases() {
        let registry = registry_fixture();
        let records = render_synapse_dns_records(&registry, false);
        let storage = records
            .iter()
            .find(|record| {
                record.get("node").and_then(|value| value.as_str()) == Some("cynic-lxc-surrealdb")
            })
            .expect("storage record should render");
        let names = storage
            .get("names")
            .and_then(|value| value.as_array())
            .expect("names should be an array");

        assert!(names
            .iter()
            .any(|name| name.as_str() == Some("organ-surrealdb.synapse.cynic")));
        assert!(!names
            .iter()
            .any(|name| name.as_str() == Some("organ-surrealdb")));
    }

    #[test]
    fn network_mesh_status_reports_policy_without_dns_resolution_by_default() {
        let registry = registry_fixture();
        let report = network_mesh_status_report(&registry, false, true);

        assert_eq!(report["policy_status"], "ok");
        assert_eq!(report["dns_status"], "not_checked");
        assert_eq!(report["runtime_status"], "not_observed");
        assert!(report["records"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
    }

    #[test]
    fn network_mesh_status_can_omit_records_for_compact_reads() {
        let registry = registry_fixture();
        let report = network_mesh_status_report(&registry, false, false);

        assert!(report["records"].is_null());
        assert_eq!(report["peers"].as_u64(), Some(2));
    }

    #[test]
    fn proxmox_resource_id_resolves_from_hypervisor_inventory() {
        let mut registry = registry_fixture();
        registry["nodes"][0]["status"] = serde_json::json!("active");
        registry["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "cynic-host-proxmox",
                "role": "hypervisor",
                "vms": [
                    {
                        "id": "lxc/103",
                        "name": "cynic-lxc-surrealdb",
                        "aliases": ["organ-surrealdb"]
                    }
                ]
            }));
        let node = find_registry_node(&registry, "cynic-lxc-surrealdb").expect("node should exist");

        assert_eq!(
            proxmox_resource_id_for_node(&registry, node),
            Some("lxc/103".to_string())
        );
    }

    #[test]
    fn proxmox_resource_kind_id_rejects_unknown_resource_forms() {
        assert_eq!(proxmox_resource_kind_id("qemu/140"), Some(("qemu", "140")));
        assert_eq!(proxmox_resource_kind_id("lxc/103"), Some(("lxc", "103")));
        assert_eq!(proxmox_resource_kind_id("node/pve"), None);
        assert_eq!(proxmox_resource_kind_id("qemu/"), None);
    }

    #[test]
    fn host_native_roles_require_host_exec_transport_instead_of_proxmox_id() {
        let cortex_host = serde_json::json!({
            "name": "cynic-host-cortex",
            "role": "cortex_host"
        });
        let inference_host = serde_json::json!({
            "name": "cynic-host-gpu",
            "role": "inference_node"
        });
        let vm = serde_json::json!({
            "name": "cynic-vm-v2",
            "role": "kernel_primary"
        });

        assert!(node_requires_host_exec_transport(&cortex_host));
        assert!(node_requires_host_exec_transport(&inference_host));
        assert!(!node_requires_host_exec_transport(&vm));
    }

    #[test]
    fn host_exec_transport_command_argv_uses_declared_default_command() {
        let node = serde_json::json!({
            "name": "cynic-host-cortex",
            "role": "cortex_host",
            "host_exec_transport": {
                "schema": "cynic.host_exec_transport.v1",
                "kind": "service_agent_surface",
                "status": "declared",
                "principal": "cynic",
                "required_bridge": "run canonical host helper",
                "reason": "host-native execution surface",
                "transport_command_default": "/tmp/cynic-host-helper"
            }
        });

        let argv = host_exec_transport_command_argv(&node)
            .expect("default command should be accepted")
            .expect("argv should be present");

        assert_eq!(argv, vec!["/tmp/cynic-host-helper"]);
    }

    #[tokio::test]
    async fn run_host_exec_transport_executes_declared_helper() {
        let capture = std::env::temp_dir().join(format!(
            "cynic-host-exec-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let request = serde_json::json!({
            "schema": "cynic.host_exec_transport.request.v1",
            "status": "completed",
            "node": "cynic-host-cortex",
            "purpose": "unit-test",
            "command": ["systemctl", "status", "cynic-node"]
        });
        let result = CynicMcpProxy::run_host_exec_transport(
            &cynic_host::ports::argv_execution::LocalArgvExecutionAdapter,
            &[
                "/usr/bin/tee".to_string(),
                capture.to_string_lossy().to_string(),
            ],
            &request,
        )
        .await
        .expect("host exec helper should run");
        let _ = std::fs::remove_file(&capture);

        assert_eq!(result["status"], "completed");
        assert_eq!(result["provider"], "host_exec_transport");
        assert_eq!(result["transport"]["status"], "completed");
        assert_eq!(result["transport"]["node"], "cynic-host-cortex");
        assert_eq!(result["transport"]["purpose"], "unit-test");
        assert_eq!(
            result["transport"]["command"],
            serde_json::json!(["systemctl", "status", "cynic-node"])
        );
    }

    #[test]
    fn observed_resource_for_node_matches_proxmox_id_before_name() {
        let mut registry = registry_fixture();
        registry["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "cynic-host-proxmox",
                "role": "hypervisor",
                "vms": [
                    {
                        "id": "qemu/140",
                        "name": "cynic-vm-v2"
                    }
                ]
            }));
        let node = find_registry_node(&registry, "cynic-vm-v2").expect("node should exist");
        let provider = serde_json::json!({
            "status": "ok",
            "resources": [
                {"id": "qemu/140", "name": "renamed-at-provider", "node": "pve", "status": "running"}
            ]
        });

        let observed = observed_resource_for_node(&registry, node, &provider).expect("match");

        assert_eq!(observed["node"], "pve");
        assert_eq!(observed["status"], "running");
    }

    #[test]
    fn service_endpoint_uses_synapse_dns_contract() {
        let mut registry = registry_fixture();
        registry["nodes"][0]["services"] = serde_json::json!([
            {"name": "surrealdb", "port": 8000}
        ]);
        let node = find_registry_node(&registry, "organ-surrealdb").expect("node should exist");
        let service = find_declared_service(node, "surrealdb").expect("service should exist");

        assert_eq!(
            service_endpoint_from_registry(node, service),
            Some("http://organ-surrealdb.synapse.cynic:8000/health".to_string())
        );
    }

    #[test]
    fn storage_fleet_report_matches_organ_keep_activation_contract() {
        let mut registry = registry_fixture();
        let storage_node = registry["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| {
                node.get("name").and_then(|value| value.as_str()) == Some("cynic-lxc-surrealdb")
            })
            .expect("storage node");
        storage_node["status"] = serde_json::json!("active");
        registry["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "cynic-host-proxmox",
                "role": "hypervisor",
                "vms": [{
                    "id": "lxc/103",
                    "name": "cynic-lxc-surrealdb"
                }]
            }));
        let provider = serde_json::json!({
            "resources": [{
                "id": "lxc/103",
                "name": "cynic-lxc-surrealdb",
                "status": "running"
            }]
        });

        let report = storage_fleet_report(&registry, &provider).expect("fleet report");

        assert_eq!(report["effective"][0]["name"], "cynic-lxc-surrealdb");
        assert_eq!(report["effective"][0]["observed_status"], "running");
        assert_eq!(report["effective"][0]["effective_status"], "active");
    }

    #[test]
    fn lxc_argv_rejects_shell_syntax() {
        assert_eq!(
            validate_lxc_argv(&["hostname".into()]).unwrap(),
            vec!["hostname"]
        );
        assert!(validate_lxc_argv(&["sh".into(), "-c".into(), "hostname".into()]).is_err());
        assert!(validate_lxc_argv(&["hostname".into(), "&&".into()]).is_err());
        assert!(validate_lxc_argv(&["cat".into(), "/etc/os-release".into()]).is_ok());
    }

    #[test]
    fn lxc_exec_bridge_plan_is_rust_native_and_bounded() {
        let params = LxcExecBridgePlanParams {
            helper_source: None,
        };
        let plan = run_lxc_exec_bridge_plan(&params).unwrap();
        assert_eq!(plan["schema"], "cynic.lxc_exec_bridge.install_plan.v1");
        assert_eq!(plan["status"], "planned");
        assert!(plan["helper_source"]
            .as_str()
            .is_some_and(|path| path.ends_with("cynic-lxc-exec-bridge")));
    }

    #[test]
    fn lxc_exec_bridge_bundle_output_is_bounded_to_tmp() {
        let params = LxcExecBridgeBundleParams {
            output_dir: None,
            helper_source: None,
        };
        assert_eq!(
            lxc_exec_bridge_bundle_output_dir(&params).unwrap(),
            "/tmp/cynic-lxc-bridge"
        );

        let rejected = LxcExecBridgeBundleParams {
            output_dir: Some("/var/lib/unsafe".to_string()),
            helper_source: None,
        };
        assert!(lxc_exec_bridge_bundle_output_dir(&rejected).is_err());
    }

    #[test]
    fn network_mesh_dns_activation_bundle_args_are_bounded() {
        let params = NetworkMeshDnsActivationBundleParams {
            output_dir: None,
            registry_ref: None,
        };

        assert_eq!(
            network_mesh_dns_activation_bundle_args(&params).unwrap(),
            vec![
                "dns",
                "bundle",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                "/tmp/cynic-synapse-dns"
            ]
        );

        let rejected_output = NetworkMeshDnsActivationBundleParams {
            output_dir: Some("/etc/coredns".to_string()),
            registry_ref: None,
        };
        assert!(network_mesh_dns_activation_bundle_args(&rejected_output).is_err());

        let rejected_registry = NetworkMeshDnsActivationBundleParams {
            output_dir: None,
            registry_ref: Some("other.json".to_string()),
        };
        assert!(network_mesh_dns_activation_bundle_args(&rejected_registry).is_err());
    }

    #[test]
    fn network_mesh_dns_activation_status_resolves_dns_by_default() {
        let params = NetworkMeshDnsActivationStatusParams {
            hosts_path: Some("/tmp/hosts".to_string()),
            corefile_path: Some("/tmp/Corefile".to_string()),
            service_path: Some("/tmp/coredns.service".to_string()),
            resolved_dropin_path: Some("/tmp/resolved.conf".to_string()),
            resolve_dns: None,
        };

        assert_eq!(
            network_mesh_dns_activation_status_args(&params),
            vec![
                "dns",
                "activation-status",
                "--hosts-path",
                "/tmp/hosts",
                "--corefile-path",
                "/tmp/Corefile",
                "--service-path",
                "/tmp/coredns.service",
                "--resolved-dropin-path",
                "/tmp/resolved.conf",
                "--resolve-dns"
            ]
        );
    }

    #[test]
    fn storage_primary_plan_args_use_default_registry() {
        let params = StoragePrimaryPlanParams { registry: None };

        assert_eq!(
            storage_primary_plan_args(&params).unwrap(),
            vec![
                "storage-primary",
                "plan",
                "--registry",
                "infra/registry.json"
            ]
        );
    }

    #[test]
    fn storage_primary_bundle_args_are_bounded() {
        let params = StoragePrimaryBundleParams {
            output_dir: None,
            registry: None,
            schema: None,
        };

        assert_eq!(
            storage_primary_bundle_args(&params).unwrap(),
            vec![
                "storage-primary",
                "bundle",
                "--registry",
                "infra/registry.json",
                "--schema",
                "crates/cynic-kernel/src/storage/schema_bootstrap.sql",
                "--output-dir",
                "/tmp/cynic-storage-primary"
            ]
        );

        let rejected_output = StoragePrimaryBundleParams {
            output_dir: Some("/var/lib/cynic-storage-primary".to_string()),
            registry: None,
            schema: None,
        };
        assert!(storage_primary_bundle_args(&rejected_output).is_err());

        let rejected_schema = StoragePrimaryBundleParams {
            output_dir: None,
            registry: None,
            schema: Some("/etc/passwd".to_string()),
        };
        assert!(storage_primary_bundle_args(&rejected_schema).is_err());
    }

    #[test]
    fn storage_primary_status_args_include_optional_evidence_paths() {
        let params = StoragePrimaryStatusParams {
            bundle_dir: Some("/tmp/storage-bundle".to_string()),
            auto_bundle_dir: Some(true),
            bundle_default_dir: Some("/tmp/ignored-storage-bundle".to_string()),
            bridge_status_json: Some("/tmp/bridge.json".to_string()),
            auto_bridge_status: Some(true),
            fleet_report_json: Some("/tmp/fleet.json".to_string()),
            mesh_status_json: Some("/tmp/mesh.json".to_string()),
            auto_mesh_status: Some(true),
            resolve_mesh_dns: Some(true),
            auto_fleet_status: Some(true),
            health_json: Some("/tmp/health.json".to_string()),
            auto_service_health: Some(true),
            service_health_timeout_seconds: Some(1),
        };

        assert_eq!(
            storage_primary_status_args(
                &params,
                storage_primary_status_uses_auto_mesh(&params)
                    || storage_primary_status_uses_auto_bridge(&params)
                    || storage_primary_status_uses_auto_fleet(&params)
                    || storage_primary_status_uses_auto_health(&params)
            )
            .unwrap(),
            vec![
                "storage-primary",
                "status",
                "--bundle-dir",
                "/tmp/storage-bundle",
                "--bridge-status-json",
                "/tmp/bridge.json",
                "--fleet-report-json",
                "/tmp/fleet.json",
                "--mesh-status-json",
                "/tmp/mesh.json",
                "--health-json",
                "/tmp/health.json"
            ]
        );
        assert!(!storage_primary_status_uses_auto_mesh(&params));
        assert!(!storage_primary_status_uses_auto_bridge(&params));
        assert!(!storage_primary_status_uses_auto_fleet(&params));
        assert!(!storage_primary_status_uses_auto_health(&params));
    }

    #[test]
    fn storage_primary_status_args_can_use_evidence_stdin_without_temp_file() {
        let params = StoragePrimaryStatusParams {
            bundle_dir: None,
            auto_bundle_dir: Some(true),
            bundle_default_dir: None,
            bridge_status_json: None,
            auto_bridge_status: Some(true),
            fleet_report_json: None,
            mesh_status_json: None,
            auto_mesh_status: Some(true),
            resolve_mesh_dns: None,
            auto_fleet_status: Some(true),
            health_json: None,
            auto_service_health: Some(true),
            service_health_timeout_seconds: None,
        };

        assert!(storage_primary_status_uses_auto_mesh(&params));
        assert!(storage_primary_status_uses_auto_bridge(&params));
        assert!(storage_primary_status_uses_auto_fleet(&params));
        assert!(storage_primary_status_uses_auto_health(&params));
        assert_eq!(
            storage_primary_status_args(&params, true).unwrap(),
            vec![
                "storage-primary",
                "status",
                "--bundle-dir",
                "/tmp/cynic-storage-primary",
                "--evidence-stdin"
            ]
        );
    }

    #[test]
    fn storage_primary_status_rejects_non_tmp_default_bundle_dir() {
        let params = StoragePrimaryStatusParams {
            bundle_dir: None,
            auto_bundle_dir: Some(true),
            bundle_default_dir: Some("/var/lib/cynic-storage-primary".to_string()),
            bridge_status_json: None,
            auto_bridge_status: None,
            fleet_report_json: None,
            mesh_status_json: None,
            auto_mesh_status: None,
            resolve_mesh_dns: None,
            auto_fleet_status: None,
            health_json: None,
            auto_service_health: None,
            service_health_timeout_seconds: None,
        };

        assert!(storage_primary_status_args(&params, false).is_err());
    }

    #[test]
    fn storage_primary_readiness_defaults_compose_all_evidence() {
        let params = storage_primary_readiness_status_params(&StoragePrimaryReadinessParams {
            resolve_mesh_dns: None,
            service_health_timeout_seconds: None,
            bundle_default_dir: None,
        });

        assert!(storage_primary_status_uses_auto_bundle(&params));
        assert!(storage_primary_status_uses_auto_bridge(&params));
        assert!(storage_primary_status_uses_auto_mesh(&params));
        assert!(storage_primary_status_uses_auto_fleet(&params));
        assert!(storage_primary_status_uses_auto_health(&params));
        assert_eq!(params.resolve_mesh_dns, Some(true));
        assert_eq!(params.service_health_timeout_seconds, Some(3));
    }

    #[test]
    fn registry_node_lookup_prefers_direct_node_over_nested_vm() {
        let registry = serde_json::json!({
            "nodes": [
                {
                    "name": "cynic-host-proxmox",
                    "vms": [
                        {"id": "lxc/103", "name": "cynic-lxc-surrealdb"}
                    ]
                },
                {
                    "name": "cynic-lxc-surrealdb",
                    "aliases": ["organ-surrealdb"],
                    "role": "storage_primary",
                    "services": [{"name": "surrealdb", "port": 8000}]
                }
            ]
        });

        let node = find_registry_node(&registry, "cynic-lxc-surrealdb").unwrap();

        assert_eq!(node["role"], "storage_primary");
        assert!(find_declared_service(node, "surrealdb").is_some());
    }

    #[test]
    fn storage_primary_readiness_result_maps_missing_to_actions() {
        let status = serde_json::json!({
            "schema": "cynic.storage_primary_activation.status.v1",
            "status": "blocked",
            "missing": ["bridge_active", "mesh_reaches_storage", "service_health_healthy"]
        });

        let result = storage_primary_readiness_result(status);

        assert_eq!(
            result["schema"],
            "cynic.storage_primary_activation.readiness.v1"
        );
        assert_eq!(result["ready"], false);
        assert_eq!(result["mcp_operation"], "storage.primary.readiness");
        assert_eq!(
            result["next_actions"],
            serde_json::json!([
                "activate_lxc_exec_bridge_on_proxmox_host",
                "activate_synapse_dns_with_network_mesh_dns_activation_bundle",
                "start_surrealdb_and_verify_synapse_health"
            ])
        );
    }

    #[test]
    fn storage_primary_status_dns_resolution_implies_auto_mesh() {
        let params = StoragePrimaryStatusParams {
            bundle_dir: None,
            auto_bundle_dir: None,
            bundle_default_dir: None,
            bridge_status_json: None,
            auto_bridge_status: None,
            fleet_report_json: None,
            mesh_status_json: None,
            auto_mesh_status: None,
            resolve_mesh_dns: Some(true),
            auto_fleet_status: None,
            health_json: None,
            auto_service_health: None,
            service_health_timeout_seconds: None,
        };

        assert!(storage_primary_status_uses_auto_mesh(&params));
    }

    #[test]
    fn effective_status_preserves_planned_but_marks_active_without_observation_stale() {
        assert_eq!(
            effective_status_from_provider("planned", Some("running")),
            "planned"
        );
        assert_eq!(
            effective_status_from_provider("active", Some("running")),
            "active"
        );
        assert_eq!(effective_status_from_provider("active", None), "stale");
        assert_eq!(
            effective_status_from_provider("active", Some("stopped")),
            "stopped"
        );
    }

    #[test]
    fn ipv4_cidr_check_rejects_wrong_plane() {
        assert!(ipv4_in_cidr("10.20.0.3", "10.20.0.0/24"));
        assert!(!ipv4_in_cidr("10.30.0.3", "10.20.0.0/24"));
    }
}

