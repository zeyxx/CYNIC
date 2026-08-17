// ── MCP ServerHandler ───────────────────────────────────────

#[tool_handler]
impl ServerHandler for CynicMcpProxy {
    fn supported_protocol_versions(
        &self,
    ) -> std::borrow::Cow<'static, [rmcp::model::ProtocolVersion]> {
        let versions = crate::protocol::supported_protocol_revisions()
            .into_iter()
            .filter_map(|revision| match revision {
                crate::protocol::TARGET_REVISION => {
                    Some(rmcp::model::ProtocolVersion::V_2026_07_28)
                }
                crate::protocol::LEGACY_REVISION => {
                    Some(rmcp::model::ProtocolVersion::V_2025_11_25)
                }
                _ => None,
            })
            .collect();
        Cow::Owned(versions)
    }

    fn discover(
        &self,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::DiscoverResult, McpError>>
           + MaybeSendFuture
           + '_ {
        let result = rmcp::model::DiscoverResult::from_server_info(
            self.supported_protocol_versions().into_owned(),
            self.get_info(),
        );
        std::future::ready(Ok(result))
    }

    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cynic-mcp", env!("CYNIC_VERSION")))
            .with_instructions("CYNIC MCP proxy — forwards to REST kernel. Zero local state.")
    }
}

#[cfg(test)]
mod http_query_tests {
    use super::*;

    #[test]
    fn query_url_percent_encodes_query_values() {
        let url = query_url(
            "http://kernel.local",
            "/agent-tasks",
            &[
                ("kind", "hermes".to_string()),
                ("domain", "gate 2/request&deploy".to_string()),
            ],
        )
        .expect("query url should parse");

        assert_eq!(
            url,
            "http://kernel.local/agent-tasks?kind=hermes&domain=gate+2%2Frequest%26deploy"
        );
    }
}

#[cfg(test)]
mod observe_tests {
    use super::*;

    #[test]
    fn observe_request_body_forwards_k15_ledger_fields() {
        let body = observe_request_body(&ObserveParams {
            tool: "cynic_lifecycle_dispatch".to_string(),
            target: Some("agent_tasks:test".to_string()),
            domain: Some("supply-chain".to_string()),
            status: Some("completed".to_string()),
            context: Some("dispatch persisted and verified".to_string()),
            project: Some("cynic".to_string()),
            agent_id: Some("cortex-test".to_string()),
            session_id: Some("session-test".to_string()),
            tags: Some(vec!["mempool".to_string(), "k15".to_string()]),
            value: Some(serde_json::json!({
                "task_id": "agent_tasks:test",
                "visible": true
            })),
            confidence: Some("verified".to_string()),
            consumer: Some("organ-forge".to_string()),
            action: Some("read_back".to_string()),
            depends_on: Some(vec!["agent_tasks:parent".to_string()]),
            maturity: Some(0.9),
        });

        assert_eq!(body["tool"], "cynic_lifecycle_dispatch");
        assert_eq!(body["consumer"], "organ-forge");
        assert_eq!(body["confidence"], "verified");
        assert_eq!(body["action"], "read_back");
        assert_eq!(body["value"]["task_id"], "agent_tasks:test");
        assert_eq!(body["value"]["visible"], true);
        assert_eq!(body["depends_on"][0], "agent_tasks:parent");
        assert_eq!(body["maturity"], 0.9);
    }
}

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
            service_endpoint_from_registry(&registry, node, service),
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

#[cfg(test)]
mod lifecycle_dispatch_tests {
    use super::*;

    #[test]
    fn lifecycle_payload_body_defaults_to_forge_dispatch_policy() {
        let payload = serde_json::json!({
            "kind": "hermes",
            "domain": "gate-2-request",
            "agent_id": "organ-forge-request",
            "content": "{\"intent\":\"build-and-test\"}"
        });

        let body = lifecycle_payload_body_with_defaults(&payload);

        assert_eq!(body["priority"], 99);
        assert_eq!(body["consumer"], "cynic-runtime");
        assert_eq!(
            body["escalation_policy"],
            "canonical_build_runner_validation_only_no_cortex_build"
        );
        assert_eq!(body["domain"], "gate-2-request");
    }

    #[test]
    fn lifecycle_payload_body_preserves_explicit_policy_fields() {
        let payload = serde_json::json!({
            "kind": "hermes",
            "domain": "deploy-request",
            "priority": 77,
            "consumer": "custom-forge",
            "escalation_policy": "custom",
            "content": "{\"intent\":\"deploy\"}"
        });

        let body = lifecycle_payload_body_with_defaults(&payload);

        assert_eq!(body["priority"], 77);
        assert_eq!(body["consumer"], "custom-forge");
        assert_eq!(body["escalation_policy"], "custom");
    }

