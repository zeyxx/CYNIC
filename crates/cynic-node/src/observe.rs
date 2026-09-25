<<<<<<< HEAD
use cynic_build::BUILD_RUNNER_OBSERVATION_SCHEMA;

=======
>>>>>>> github/main
use crate::adapters::host_observation::{CommandObservation, TcpObservation};
#[cfg(any(test, kani))]
use crate::build_attestation::runtime_binary_attestation;
use crate::build_attestation::{BinaryAttestation, RuntimeArtifactDeclarationDrift};
use crate::domain::network_projection::{
    CanonicalNetworkMeshStatusProjection, DnsProjectionObservation, SharedRuntimeObservation,
    WireguardTransportObservation,
};
#[cfg(any(test, kani))]
use crate::domain::network_projection::{
    DnsProjectionChecks, DnsProjectionState, WireguardTransportChecks, WireguardTransportState,
    canonical_network_mesh_status_projection, shared_runtime_observation,
};
#[cfg(any(test, kani))]
use crate::domain::node_observation::{NODE_OBSERVATION_SCHEMA, classify_runtime};
use crate::domain::node_observation::{
    NodeRuntimeBlocker, NodeRuntimeCapabilities, NodeRuntimeRemediation, NodeRuntimeSeverity,
    NodeRuntimeStatus,
};
use crate::domain::observation_runtime::DnsResolutionObservation;
#[cfg(any(test, kani))]
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TopologyObservation {
    pub(crate) source: String,
    pub(crate) registry_path: Option<String>,
    pub(crate) canonical_service_plane: String,
    pub(crate) synapse_cidr: String,
    pub(crate) provider_fallback_cidr: String,
    pub(crate) provider_fallback_gateway: Option<String>,
    pub(crate) dns_zone: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkObservation {
    pub(crate) interfaces: CommandObservation,
    pub(crate) routes: CommandObservation,
    pub(crate) provider_fallback_route: CommandObservation,
    pub(crate) synapse_route: CommandObservation,
    pub(crate) wireguard: CommandObservation,
    pub(crate) network_manager_connections: CommandObservation,
    pub(crate) policy_routes: CommandObservation,
    pub(crate) namespaces: CommandObservation,
    pub(crate) firewall: CommandObservation,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsObservation {
    pub(crate) critical_hosts: Vec<DnsResolutionObservation>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SystemdObservation {
    pub(crate) wg_quick_wg0: CommandObservation,
    pub(crate) network_manager: CommandObservation,
    pub(crate) systemd_resolved: CommandObservation,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ServiceObservation {
    pub(crate) systemd: SystemdObservation,
    pub(crate) tcp: Vec<TcpObservation>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NodeObservation {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) hostname: String,
    pub(crate) binary_attestation: BinaryAttestation,
    pub(crate) status: NodeRuntimeStatus,
    pub(crate) severity: NodeRuntimeSeverity,
    pub(crate) capabilities: NodeRuntimeCapabilities,
    pub(crate) blockers: Vec<NodeRuntimeBlocker>,
    pub(crate) remediation: Vec<NodeRuntimeRemediation>,
    pub(crate) topology: TopologyObservation,
    pub(crate) network_mesh_status: CanonicalNetworkMeshStatusProjection,
    pub(crate) runtime_observation: SharedRuntimeObservation,
    pub(crate) network: NetworkObservation,
    pub(crate) dns: DnsObservation,
    pub(crate) wireguard_transport: WireguardTransportObservation,
    pub(crate) dns_projection: DnsProjectionObservation,
    pub(crate) services: ServiceObservation,
}

<<<<<<< HEAD
=======
#[derive(Debug, Clone, Serialize)]
pub(crate) struct RegistryNodeObservation {
    pub(crate) schema: &'static str,
    pub(crate) node_id: String,
    pub(crate) hostname: String,
    pub(crate) observed_at_unix_ms: u64,
    pub(crate) registry_path: String,
    pub(crate) role: Option<String>,
    pub(crate) evidence: serde_json::Value,
}

>>>>>>> github/main
#[cfg(any(test, kani))]
#[derive(Debug, Clone)]
pub(crate) struct SyntheticNodeObservationContext {
    pub(crate) wg_quick_unit: String,
    pub(crate) resolved_unit: String,
    pub(crate) ip_bin: String,
<<<<<<< HEAD
    pub(crate) network_manager_bin: String,
    pub(crate) nft_bin: String,
=======
>>>>>>> github/main
    pub(crate) wg_bin: String,
    pub(crate) systemctl_bin: String,
}

#[cfg(any(test, kani))]
impl SyntheticNodeObservationContext {
    pub(crate) fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let host_capabilities = runtime_layout.capabilities();
        Self {
            wg_quick_unit: runtime_layout.wg_quick_unit(),
            resolved_unit: host_capabilities.dns_provider.resolved_unit.clone(),
            ip_bin: host_capabilities.route_tool.command.clone(),
<<<<<<< HEAD
            network_manager_bin: host_capabilities.network_manager.command.clone(),
            nft_bin: std::env::var("CYNIC_NFT_BIN").unwrap_or_else(|_| "nft".to_string()),
=======
>>>>>>> github/main
            wg_bin: host_capabilities.wireguard_tool.command.clone(),
            systemctl_bin: host_capabilities.service_manager.command.clone(),
        }
    }
}

impl NodeObservation {
<<<<<<< HEAD
    pub(crate) fn registry_observation(
        &self,
        registry_path: &str,
    ) -> cynic_build::RuntimeNodeObservation {
        let observed_at_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
        let resolution = cynic_host::registry::resolve_host_profile_resolution(
            &self.hostname,
            Some(registry_path),
        );
        let (status, node_id, declared_role, identity_match) = match resolution {
            Ok(resolution) => (
                "observed".to_string(),
                resolution
                    .matched_node
                    .unwrap_or_else(|| self.hostname.clone()),
                resolution.matched_role,
                cynic_protocol::SecurityEvidenceState::Proven,
            ),
            Err(_) => (
                "blocked".to_string(),
                self.hostname.clone(),
                None,
                cynic_protocol::SecurityEvidenceState::Unknown,
            ),
        };
        cynic_build::RuntimeNodeObservation {
            schema: BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
            status,
            node_id,
            hostname: self.hostname.clone(),
            observed_at_unix_ms,
            registry_path: registry_path.to_string(),
            declared_role,
            identity_match,
=======
    pub(crate) fn registry_observation(&self, registry_path: &str) -> RegistryNodeObservation {
        let machine_id_path = std::env::var("CYNIC_MACHINE_ID_PATH")
            .unwrap_or_else(|_| "/etc/machine-id".to_string());
        let machine_id = std::fs::read_to_string(machine_id_path)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("hostname:{}", self.hostname));
        let observed_at_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
        RegistryNodeObservation {
            schema: "cynic.runtime.node_observation.v1",
            node_id: machine_id,
            hostname: self.hostname.clone(),
            observed_at_unix_ms,
            registry_path: registry_path.to_string(),
            role: None,
>>>>>>> github/main
            evidence: serde_json::json!({
                "node_status": self.status,
                "node_severity": self.severity,
                "capabilities": self.capabilities,
                "binary_attestation": self.binary_attestation,
                "network": self.network,
                "network_surfaces": crate::network_surface_inventory::inventory_network_surfaces(
                    &self.network.interfaces.stdout,
                    &self.network.routes.stdout,
                ),
                "route_classifications": crate::network_surface_inventory::classify_route_surfaces(
                    &self.network.routes.stdout,
                ),
                "topology": self.topology,
            }),
<<<<<<< HEAD
            evidence_refs: vec![format!(
                "node-local-observation:{}:{}",
                self.hostname, observed_at_unix_ms
            )],
=======
>>>>>>> github/main
        }
    }

    pub(crate) fn network_actuation_observation(
        &self,
        mesh_interface: impl Into<String>,
    ) -> cynic_host::network_actuation::NetworkActuationObservation {
        cynic_host::network_actuation::NetworkActuationObservation {
            provider_fallback_active: self.capabilities.provider_fallback_active,
            synapse_route_active: self.capabilities.synapse_route_active,
            wireguard_active: self.capabilities.wireguard_active,
            management_link_active: false,
            target: cynic_host::network_actuation::NetworkActuationTarget {
                provider_fallback_cidr: self.topology.provider_fallback_cidr.clone(),
                provider_fallback_gateway: self.topology.provider_fallback_gateway.clone(),
                synapse_cidr: self.topology.synapse_cidr.clone(),
                mesh_interface: mesh_interface.into(),
                management_interface: None,
                management_connection: None,
                management_address: None,
                management_gateway: None,
            },
        }
    }

    pub(crate) fn is_warning_or_worse(&self) -> bool {
        self.severity != NodeRuntimeSeverity::Clear
    }

    pub(crate) fn is_critical(&self) -> bool {
        self.severity == NodeRuntimeSeverity::Critical
    }

    pub(crate) fn runtime_artifact_declaration_drift(&self) -> RuntimeArtifactDeclarationDrift {
        self.binary_attestation.declaration_drift()
    }

    #[cfg(any(test, kani))]
    pub(crate) fn synthetic_for_tests(
        capabilities: NodeRuntimeCapabilities,
        provider_fallback_gateway: Option<String>,
    ) -> Self {
        Self::synthetic_for_tests_with_context(
            capabilities,
            provider_fallback_gateway,
            &SyntheticNodeObservationContext::from_current_runtime(),
        )
    }

    #[cfg(any(test, kani))]
    pub(crate) fn synthetic_for_tests_with_context(
        capabilities: NodeRuntimeCapabilities,
        provider_fallback_gateway: Option<String>,
        context: &SyntheticNodeObservationContext,
    ) -> Self {
        let verdict = classify_runtime(capabilities.clone());
        let network_mesh_status =
            canonical_network_mesh_status_projection(&verdict.status, capabilities.critical_dns_ok);
        let wireguard_transport = WireguardTransportObservation {
            state: WireguardTransportState::MeshAbsent,
            checks: WireguardTransportChecks {
                wg_show_has_output: capabilities.wireguard_active,
                wg_quick_active: capabilities.wireguard_active,
                synapse_route_present: capabilities.synapse_route_active,
                provider_fallback_still_required: capabilities.provider_fallback_active,
                interface_present: capabilities.wireguard_active,
                peer_present: capabilities.wireguard_active,
                handshake_present: capabilities.wireguard_active,
            },
            drift_classes: vec!["mesh_transport_drift", "provider_fallback_dependency"],
        };
        let dns_projection = DnsProjectionObservation {
            state: DnsProjectionState::Active,
            checks: DnsProjectionChecks {
                critical_hosts_resolve: capabilities.critical_dns_ok,
                systemd_resolved_active: true,
                provider_fallback_dependency: capabilities.provider_fallback_active,
                resolved_matches_expected_synapse: capabilities.critical_dns_canonical,
            },
            drift_classes: if capabilities.provider_fallback_active {
                vec!["provider_fallback_dependency"]
            } else {
                Vec::new()
            },
        };
        let runtime_observation =
            shared_runtime_observation(&verdict, &wireguard_transport, &dns_projection);
        Self {
            schema: NODE_OBSERVATION_SCHEMA,
            generated_at: "1970-01-01T00:00:00+00:00".to_string(),
            hostname: "test-node".to_string(),
            binary_attestation: runtime_binary_attestation(Some("test-registry.json")),
            status: verdict.status,
            severity: verdict.severity,
            capabilities: verdict.capabilities,
            blockers: verdict.blockers,
            remediation: verdict.remediation,
            topology: TopologyObservation {
                source: "test".to_string(),
                registry_path: Some("test-registry.json".to_string()),
                canonical_service_plane: "synapse".to_string(),
                synapse_cidr: "10.20.0.0/24".to_string(),
                provider_fallback_cidr: "10.10.10.0/24".to_string(),
                provider_fallback_gateway,
                dns_zone: "synapse.cynic".to_string(),
            },
            network_mesh_status,
            runtime_observation,
            network: NetworkObservation {
                interfaces: command_observation_for_tests(&context.ip_bin, &["-brief", "addr"]),
                routes: command_observation_for_tests(&context.ip_bin, &["route"]),
<<<<<<< HEAD
                network_manager_connections: command_observation_for_tests(
                    &context.network_manager_bin,
                    &[
                        "-t",
                        "-f",
                        "NAME,UUID,TYPE,DEVICE",
                        "connection",
                        "show",
                        "--active",
                    ],
                ),
                policy_routes: command_observation_for_tests(&context.ip_bin, &["rule"]),
                namespaces: command_observation_for_tests(&context.ip_bin, &["netns", "list"]),
                firewall: command_observation_for_tests(&context.nft_bin, &["list", "ruleset"]),
=======
>>>>>>> github/main
                provider_fallback_route: command_observation_for_tests(
                    &context.ip_bin,
                    &["route", "show", "10.10.10.0/24"],
                ),
                synapse_route: command_observation_for_tests(
                    &context.ip_bin,
                    &["route", "show", "10.20.0.0/24"],
                ),
                wireguard: command_observation_for_tests(&context.wg_bin, &["show"]),
            },
            dns: DnsObservation {
                critical_hosts: Vec::new(),
            },
            wireguard_transport,
            dns_projection,
            services: ServiceObservation {
                systemd: SystemdObservation {
                    wg_quick_wg0: command_observation_for_tests(
                        &context.systemctl_bin,
                        &["is-active", &context.wg_quick_unit],
                    ),
                    network_manager: command_observation_for_tests(
                        &context.systemctl_bin,
                        &["is-active", "NetworkManager"],
                    ),
                    systemd_resolved: command_observation_for_tests(
                        &context.systemctl_bin,
                        &["is-active", &context.resolved_unit],
                    ),
                },
                tcp: Vec::new(),
            },
        }
    }
}

#[cfg(any(test, kani))]
fn command_observation_for_tests(command: &str, args: &[&str]) -> CommandObservation {
    CommandObservation {
        command: command.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
    }
}

pub(crate) fn observe_payload(registry_path: Option<&str>) -> NodeObservation {
    crate::usecases::observe_node_uc::build_node_observation(registry_path)
}

#[cfg(test)]
mod tests {
    use super::{NodeObservation, SyntheticNodeObservationContext};
    use crate::domain::node_observation::NodeRuntimeCapabilities;

    #[test]
    fn synthetic_observation_uses_injected_command_context() {
        let observation = NodeObservation::synthetic_for_tests_with_context(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
            &SyntheticNodeObservationContext {
                wg_quick_unit: "wg-quick@wg-portable".to_string(),
                resolved_unit: "portable-resolved".to_string(),
                ip_bin: "/portable/bin/ip".to_string(),
<<<<<<< HEAD
                network_manager_bin: "/portable/bin/nmcli".to_string(),
                nft_bin: "/portable/bin/nft".to_string(),
=======
>>>>>>> github/main
                wg_bin: "/portable/bin/wg".to_string(),
                systemctl_bin: "/portable/bin/systemctl".to_string(),
            },
        );

        assert_eq!(observation.network.interfaces.command, "/portable/bin/ip");
        assert_eq!(observation.network.wireguard.command, "/portable/bin/wg");
        assert_eq!(
            observation.services.systemd.wg_quick_wg0.command,
            "/portable/bin/systemctl"
        );
        assert_eq!(
            observation.services.systemd.wg_quick_wg0.args,
            vec!["is-active".to_string(), "wg-quick@wg-portable".to_string()]
        );
        assert_eq!(
            observation.services.systemd.systemd_resolved.args,
            vec!["is-active".to_string(), "portable-resolved".to_string()]
        );
    }
}
