use chrono::Utc;
use cynic_host::runtime_layout::current_runtime_layout;

use crate::adapters::host_observation::{
    command_json, command_reports_active, command_succeeded_with_stdout, resolve_host, tcp_open,
    tcp_probe,
};
use crate::adapters::runtime_observation::LocalMeshRuntimeObserver;
use crate::build_attestation::runtime_binary_attestation;
use crate::domain::network_projection::{
    canonical_network_mesh_status_projection, shared_runtime_observation,
    wireguard_transport_observation,
};
use crate::domain::node_observation::{NodeRuntimeCapabilities, classify_runtime};
use crate::domain::observation_plan::build_observation_plan;
use crate::domain::observation_runtime::{
    dns_projection_surface, dns_resolution_observation, hostname,
};
use crate::observe::{
    DnsObservation, NetworkObservation, NodeObservation, ServiceObservation, SystemdObservation,
    TopologyObservation,
};
use crate::ports::network_interfaces_port::{
    MeshRuntimeObservationPort, NodeRuntimeObservationPort, ProviderServiceCheck,
};

#[derive(Debug, Clone)]
struct NodeObservationRuntimeInputs {
    interfaces: crate::adapters::host_observation::CommandObservation,
    routes: crate::adapters::host_observation::CommandObservation,
    provider_fallback_route: crate::adapters::host_observation::CommandObservation,
    synapse_route: crate::adapters::host_observation::CommandObservation,
    wireguard: crate::adapters::host_observation::CommandObservation,
    mesh_runtime: crate::ports::network_interfaces_port::MeshRuntimeObservation,
    dns_checks: Vec<crate::domain::observation_runtime::DnsResolutionObservation>,
    tcp_results: Vec<crate::adapters::host_observation::TcpObservation>,
    wg_quick_wg0: crate::adapters::host_observation::CommandObservation,
    network_manager: crate::adapters::host_observation::CommandObservation,
    systemd_resolved: crate::adapters::host_observation::CommandObservation,
    hostname: String,
    binary_attestation: crate::build_attestation::BinaryAttestation,
}

struct LocalNodeRuntimeObservationPort {
    ip_bin: String,
    wg_bin: String,
    systemctl_bin: String,
    mesh_interface: String,
    wg_quick_unit: String,
    resolved_unit: String,
}

impl NodeRuntimeObservationPort for LocalNodeRuntimeObservationPort {
    fn observe_interfaces(&self) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.ip_bin, &["-brief", "addr"])
    }

    fn observe_routes(&self) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.ip_bin, &["route"])
    }

    fn observe_provider_fallback_route(
        &self,
        cidr: &str,
    ) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.ip_bin, &["route", "show", cidr])
    }

    fn observe_synapse_route(
        &self,
        cidr: &str,
    ) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.ip_bin, &["route", "show", cidr])
    }

    fn observe_wireguard(&self) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.wg_bin, &["show"])
    }

    fn observe_mesh_runtime(
        &self,
        synapse_cidr: &str,
        synapse_probe_ip: &str,
    ) -> crate::ports::network_interfaces_port::MeshRuntimeObservation {
        let mesh_runtime_observer =
            LocalMeshRuntimeObserver::new(&self.mesh_interface, synapse_cidr, synapse_probe_ip);
        mesh_runtime_observer.observe_runtime().unwrap_or(
            crate::ports::network_interfaces_port::MeshRuntimeObservation {
                schema: "cynic.network_mesh.runtime_observation.v1",
                status: "blocked".to_string(),
                blockers: vec!["mesh_runtime_observer_failed".to_string()],
                interface_present: false,
                wg_quick_active: false,
                route_uses_interface: false,
                wg_show_has_output: false,
            },
        )
    }

    fn observe_dns_checks(
        &self,
        critical_hosts: &[String],
        expected_synapse_ips: &std::collections::BTreeMap<String, String>,
    ) -> Vec<crate::domain::observation_runtime::DnsResolutionObservation> {
        critical_hosts
            .iter()
            .map(|host| {
                dns_resolution_observation(
                    host,
                    expected_synapse_ips.get(host).cloned(),
                    resolve_host(host),
                )
            })
            .collect()
    }

    fn observe_tcp_checks(
        &self,
        checks: &[ProviderServiceCheck],
    ) -> Vec<crate::adapters::host_observation::TcpObservation> {
        checks
            .iter()
            .map(|check| tcp_probe(&check.host, check.port))
            .collect()
    }

    fn observe_wg_quick_status(&self) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.systemctl_bin, &["is-active", &self.wg_quick_unit])
    }

    fn observe_network_manager_status(
        &self,
    ) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.systemctl_bin, &["is-active", "NetworkManager"])
    }

    fn observe_systemd_resolved_status(
        &self,
    ) -> crate::adapters::host_observation::CommandObservation {
        command_json(&self.systemctl_bin, &["is-active", &self.resolved_unit])
    }

    fn observe_hostname(&self) -> String {
        hostname()
    }

    fn observe_binary_attestation(
        &self,
        registry_path: Option<&str>,
    ) -> crate::build_attestation::BinaryAttestation {
        runtime_binary_attestation(registry_path)
    }
}