    #[test]
    fn generic_dispatch_result_accepts_pending_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:pending"});
        let pending = serde_json::json!({
            "tasks": [{"id": "agent_tasks:pending"}],
            "count": 1
        });
        let completed = serde_json::json!({"tasks": [], "count": 0});

        let result = verified_agent_task_dispatch_result(
            &response,
            "hermes",
            "mcp-runtime",
            &pending,
            &completed,
        )
        .expect("pending task id should prove dispatch visibility");

        assert_eq!(result["status"], "dispatched");
        assert_eq!(result["task_id"], "agent_tasks:pending");
        assert_eq!(result["visible_dispatched_ids"][0], "agent_tasks:pending");
    }

    #[test]
    fn generic_dispatch_result_rejects_missing_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:missing"});
        let pending = serde_json::json!({"tasks": [], "count": 0});
        let completed = serde_json::json!({"tasks": [], "count": 0});

        let result = verified_agent_task_dispatch_result(
            &response,
            "hermes",
            "mcp-runtime",
            &pending,
            &completed,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-back incomplete"));
    }

    #[test]
    fn task_completion_result_accepts_completed_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:done", "status": "completed"});
        let completed = serde_json::json!({
            "tasks": [{"id": "agent_tasks:done"}],
            "count": 1
        });

        let result = verified_agent_task_completion_result(
            &response,
            "agent_tasks:done",
            "hermes",
            Some("gate-2-request"),
            &completed,
        )
        .expect("completed task id should prove result visibility");

        assert_eq!(result["status"], "completed");
        assert_eq!(result["task_id"], "agent_tasks:done");
        assert_eq!(result["read_back_verified"], true);
        assert_eq!(result["visible_completed_ids"][0], "agent_tasks:done");
    }

    #[test]
    fn task_completion_result_rejects_missing_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:missing", "status": "completed"});
        let completed = serde_json::json!({"tasks": [], "count": 0});

        let result = verified_agent_task_completion_result(
            &response,
            "agent_tasks:missing",
            "hermes",
            Some("gate-2-request"),
            &completed,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-back incomplete"));
    }

    #[test]
    fn lifecycle_payloads_from_plan_rejects_empty_payloads() {
        let payload_plan = serde_json::json!({
            "status": "ok",
            "payloads": []
        });

        let result = lifecycle_payloads_from_plan(&payload_plan);

        assert!(result.is_err());
    }

    #[test]
    fn lifecycle_payload_dry_run_result_marks_non_mutating_plan() {
        let payload_plan = serde_json::json!({
            "status": "ok",
            "payloads": [{"kind": "hermes", "domain": "gate-2-request"}]
        });

        let result = lifecycle_payload_dry_run_result(payload_plan);

        assert_eq!(result["dry_run"], true);
        assert_eq!(result["mutates_mempool"], false);
    }

    #[test]
    fn lifecycle_payload_intent_extracts_inner_content_intent() {
        let payload = serde_json::json!({
            "content": "{\"lifecycle\":\"kernel_release\",\"intent\":\"integration-test\"}"
        });

        assert_eq!(
            lifecycle_payload_intent(&payload),
            Some(serde_json::json!("integration-test"))
        );
    }

    #[test]
    fn lifecycle_payload_domain_uses_first_payload_or_profile_fallback() {
        let payloads = vec![serde_json::json!({"domain": "gate-2-request"})];

        assert_eq!(
            lifecycle_payload_domain(&payloads, "gate-2"),
            "gate-2-request"
        );
        assert_eq!(lifecycle_payload_domain(&[], "gate-2"), "gate-2");
    }

    #[test]
    fn visible_dispatched_task_ids_accepts_pending_or_completed_visibility() {
        let dispatched = vec![
            serde_json::json!({"task_id": "agent_tasks:pending"}),
            serde_json::json!({"task_id": "agent_tasks:completed"}),
            serde_json::json!({"task_id": "agent_tasks:missing"}),
        ];
        let pending = serde_json::json!({
            "tasks": [{"id": "agent_tasks:pending"}],
            "count": 1
        });
        let completed = serde_json::json!({
            "tasks": [{"id": "agent_tasks:completed"}],
            "count": 1
        });

        let (dispatched_ids, visible_ids) =
            visible_dispatched_task_ids(&dispatched, &pending, &completed);

        assert_eq!(dispatched_ids.len(), 3);
        assert_eq!(
            visible_ids,
            vec![
                "agent_tasks:pending".to_string(),
                "agent_tasks:completed".to_string()
            ]
        );
    }

    #[test]
    fn missing_dispatched_task_ids_detects_partial_read_back() {
        let dispatched_ids = vec![
            "agent_tasks:pending".to_string(),
            "agent_tasks:completed".to_string(),
            "agent_tasks:missing".to_string(),
        ];
        let visible_ids = vec![
            "agent_tasks:pending".to_string(),
            "agent_tasks:completed".to_string(),
        ];

        let missing = missing_dispatched_task_ids(&dispatched_ids, &visible_ids);

        assert_eq!(missing, vec!["agent_tasks:missing".to_string()]);
    }
}

