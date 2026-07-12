use super::types::{RuntimeExecutionLayer, runtime_shared_responsibility};

pub fn runtime_execution_layers(registry_path: &str) -> Vec<RuntimeExecutionLayer> {
    vec![
        RuntimeExecutionLayer {
            id: "l0_registry_and_paths",
            name: "Registry And Runtime Paths",
            purpose: "Resolve desired topology, local runtime paths, and working directories before touching hosts.",
            cloud_model: "IaC control plane",
            service_model: "control_plane",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "declare topology",
                    "choose deployment target",
                    "validate desired state",
                ],
                &["provide local filesystem and execution substrate"],
            ),
            mutability: "read_only",
            cloud_characteristics: vec!["on_demand_self_service", "measured_service"],
            risk_controls: vec!["registry_json_valid", "no_secret_material_in_registry"],
            entry_gate: vec!["registry_json_valid"],
            read_only_probe: runtime_cmd(&[
                "cynic",
                "runtime",
                "paths",
                "--json",
                "--registry",
                registry_path,
            ]),
            dry_run: None,
            apply: None,
            exit_gate: vec!["runtime_paths_resolved", "registry_consumed"],
        },
        RuntimeExecutionLayer {
            id: "l1_host_identity",
            name: "Host Identity And Bootstrap",
            purpose: "Classify the current host against registry targets and identify missing OS prerequisites.",
            cloud_model: "IaaS substrate",
            service_model: "IaaS",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "install operating system prerequisites",
                    "configure host firewall",
                    "protect OS and runtime software",
                ],
                &[
                    "provide compute",
                    "provide memory",
                    "provide virtualization boundary",
                ],
            ),
            mutability: "read_only_then_bounded_apply",
            cloud_characteristics: vec!["broad_network_access", "resource_pooling"],
            risk_controls: vec![
                "bounded_privilege_helpers",
                "identity_and_access_control",
                "host_bootstrap_evidence",
            ],
            entry_gate: vec!["runtime_paths_resolved"],
            read_only_probe: runtime_cmd(&[
                "cynic",
                "bootstrap",
                "constellation",
                "--json",
                "--registry",
                registry_path,
            ]),
            dry_run: Some(runtime_cmd(&[
                "cynic",
                "bootstrap",
                "host",
                "apply-report",
                "--registry",
                registry_path,
            ])),
            apply: Some(runtime_cmd(&[
                "cynic",
                "bootstrap",
                "host",
                "apply",
                "--registry",
                registry_path,
                "--apply",
            ])),
            exit_gate: vec!["host_identity_observed", "host_bootstrap_target_satisfied"],
        },
        RuntimeExecutionLayer {
            id: "l2_network_projection",
            name: "Network Projection",
            purpose: "Generate DNS and provider-fallback artifacts from the registry without assuming one CIDR or DNS zone.",
            cloud_model: "network as code",
            service_model: "IaaS_network",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "administer virtual network",
                    "configure DNS",
                    "configure firewall routes",
                ],
                &["provide network transport", "provide carrier connectivity"],
            ),
            mutability: "artifact_generation_then_bounded_apply",
            cloud_characteristics: vec!["broad_network_access", "rapid_elasticity"],
            risk_controls: vec![
                "dns_zone_from_registry",
                "provider_fallback_is_noncanonical",
                "no_default_zone_leak",
            ],
            entry_gate: vec!["host_identity_observed"],
            read_only_probe: runtime_cmd(&[
                "cynic",
                "network",
                "runtime",
                "--registry",
                registry_path,
            ]),
            dry_run: Some(runtime_cmd(&[
                "cynic",
                "network",
                "dns",
                "bundle",
                "--registry",
                registry_path,
                "--output-dir",
                "<artifact-dir>/dns",
            ])),
            apply: Some(runtime_cmd(&[
                "cynic",
                "network",
                "actuator",
                "apply",
                "--registry",
                registry_path,
                "--profile",
                "<profile>",
                "--apply",
            ])),
            exit_gate: vec!["dns_bundle_generated", "network_runtime_observed"],
        },
        RuntimeExecutionLayer {
            id: "l3_storage_projection",
            name: "Storage Projection",
            purpose: "Prepare stateful storage schemas and health probes before kernel activation.",
            cloud_model: "PaaS data service",
            service_model: "PaaS",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "define schema",
                    "define backup policy",
                    "define access control",
                    "define encryption requirements",
                ],
                &[
                    "provide durable storage substrate",
                    "provide block_or_object_storage",
                ],
            ),
            mutability: "artifact_generation_then_operator_apply",
            cloud_characteristics: vec!["resource_pooling", "measured_service", "sla_or_exit_gate"],
            risk_controls: vec![
                "redundancy_policy_required",
                "encryption_at_rest_required",
                "encryption_in_transit_required",
                "storage_acl_or_rbac_required",
            ],
            entry_gate: vec!["network_runtime_observed"],
            read_only_probe: runtime_cmd(&[
                "cynic",
                "network",
                "runtime",
                "--registry",
                registry_path,
            ]),
            dry_run: Some(vec![
                "surreal".to_string(),
                "validate".to_string(),
                "infra/surreal_init.surql".to_string(),
            ]),
            apply: None,
            exit_gate: vec!["storage_schema_valid", "storage_service_observed"],
        },
        RuntimeExecutionLayer {
            id: "l4_service_runtime",
            name: "Service Runtime",
            purpose: "Install and supervise concrete systemd services declared by the registry.",
            cloud_model: "runtime platform",
            service_model: "PaaS_runtime",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "package binaries",
                    "configure service units",
                    "monitor service health",
                ],
                &["provide OS process manager", "provide compute substrate"],
            ),
            mutability: "bounded_apply",
            cloud_characteristics: vec!["on_demand_self_service", "rapid_elasticity"],
            risk_controls: vec!["systemd_unit_projection", "service_health_sampling"],
            entry_gate: vec!["storage_service_observed"],
            read_only_probe: runtime_cmd(&[
                "cynic",
                "runtime",
                "paths",
                "--json",
                "--registry",
                registry_path,
            ]),
            dry_run: Some(runtime_cmd(&[
                "cynic",
                "bootstrap",
                "host",
                "plan",
                "--registry",
                registry_path,
            ])),
            apply: None,
            exit_gate: vec!["systemd_units_projected", "services_sampled"],
        },
        RuntimeExecutionLayer {
            id: "l5_kernel_and_control_plane",
            name: "Kernel And Control Plane",
            purpose: "Activate the kernel/control-plane surfaces only after substrate, network, storage, and services have evidence.",
            cloud_model: "control plane",
            service_model: "PaaS_control_plane",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "operate kernel service",
                    "publish health endpoint",
                    "enforce lifecycle gates",
                ],
                &["provide reachable host and network substrate"],
            ),
            mutability: "bounded_apply",
            cloud_characteristics: vec!["sla_or_exit_gate", "measured_service"],
            risk_controls: vec![
                "kernel_health_observed",
                "audit_trail_required",
                "rollback_path_required",
            ],
            entry_gate: vec!["services_sampled"],
            read_only_probe: runtime_cmd(&[
                "cynic",
                "network",
                "evidence",
                "--registry",
                registry_path,
            ]),
            dry_run: Some(runtime_cmd(&[
                "cynic",
                "bootstrap",
                "constellation",
                "--json",
                "--registry",
                registry_path,
            ])),
            apply: None,
            exit_gate: vec!["kernel_health_observed", "control_plane_reachable"],
        },
        RuntimeExecutionLayer {
            id: "l6_agent_exposure",
            name: "Agent Exposure",
            purpose: "Expose only the runtime capabilities that are backed by active binaries and explicit policy.",
            cloud_model: "SaaS/operator interface",
            service_model: "SaaS",
            deployment_model: "private_or_hybrid",
            shared_responsibility: runtime_shared_responsibility(
                &[
                    "project agent capabilities",
                    "limit exposed tools",
                    "audit tool usage",
                ],
                &[
                    "provide consumer access path",
                    "provide transport availability",
                ],
            ),
            mutability: "projection",
            cloud_characteristics: vec!["broad_network_access", "measured_service"],
            risk_controls: vec![
                "least_privilege_exposure",
                "auditor_read_path",
                "no_deleted_surface_exposed",
            ],
            entry_gate: vec!["control_plane_reachable"],
            read_only_probe: runtime_cmd(&["cynic", "commands", "--json"]),
            dry_run: None,
            apply: None,
            exit_gate: vec!["agent_surface_projected", "no_deleted_surface_exposed"],
        },
    ]
}

fn runtime_cmd(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_layers_encode_bottom_up_gate_order() {
        let layers = runtime_execution_layers("infra/registry.json");
        let ids = layers.iter().map(|layer| layer.id).collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "l0_registry_and_paths",
                "l1_host_identity",
                "l2_network_projection",
                "l3_storage_projection",
                "l4_service_runtime",
                "l5_kernel_and_control_plane",
                "l6_agent_exposure"
            ]
        );

        let network = layers
            .iter()
            .find(|layer| layer.id == "l2_network_projection")
            .expect("network layer exists");
        assert_eq!(network.entry_gate, vec!["host_identity_observed"]);
        assert_eq!(
            network.exit_gate,
            vec!["dns_bundle_generated", "network_runtime_observed"]
        );

        let storage = layers
            .iter()
            .find(|layer| layer.id == "l3_storage_projection")
            .expect("storage layer exists");
        assert_eq!(storage.entry_gate, vec!["network_runtime_observed"]);
    }
}