fn wireguard_transport_surface(
    synapse_route_active: bool,
    provider_fallback_active: bool,
    wireguard_active: bool,
    wg_quick_active: bool,
    wireguard_stdout: &str,
) -> crate::domain::network_projection::WireguardTransportObservation {
    let interface_present = wireguard_stdout
        .lines()
        .any(|line| line.starts_with("interface:"));
    let peer_present = wireguard_stdout
        .lines()
        .any(|line| line.starts_with("peer:"));
    let handshake_present = wireguard_stdout
        .lines()
        .any(|line| line.trim_start().starts_with("latest handshake:") && !line.contains("Never"));
    wireguard_transport_observation(
        synapse_route_active,
        provider_fallback_active,
        wireguard_active,
        wg_quick_active,
        interface_present,
        peer_present,
        handshake_present,
    )
}

fn first_usable_ipv4_in_cidr(cidr: &str) -> Option<String> {
    let (network, prefix) = cidr.split_once('/')?;
    let prefix = prefix.parse::<u32>().ok()?;
    if prefix > 32 {
        return None;
    }
    let network = parse_ipv4(network)?;
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let first = (network & mask).saturating_add(1);
    Some(format!(
        "{}.{}.{}.{}",
        (first >> 24) & 0xff,
        (first >> 16) & 0xff,
        (first >> 8) & 0xff,
        first & 0xff
    ))
}

fn parse_ipv4(value: &str) -> Option<u32> {
    let mut output = 0u32;
    let mut parts = 0usize;
    for part in value.split('.') {
        let octet = part.parse::<u8>().ok()?;
        output = (output << 8) | u32::from(octet);
        parts += 1;
    }
    (parts == 4).then_some(output)
}

pub(crate) fn build_node_observation(registry_path: Option<&str>) -> NodeObservation {
    let plan = build_observation_plan(registry_path);
    let runtime_layout = current_runtime_layout();
    let capabilities = runtime_layout.capabilities();
    build_node_observation_with_port(
        registry_path,
        plan,
        &LocalNodeRuntimeObservationPort {
            ip_bin: capabilities.route_tool.command.clone(),
            wg_bin: capabilities.wireguard_tool.command.clone(),
            systemctl_bin: capabilities.service_manager.command.clone(),
            mesh_interface: capabilities.wireguard_tool.mesh_interface.clone(),
            wg_quick_unit: runtime_layout.wg_quick_unit(),
            resolved_unit: capabilities.dns_provider.resolved_unit.clone(),
        },
    )
}