#[cfg(test)]
mod mcp_surface_tests {
    use super::*;

    fn exposure_fixture() -> serde_json::Value {
        serde_json::json!({
            "constitutional_execution": {
                "execution_contract": "cynic.agent_workload_envelope.v1"
            },
            "mcp_tool_domains": {
                "organism_status": {
                    "description": "status",
                    "tools": ["cynic_api_contracts", "cynic_mcp_surface"],
                    "operator_tools": []
                },
                "lifecycle": {
                    "description": "lifecycle",
                    "tools": ["cynic_lifecycle_status", "cynic_lifecycle_dispatch"],
                    "operator_tools": ["cynic_lifecycle_status", "cynic_lifecycle_dispatch"]
                }
            },
            "mcp_servers": [
                {
                    "name": "cynic",
                    "tools": [
                        {"name": "cynic_api_contracts"},
                        {"name": "cynic_mcp_surface"},
                        {"name": "cynic_lifecycle_status"},
                        {"name": "cynic_lifecycle_dispatch"}
                    ]
                }
            ]
        })
    }

    #[test]
    fn mcp_surface_findings_accepts_exposed_tools_classified_by_domain() {
        let exposure = exposure_fixture();

        assert_eq!(
            mcp_surface_findings(&exposure),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn mcp_surface_findings_rejects_exposed_tool_without_domain() {
        let mut exposure = exposure_fixture();
        exposure["mcp_servers"][0]["tools"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"name": "unclassified_tool"}));

        let findings = mcp_surface_findings(&exposure);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("exposed_tool_missing_domain")
                && finding.get("tool").and_then(|value| value.as_str()) == Some("unclassified_tool")
        }));
    }

    #[test]
    fn mcp_surface_findings_rejects_domain_tool_not_exposed() {
        let mut exposure = exposure_fixture();
        exposure["mcp_tool_domains"]["coordination"] = serde_json::json!({
            "tools": ["cynic_coord_claim"],
            "operator_tools": ["cynic_coord_claim"]
        });

        let findings = mcp_surface_findings(&exposure);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("domain_tool_not_exposed")
                && finding.get("tool").and_then(|value| value.as_str()) == Some("cynic_coord_claim")
        }));
    }

    #[test]
    fn mcp_domain_surface_sorts_domains_for_stable_agent_reads() {
        let exposure = exposure_fixture();
        let domains = mcp_domain_surface(&exposure);
        let names: Vec<&str> = domains
            .iter()
            .filter_map(|domain| domain.get("name").and_then(|value| value.as_str()))
            .collect();

        assert_eq!(names, vec!["lifecycle", "organism_status"]);
    }
}

#[cfg(test)]
mod infrastructure_mcp_contract_tests {
    use super::*;

