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