fn build_node_observation_with_port(
    registry_path: Option<&str>,
    plan: crate::domain::observation_plan::ObservationPlan,
    port: &impl NodeRuntimeObservationPort,
) -> NodeObservation {
    let interfaces = port.observe_interfaces();
    let routes = port.observe_routes();
    let provider_fallback_route =
        port.observe_provider_fallback_route(&plan.provider_fallback_cidr);
    let synapse_route = port.observe_synapse_route(&plan.synapse_cidr);
    let wireguard = port.observe_wireguard();
    let synapse_probe_ip = first_usable_ipv4_in_cidr(&plan.synapse_cidr)
        .unwrap_or_else(|| plan.provider_fallback_gateway.clone().unwrap_or_default());
    let mesh_runtime = port.observe_mesh_runtime(&plan.synapse_cidr, &synapse_probe_ip);
    let dns_checks = port.observe_dns_checks(&plan.critical_hosts, &plan.expected_synapse_ips);
    let tcp_checks = plan
        .tcp_checks
        .iter()
        .map(|check| ProviderServiceCheck {
            node: "unknown".to_string(),
            service: "unknown".to_string(),
            host: check.host.clone(),
            ip: String::new(),
            port: check.port,
        })
        .collect::<Vec<_>>();
    let tcp_results = port.observe_tcp_checks(&tcp_checks);
    let wg_quick_wg0 = port.observe_wg_quick_status();
    let network_manager = port.observe_network_manager_status();
    let systemd_resolved = port.observe_systemd_resolved_status();
    let hostname = port.observe_hostname();
    let binary_attestation = port.observe_binary_attestation(registry_path);

    build_node_observation_with_runtime(
        plan,
        NodeObservationRuntimeInputs {
            interfaces,
            routes,
            provider_fallback_route,
            synapse_route,
            wireguard,
            mesh_runtime,
            dns_checks,
            tcp_results,
            wg_quick_wg0,
            network_manager,
            systemd_resolved,
            hostname,
            binary_attestation,
        },
    )
}