    fn contract_fixture() -> serde_json::Value {
        serde_json::json!({
            "schema": "cynic.infrastructure_mcp.v1",
            "status": "target_contract",
            "principles": [
                "deny_by_default",
                "registry_desired_state_first",
                "synapse_dns_is_the_service_contract",
                "mutating_operations_require_preflight_and_audit",
                "private_keys_never_leave_the_node"
            ],
            "operation_families": {
                "fleet": {
                    "operations": {
                        "fleet.node.exec": {
                            "mcp_tool": "fleet_node_exec",
                            "mutates": true,
                            "current_coverage": [
                                "lxc_structured_precondition_missing_until_exec_bridge_exists_or_transport_configured"
                            ],
                            "lxc_bridge_contract": "axioms/lxc-exec-bridge-v1.json"
                        }
                    },
                    "lxc_exec_bridge.plan": {
                        "mcp_tool": "lxc_exec_bridge_plan",
                        "mutates": false
                    },
                    "lxc_exec_bridge.bundle": {
                        "mcp_tool": "lxc_exec_bridge_bundle",
                        "mutates": false,
                        "writes_local_output_dir": true
                    },
                    "lxc_exec_bridge.status": {
                        "mcp_tool": "lxc_exec_bridge_status",
                        "mutates": false
                    }
                },
                "mesh": {
                    "operations": {
                        "network.mesh.status": {
                            "mcp_tool": "network_mesh_status",
                            "mutates": false
                        },
                        "network.mesh.dns.render": {
                            "mcp_tool": "network_mesh_dns_render",
                            "mutates": false
                        },
                        "network.mesh.dns.activation_plan": {
                            "mcp_tool": "network_mesh_dns_activation_plan",
                            "mutates": false
                        },
                        "network.mesh.dns.activation_bundle": {
                            "mcp_tool": "network_mesh_dns_activation_bundle",
                            "mutates": false,
                            "writes_local_output_dir": true
                        },
                        "network.mesh.dns.activation_status": {
                            "mcp_tool": "network_mesh_dns_activation_status",
                            "mutates": false
                        }
                    }
                },
                "storage": {
                    "operations": {
                        "storage.primary.plan": {
                            "mcp_tool": "storage_primary_plan",
                            "mutates": false
                        },
                        "storage.primary.bundle": {
                            "mcp_tool": "storage_primary_bundle",
                            "mutates": false,
                            "writes_local_output_dir": true
                        },
                        "storage.primary.status": {
                            "mcp_tool": "storage_primary_status",
                            "mutates": false,
                            "preconditions_reported": [
                                "registry_declares_storage_node",
                                "endpoint_is_synapse_dns",
                                "bridge_active",
                                "fleet_observes_node_running",
                                "mesh_reaches_storage",
                                "bundle_complete",
                                "service_health_healthy"
                            ]
                        },
                        "storage.primary.readiness": {
                            "mcp_tool": "storage_primary_readiness",
                            "mutates": false,
                            "composes": [
                                "storage.primary.status",
                                "lxc_exec_bridge.status",
                                "network.mesh.status",
                                "network.mesh.dns.activation_status",
                                "fleet.node.status",
                                "service.health"
                            ]
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn infrastructure_mcp_contract_accepts_expected_contract_shape() {
        let contract = contract_fixture();

        assert_eq!(
            infrastructure_mcp_contract_findings("axioms/infrastructure-mcp-v1.json", &contract),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn infrastructure_mcp_contract_rejects_tool_drift() {
        let mut contract = contract_fixture();
        contract["operation_families"]["storage"]["operations"]["storage.primary.status"]
            ["mcp_tool"] = serde_json::json!("wrong_tool");

        let findings =
            infrastructure_mcp_contract_findings("axioms/infrastructure-mcp-v1.json", &contract);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infrastructure_mcp_tool_drift")
        }));
    }

    #[test]
    fn infrastructure_mcp_contract_rejects_missing_readiness_dependency() {
        let mut contract = contract_fixture();
        contract["operation_families"]["storage"]["operations"]["storage.primary.readiness"]
            ["composes"] = serde_json::json!(["storage.primary.status"]);

        let findings =
            infrastructure_mcp_contract_findings("axioms/infrastructure-mcp-v1.json", &contract);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infrastructure_mcp_readiness_dependency_missing")
        }));
    }
}

#[cfg(test)]
mod command_surfaces_contract_tests {
    use super::*;

    fn base_contract() -> serde_json::Value {
        serde_json::json!({
            "schema": "cynic.command_surfaces.v1",
            "surface_roles": {
                "human_operator": {},
                "interactive_agent": {},
                "machine_fallback": {},
                "validation_dev": {},
                "runtime_daemon": {}
            },
            "commands": [
                {
                    "name": "cynic_lifecycle_dispatch",
                    "role": "interactive_agent",
                    "domain": "lifecycle",
                    "canonical": true,
                    "mutates": true,
                    "mutates_mempool": true,
                    "json_output_required": true,
                    "read_back_required": true
                },
                {
                    "name": "make lint-organ-memory-contract",
                    "role": "validation_dev",
                    "domain": "validation",
                    "canonical": true,
                    "mutates": false,
                    "mutates_mempool": false,
                    "json_output_required": true
                }
            ]
        })
    }

    fn base_exposure() -> serde_json::Value {
        serde_json::json!({
            "mcp_tool_domains": {
                "lifecycle": {
                    "tools": ["cynic_lifecycle_dispatch"]
                }
            }
        })
    }

    #[test]
    fn command_surfaces_contract_accepts_valid_shape() {
        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &base_contract(),
            &base_exposure(),
            &std::collections::BTreeSet::from(["lint-organ-memory-contract".to_string()]),
        );

        assert_eq!(findings, Vec::<serde_json::Value>::new());
    }

    #[test]
    fn command_surfaces_contract_rejects_machine_command_without_json() {
        let mut contract = base_contract();
        contract["commands"][0]["json_output_required"] = serde_json::json!(false);

        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &contract,
            &base_exposure(),
            &std::collections::BTreeSet::from(["lint-organ-memory-contract".to_string()]),
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("machine_command_without_json_output")
        }));
    }

    #[test]
    fn command_surfaces_contract_rejects_mempool_mutation_without_read_back() {
        let mut contract = base_contract();
        contract["commands"][0]["read_back_required"] = serde_json::json!(false);

        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &contract,
            &base_exposure(),
            &std::collections::BTreeSet::from(["lint-organ-memory-contract".to_string()]),
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("mempool_mutation_without_read_back")
        }));
    }

    #[test]
    fn command_surfaces_contract_rejects_canonical_mutating_lifecycle_make_target() {
        let mut contract = base_contract();
        contract["commands"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "make deploy-request",
                "role": "validation_dev",
                "domain": "lifecycle",
                "canonical": true,
                "mutates": true,
                "mutates_mempool": true,
                "json_output_required": false,
                "read_back_required": true
            }));

        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &contract,
            &base_exposure(),
            &std::collections::BTreeSet::from([
                "lint-organ-memory-contract".to_string(),
                "deploy-request".to_string(),
            ]),
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("make_lifecycle_mutation_marked_canonical")
        }));
    }
}

#[cfg(test)]
mod infra_consumption_contract_tests {
    use super::*;

    fn temp_fixture_dir(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "cynic-mcp-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("fixture directory should be created");
        dir
    }

    #[test]
    fn infra_consumption_contract_accepts_expected_contract_shape() {
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["build_runner"],
            "required_ports": ["LifecycleDispatchPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["cargo build -p cynic-mcp"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["Makefile"],
            "forbidden_patterns": [
                {
                    "id": "cortex_local_mcp_build",
                    "pattern": "cargo build -p cynic-mcp",
                    "severity": "critical",
                    "detail": "MCP builds must be lifecycle-dispatched."
                }
            ]
        });

        assert_eq!(
            infra_consumption_contract_findings(
                "/tmp",
                "axioms/infra-consumption-v1.json",
                &contract
            ),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn infra_consumption_contract_rejects_uncovered_forbidden_on_cortex_rule() {
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["build_runner"],
            "required_ports": ["LifecycleDispatchPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["rsync target/release/cynic-kernel"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["Makefile"],
            "forbidden_patterns": [
                {
                    "id": "cortex_local_mcp_build",
                    "pattern": "cargo build -p cynic-mcp",
                    "severity": "critical",
                    "detail": "MCP builds must be lifecycle-dispatched."
                }
            ]
        });

        let findings = infra_consumption_contract_findings(
            "/tmp",
            "axioms/infra-consumption-v1.json",
            &contract,
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infra_consumption_forbidden_on_cortex_uncovered")
        }));
    }

    #[test]
    fn infra_consumption_contract_rejects_forbidden_cortex_local_mcp_build() {
        let temp = temp_fixture_dir("infra-consumption-mcp-build");
        let scanned = temp.join("wrapper.sh");
        std::fs::write(&scanned, "cargo build -p cynic-mcp --release\n")
            .expect("fixture file should be written");
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["build_runner"],
            "required_ports": ["LifecycleDispatchPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["cargo build -p cynic-mcp"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["wrapper.sh"],
            "forbidden_patterns": [
                {
                    "id": "cortex_local_mcp_build",
                    "pattern": "cargo build -p cynic-mcp",
                    "severity": "critical",
                    "detail": "MCP builds must be lifecycle-dispatched."
                }
            ]
        });

        let findings = infra_consumption_contract_findings(
            temp.to_str().expect("temp path should be utf-8"),
            "axioms/infra-consumption-v1.json",
            &contract,
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infra_consumption_forbidden_cortex_local_mcp_build")
        }));
    }

    #[test]
    fn infra_consumption_contract_rejects_cortex_local_surrealdb_endpoint() {
        let temp = temp_fixture_dir("infra-consumption-surreal");
        let scanned = temp.join("Makefile");
        std::fs::write(
            &scanned,
            "backup:\n\tsurreal export --endpoint http://localhost:8000\n",
        )
        .expect("fixture file should be written");
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["storage_primary"],
            "required_ports": ["StoragePrimaryPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["surreal --endpoint http://localhost:8000"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["Makefile"],
            "forbidden_patterns": [
                {
                    "id": "local_surrealdb_endpoint",
                    "pattern": "surreal (import|sql|export|is-ready) --endpoint http://localhost:8000",
                    "severity": "critical",
                    "detail": "SurrealDB must be consumed through storage_primary/Synapse or Kernel coordination API."
                }
            ]
        });

        let findings = infra_consumption_contract_findings(
            temp.to_str().expect("temp path should be utf-8"),
            "axioms/infra-consumption-v1.json",
            &contract,
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infra_consumption_forbidden_local_surrealdb_endpoint")
        }));
    }
}

#[cfg(test)]
mod api_topology_contract_tests {
    use super::*;

    #[test]
    fn api_geometry_report_detects_root_split_and_dangerous_operation() {
        let openapi = serde_json::json!({
            "paths": {
                "/legacy/a": {"get": {}},
                "/legacy/b": {"get": {}},
                "/lifecycle/deploy": {"post": {}}
            }
        });
        let topology = serde_json::json!({
            "topology_budget": {"max_root_splits": 0, "max_transitional_operations": 0},
            "operations": {
                "GET /legacy/a": {
                    "layer": "compatibility",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/canonical-a",
                    "status": "transitional"
                },
                "GET /legacy/b": {
                    "layer": "compatibility",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/canonical-b",
                    "status": "canonical"
                },
                "POST /lifecycle/deploy": {
                    "layer": "metabolism",
                    "mutation_level": "deployment",
                    "agent_safe_default": false,
                    "canonical_family": "/lifecycle",
                    "status": "canonical"
                }
            }
        });

        let roots: std::collections::BTreeSet<String> = topology
            .get("operations")
            .and_then(|value| value.as_object())
            .unwrap()
            .keys()
            .map(|operation| api_operation_root(operation))
            .collect();

        assert!(roots.contains("/legacy"));
        assert!(openapi_operations_json(&openapi).contains("POST /lifecycle/deploy"));
    }