fn build_node_observation_with_runtime(
    plan: crate::domain::observation_plan::ObservationPlan,
    inputs: NodeObservationRuntimeInputs,
) -> NodeObservation {
    let provider_fallback_active = command_succeeded_with_stdout(&inputs.provider_fallback_route);
    let synapse_route_active = inputs.mesh_runtime.route_uses_interface
        || command_succeeded_with_stdout(&inputs.synapse_route);
    let wireguard_active =
        inputs.mesh_runtime.wg_show_has_output || command_succeeded_with_stdout(&inputs.wireguard);
    let dns_ok = inputs
        .dns_checks
        .iter()
        .all(|item| command_succeeded_with_stdout(&item.command));
    let critical_tcp_open = inputs.tcp_results.iter().all(tcp_open);

    let wireguard_transport = wireguard_transport_surface(
        synapse_route_active,
        provider_fallback_active,
        wireguard_active,
        command_reports_active(&inputs.wg_quick_wg0),
        &inputs.wireguard.stdout,
    );
    let dns_projection = dns_projection_surface(
        &inputs.dns_checks,
        provider_fallback_active,
        &inputs.systemd_resolved,
    );
    let verdict = classify_runtime(NodeRuntimeCapabilities {
        provider_fallback_active,
        synapse_route_active,
        wireguard_active,
        critical_dns_ok: dns_ok,
        critical_dns_canonical: dns_projection.checks.resolved_matches_expected_synapse,
        critical_tcp_open,
    });
    let network_mesh_status = canonical_network_mesh_status_projection(&verdict.status, dns_ok);
    let runtime_observation =
        shared_runtime_observation(&verdict, &wireguard_transport, &dns_projection);

    NodeObservation {
        schema: crate::domain::node_observation::NODE_OBSERVATION_SCHEMA,
        generated_at: Utc::now().to_rfc3339(),
        hostname: inputs.hostname,
        binary_attestation: inputs.binary_attestation,
        status: verdict.status,
        severity: verdict.severity,
        capabilities: verdict.capabilities,
        blockers: verdict.blockers,
        remediation: verdict.remediation,
        topology: TopologyObservation {
            source: plan.source,
            registry_path: plan.registry_path,
            canonical_service_plane: plan.canonical_service_plane,
            synapse_cidr: plan.synapse_cidr,
            provider_fallback_cidr: plan.provider_fallback_cidr,
            provider_fallback_gateway: plan.provider_fallback_gateway,
            dns_zone: plan.dns_zone,
        },
        network_mesh_status,
        runtime_observation,
        network: NetworkObservation {
            interfaces: inputs.interfaces,
            routes: inputs.routes,
            provider_fallback_route: inputs.provider_fallback_route,
            synapse_route: inputs.synapse_route,
            wireguard: inputs.wireguard,
        },
        dns: DnsObservation {
            critical_hosts: inputs.dns_checks,
        },
        wireguard_transport,
        dns_projection,
        services: ServiceObservation {
            systemd: SystemdObservation {
                wg_quick_wg0: inputs.wg_quick_wg0,
                network_manager: inputs.network_manager,
                systemd_resolved: inputs.systemd_resolved,
            },
            tcp: inputs.tcp_results,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{NodeObservationRuntimeInputs, build_node_observation_with_runtime};
    use crate::adapters::host_observation::{CommandObservation, TcpObservation};
    use crate::build_attestation::runtime_binary_attestation;
    use crate::domain::node_observation::{NodeRuntimeSeverity, NodeRuntimeStatus};
    use crate::domain::observation_plan::build_observation_plan;
    use crate::domain::observation_runtime::dns_resolution_observation;
    use crate::observe::NodeObservation;
    use crate::ports::network_interfaces_port::MeshRuntimeObservation;

    fn command_ok(command: &str, args: &[&str], stdout: &str) -> CommandObservation {
        CommandObservation {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn command_fail(command: &str, args: &[&str]) -> CommandObservation {
        CommandObservation {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "failed".to_string(),
        }
    }

    #[test]
    fn pure_builder_produces_expected_clear_mesh_state_without_host_runtime() {
        let plan = build_observation_plan(None);
        let dns_check = dns_resolution_observation(
            "forge.synapse.cynic",
            Some("10.20.0.10".to_string()),
            command_ok(
                "getent",
                &["hosts", "forge.synapse.cynic"],
                "10.20.0.10 forge.synapse.cynic",
            ),
        );
        let observation = build_node_observation_with_runtime(
            plan,
            NodeObservationRuntimeInputs {
                interfaces: command_ok("ip", &["-brief", "addr"], "wg0 UP 10.20.0.2/24"),
                routes: command_ok("ip", &["route"], "10.20.0.0/24 dev wg0"),
                provider_fallback_route: command_fail("ip", &["route", "show", "10.10.10.0/24"]),
                synapse_route: command_ok(
                    "ip",
                    &["route", "show", "10.20.0.0/24"],
                    "10.20.0.0/24 dev wg0",
                ),
                wireguard: command_ok(
                    "wg",
                    &["show"],
                    "interface: wg0\npeer: abc\nlatest handshake: 1 second ago",
                ),
                mesh_runtime: MeshRuntimeObservation {
                    schema: "cynic.network_mesh.runtime_observation.v1",
                    status: "active".to_string(),
                    blockers: Vec::new(),
                    interface_present: true,
                    wg_quick_active: true,
                    route_uses_interface: true,
                    wg_show_has_output: true,
                },
                dns_checks: vec![dns_check],
                tcp_results: vec![TcpObservation {
                    host: "forge.synapse.cynic".to_string(),
                    port: 22,
                    status: "open".to_string(),
                }],
                wg_quick_wg0: command_ok("systemctl", &["is-active", "wg-quick@wg0"], "active"),
                network_manager: command_ok(
                    "systemctl",
                    &["is-active", "NetworkManager"],
                    "active",
                ),
                systemd_resolved: command_ok(
                    "systemctl",
                    &["is-active", "systemd-resolved"],
                    "active",
                ),
                hostname: "test-node".to_string(),
                binary_attestation: runtime_binary_attestation(None),
            },
        );

        assert_node_observation(observation);
    }

    fn assert_node_observation(observation: NodeObservation) {
        assert_eq!(
            observation.schema,
            crate::domain::node_observation::NODE_OBSERVATION_SCHEMA
        );
        assert_eq!(observation.hostname, "test-node");
        assert_eq!(observation.status, NodeRuntimeStatus::MeshActive);
        assert_eq!(observation.severity, NodeRuntimeSeverity::Clear);
        assert!(observation.capabilities.synapse_route_active);
        assert!(observation.capabilities.wireguard_active);
        assert!(observation.capabilities.critical_dns_ok);
        assert!(observation.capabilities.critical_tcp_open);
    }
}