    #[test]
    fn api_topology_contract_rejects_openapi_operation_without_topology_entry() {
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "x-cynic-api-topology-contract": {
                "schema": "cynic.api_topology.v1",
                "path": "axioms/api-topology-v1.json"
            },
            "x-cynic-api-layer-alignment-contract": {
                "schema": "cynic.api_layer_alignment.v1",
                "path": "axioms/api-layer-alignment-v1.json"
            },
            "paths": {
                "/openapi.json": {"get": {"security": []}},
                "/api/topology": {"get": {"security": []}},
                "/api/layer-alignment": {"get": {"security": []}},
                "/unmapped": {"get": {"security": []}}
            }
        });
        let topology = serde_json::json!({
            "schema": "cynic.api_topology.v1",
            "status": "normative",
            "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
            "allowed": {
                "layers": ["perception"],
                "mutation_levels": ["read"],
                "statuses": ["canonical"]
            },
            "topology_budget": {
                "max_root_splits": 0,
                "max_transitional_operations": 0,
                "max_legacy_operations": 0,
                "compat_operations": {"allowed": false}
            },
            "public_discovery_endpoints": {
                "GET /openapi.json": {"serves": "docs/api/openapi.json"},
                "GET /api/topology": {"serves": "axioms/api-topology-v1.json"},
                "GET /api/layer-alignment": {"serves": "axioms/api-layer-alignment-v1.json"}
            },
            "operations": {
                "GET /openapi.json": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/api",
                    "status": "canonical"
                },
                "GET /api/topology": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/api",
                    "status": "canonical"
                },
                "GET /api/layer-alignment": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/api",
                    "status": "canonical"
                }
            }
        });
        let alignment = serde_json::json!({
            "schema": "cynic.api_layer_alignment.v1",
            "layers": {"perception": {}}
        });
        let discovery = concat!(
            "const OPENAPI_JSON: &str = include_str!(\"../../../../../docs/api/openapi.json\");\n",
            "const API_TOPOLOGY_JSON: &str = include_str!(\"../../../../../axioms/api-topology-v1.json\");\n",
            "const API_LAYER_ALIGNMENT_JSON: &str = include_str!(\"../../../../../axioms/api-layer-alignment-v1.json\");\n",
            "pub async fn openapi_handler() -> Response { json_contract(OPENAPI_JSON) }\n",
            "pub async fn api_topology_handler() -> Response { json_contract(API_TOPOLOGY_JSON) }\n",
            "pub async fn api_layer_alignment_handler() -> Response { json_contract(API_LAYER_ALIGNMENT_JSON) }\n"
        );
        let middleware = "path == \"/openapi.json\" || path == \"/api/topology\" || path == \"/api/layer-alignment\"\n";

        let findings = api_topology_contract_findings(
            "/tmp", &openapi, &topology, &alignment, discovery, middleware,
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("api_topology_operation_missing")
        }));
    }
}

#[cfg(test)]
mod api_layer_alignment_contract_tests {
    use super::*;

    #[test]
    fn api_layer_alignment_rejects_symbolic_vocabulary_drift() {
        let contract = serde_json::json!({
            "schema": "cynic.api_layer_alignment.v1",
            "status": "normative",
            "symbolic_vocabulary": {"Keter": "source"},
            "guardrails": ["analogy stays below evidence"],
            "layers": {
                "perception": {
                    "engineering_role": "test",
                    "agent_question": "test?",
                    "system_primitives": ["read", "poll", "file descriptor"],
                    "symbolic_alignment": ["Keter"],
                    "must": ["test"],
                    "must_not": ["test"]
                }
            },
            "vertical_alignment_rules": [{"id": "test", "rule": "test", "enforcement": "validator"}]
        });
        let topology = serde_json::json!({
            "schema": "cynic.api_topology.v1",
            "status": "normative",
            "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
            "allowed": {"layers": ["perception"]},
            "operations": {}
        });
        let peer = serde_json::json!({
            "schema": "test",
            "status": "normative",
            "symbolic_vocabulary": {
                "Keter": "source",
                "Binah": "bounded form"
            }
        });

        let findings = api_layer_alignment_contract_findings(
            &contract,
            &topology,
            &peer,
            &peer,
            "axioms/api-layer-alignment-v1.json",
            "axioms/api-topology-v1.json",
            "axioms/meta-layer-contract-v1.json",
            "axioms/organ-layer-topology-v1.json",
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("api_layer_symbolic_vocabulary_missing_peer_symbols")
        }));
    }

    #[test]
    fn api_layer_alignment_rejects_missing_system_primitives_strength() {
        let contract = serde_json::json!({
            "schema": "cynic.api_layer_alignment.v1",
            "status": "normative",
            "symbolic_vocabulary": {"Keter": "source"},
            "guardrails": ["analogy stays below evidence"],
            "layers": {
                "perception": {
                    "engineering_role": "test",
                    "agent_question": "test?",
                    "system_primitives": ["read", "poll"],
                    "symbolic_alignment": ["Keter"],
                    "must": ["test"],
                    "must_not": ["test"]
                }
            },
            "vertical_alignment_rules": [{"id": "test", "rule": "test", "enforcement": "validator"}]
        });
        let topology = serde_json::json!({
            "schema": "cynic.api_topology.v1",
            "status": "normative",
            "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
            "allowed": {"layers": ["perception"]},
            "operations": {}
        });
        let peer = serde_json::json!({
            "schema": "test",
            "status": "normative",
            "symbolic_vocabulary": {"Keter": "source"}
        });

        let findings = api_layer_alignment_contract_findings(
            &contract,
            &topology,
            &peer,
            &peer,
            "axioms/api-layer-alignment-v1.json",
            "axioms/api-topology-v1.json",
            "axioms/meta-layer-contract-v1.json",
            "axioms/organ-layer-topology-v1.json",
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("api_layer_alignment_system_primitives_too_weak")
        }));
    }
}

#[cfg(test)]
mod openapi_route_contract_tests {
    use super::*;

    #[test]
    fn openapi_route_contract_rejects_router_path_missing_from_openapi() {
        let router_text =
            "Router::new().route(\"/health\", get(health)).route(\"/missing\", get(missing));\n";
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "paths": {
                "/health": {"get": {}}
            }
        });

        let findings = openapi_route_contract_findings("/tmp", router_text, &openapi);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("openapi_route_missing")
        }));
    }

    #[test]
    fn openapi_route_contract_rejects_stale_openapi_path() {
        let router_text = "Router::new().route(\"/health\", get(health));\n";
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "paths": {
                "/health": {"get": {}},
                "/stale": {"get": {}}
            }
        });

        let findings = openapi_route_contract_findings("/tmp", router_text, &openapi);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("openapi_route_stale")
        }));
    }
}

#[cfg(test)]
mod hardcoded_paths_contract_tests {
    use super::*;

    #[test]
    fn hardcoded_paths_contract_rejects_home_path_usage() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-hardcoded-paths-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        let file = temp.join("example.py");
        let home_path = format!("/{}/user/project", "home");
        std::fs::write(&file, format!("ROOT = \"{home_path}\"\n"))
            .expect("fixture file should be written");

        let findings = hardcoded_path_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["example.py"],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("hardcoded_absolute_path")
        }));
    }

    #[test]
    fn hardcoded_paths_contract_accepts_relative_or_env_paths() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-hardcoded-paths-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        let file = temp.join("example.py");
        std::fs::write(&file, "ROOT = os.getenv(\"CYNIC_CONFIG_ROOT\")\n")
            .expect("fixture file should be written");

        let findings = hardcoded_path_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["example.py"],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod reqwest_usage_contract_tests {
    use super::*;

    #[test]
    fn reqwest_usage_contract_rejects_domain_reqwest_usage() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-reqwest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let domain_dir = temp.join("crates/cynic-kernel/src/domain");
        std::fs::create_dir_all(&domain_dir).expect("fixture directory should be created");
        let file = domain_dir.join("bad.rs");
        std::fs::write(&file, "use reqwest::Client;\n").expect("fixture file should be written");

        let findings = validate_reqwest_usage(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("misplaced_reqwest_usage")
        }));
    }

    #[test]
    fn reqwest_usage_contract_rejects_removed_kernel_proxy_path() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-reqwest-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let proxy_dir = temp.join("crates/cynic-kernel/src/api/mcp");
        std::fs::create_dir_all(&proxy_dir).expect("fixture directory should be created");
        let file = proxy_dir.join("proxy.rs");
        std::fs::write(&file, "use reqwest::Client;\n").expect("fixture file should be written");

        let findings = validate_reqwest_usage(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("misplaced_reqwest_usage")
        }));
    }
}

#[cfg(test)]
mod secrets_contract_tests {
    use super::*;

    #[test]
    fn secrets_contract_rejects_raw_secret() {
        let findings = secret_findings_from_diff("+API_KEY=sk-abcdefghijklmnopqrstuvwxyz\n");

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("raw_secret_exposed")
        }));
    }

    #[test]
    fn secrets_contract_rejects_hardcoded_credential() {
        let findings = secret_findings_from_diff("+token: supersecretvalue\n");

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("hardcoded_credential_exposed")
        }));
    }

    #[test]
    fn secrets_contract_allows_excluded_patterns() {
        let findings = secret_findings_from_diff("+api_key=api_key\n+token?: string\n");

        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod ip_contract_tests {
    use super::*;

    #[test]
    fn ip_contract_rejects_staged_hardcoded_ip() {
        let findings = ip_findings_from_diff("+server = \"100.74.12.5\"\n");

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("staged_hardcoded_ip")
        }));
    }

    #[test]
    fn ip_contract_rejects_backends_hardcoded_ip() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-ips-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(temp.join("backends.toml"), "host = \"100.75.1.8\"\n")
            .expect("fixture file should be written");

        let findings = ip_findings_from_backends(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("backends_hardcoded_ip")
        }));
    }

    #[test]
    fn ip_contract_accepts_no_ip() {
        let findings = ip_findings_from_diff("+server = env(\"HOST\")\n");
        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod unlogged_ok_contract_tests {
    use super::*;

    #[test]
    fn unlogged_ok_contract_rejects_unlogged_ok_call() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-unlogged-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("bad.rs"),
            "fn test() {\n    let _ = thing.ok();\n}\n",
        )
        .expect("fixture file should be written");

        let findings = unlogged_ok_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["bad.rs".to_string()],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("unlogged_ok_call")
        }));
    }

    #[test]
    fn unlogged_ok_contract_allows_logged_or_exempt_ok_call() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-unlogged-ok-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("good.rs"),
            "fn test() {\n    tracing::warn!(\"x\");\n    let _ = thing.ok();\n    let _ = env::var(\"A\").ok();\n}\n",
        )
        .expect("fixture file should be written");

        let findings = unlogged_ok_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["good.rs".to_string()],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod allow_without_why_contract_tests {
    use super::*;

    #[test]
    fn allow_without_why_contract_rejects_allow_without_context() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-allow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let kernel_dir = temp.join("crates/cynic-kernel/src");
        std::fs::create_dir_all(&kernel_dir).expect("fixture directory should be created");
        std::fs::write(
            kernel_dir.join("bad.rs"),
            "#[allow(clippy::too_many_arguments)]\nfn x() {}\n",
        )
        .expect("fixture file should be written");

        let findings =
            validate_allow_without_why(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("allow_without_why")
        }));
    }

    #[test]
    fn allow_without_why_contract_accepts_nearby_why_comment() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-allow-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let kernel_dir = temp.join("crates/cynic-kernel/src");
        std::fs::create_dir_all(&kernel_dir).expect("fixture directory should be created");
        std::fs::write(
            kernel_dir.join("good.rs"),
            "// WHY: justified\n#[allow(clippy::too_many_arguments)]\nfn x() {}\n",
        )
        .expect("fixture file should be written");

        let findings =
            validate_allow_without_why(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod pipeline_functions_contract_tests {
    use super::*;

    #[test]
    fn pipeline_functions_contract_rejects_reimplementation() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-pipeline-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("bad.rs"),
            "fn compute_qscore(x: i32) -> i32 { x }\n",
        )
        .expect("fixture file should be written");

        let findings = pipeline_function_findings(
            temp.as_path(),
            temp.to_str().expect("temp path should be utf-8"),
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("reimplemented_pipeline_function")
        }));
    }

    #[test]
    fn pipeline_functions_contract_ignores_comments() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-pipeline-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("good.rs"),
            "// fn compute_qscore(x: i32) -> i32 { x }\n",
        )
        .expect("fixture file should be written");

        let findings = pipeline_function_findings(
            temp.as_path(),
            temp.to_str().expect("temp path should be utf-8"),
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod coordination_profile_tests {
    use super::*;
    use std::path::Path;

    fn load_coordination_protocol_for_tests() -> serde_json::Value {
        let mut cursor = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        let protocol_path = loop {
            let candidate = cursor.join("contracts/foundation.json");
            if candidate.is_file() {
                break candidate;
            }
            match cursor.parent() {
                Some(parent) => cursor = parent.to_path_buf(),
                None => break cursor.join("contracts/foundation.json"),
            }
        };
        let path = protocol_path;
        let content = std::fs::read_to_string(&path)
            .expect("coordination protocol fixture should be readable in test workspace");
        serde_json::from_str(&content).expect("coordination protocol fixture should be valid json")
    }

    #[test]
    fn contract_events_required_fields_merges_profile_minimum_fields() {
        let protocol = load_coordination_protocol_for_tests();
        let profile = serde_json::json!(["branch", "owner", "scope"]);

        let required =
            contract_events_required_fields(&protocol, Some(&profile), "workstream.started");

        assert_eq!(required, vec!["branch", "owner", "scope"]);
    }

    #[test]
    fn contract_events_required_fields_remains_protocol_only_if_profile_absent() {
        let protocol = load_coordination_protocol_for_tests();
        let required = contract_events_required_fields(&protocol, None, "decision.record");

        assert_eq!(required, Vec::<String>::new());
    }
}
