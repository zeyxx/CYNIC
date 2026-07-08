use chrono::Utc;

use crate::adapters::host_observation::{
    command_json, command_reports_active, command_succeeded_with_stdout, resolve_host, tcp_open,
    tcp_probe,
};
use crate::adapters::runtime_observation::LocalMeshRuntimeObserver;
use crate::build_attestation::runtime_binary_attestation;
use crate::domain::node_observation::{NodeRuntimeCapabilities, classify_runtime};
use crate::domain::observation_plan::build_observation_plan;
use crate::domain::observation_runtime::{
    dns_projection_surface, dns_resolution_observation, hostname,
};
use crate::domain::network_projection::{
    canonical_network_mesh_status_projection, shared_runtime_observation,
    wireguard_transport_observation,
};
use crate::observe::{
    DnsObservation, NetworkObservation, NodeObservation, ServiceObservation, SystemdObservation,
    TopologyObservation,
};
use crate::ports::network_interfaces_port::MeshRuntimeObservationPort;

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
    let peer_present = wireguard_stdout.lines().any(|line| line.starts_with("peer:"));
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

pub(crate) fn build_node_observation(registry_path: Option<&str>) -> NodeObservation {
    let plan = build_observation_plan(registry_path);

    let interfaces = command_json("ip", &["-brief", "addr"]);
    let routes = command_json("ip", &["route"]);
    let provider_fallback_route = command_json("ip", &["route", "show", &plan.provider_fallback_cidr]);
    let synapse_route = command_json("ip", &["route", "show", &plan.synapse_cidr]);
    let wireguard = command_json("wg", &["show"]);
    let mesh_runtime_observer = LocalMeshRuntimeObserver::new("wg0", &plan.synapse_cidr, "10.20.0.1");
    let mesh_runtime = mesh_runtime_observer
        .observe_runtime()
        .unwrap_or(crate::ports::network_interfaces_port::MeshRuntimeObservation {
            schema: "cynic.network_mesh.runtime_observation.v1",
            status: "blocked".to_string(),
            blockers: vec!["mesh_runtime_observer_failed".to_string()],
            interface_present: false,
            wg_quick_active: false,
            route_uses_interface: false,
            wg_show_has_output: false,
        });
    let dns_checks = plan
        .critical_hosts
        .iter()
        .map(|host| {
            dns_resolution_observation(
                host,
                plan.expected_synapse_ips.get(host).cloned(),
                resolve_host(host),
            )
        })
        .collect::<Vec<_>>();
    let tcp_results = plan
        .tcp_checks
        .iter()
        .map(|check| tcp_probe(&check.host, check.port))
        .collect::<Vec<_>>();
    let wg_quick_wg0 = command_json("systemctl", &["is-active", "wg-quick@wg0"]);
    let network_manager = command_json("systemctl", &["is-active", "NetworkManager"]);
    let systemd_resolved = command_json("systemctl", &["is-active", "systemd-resolved"]);

    let provider_fallback_active = command_succeeded_with_stdout(&provider_fallback_route);
    let synapse_route_active =
        mesh_runtime.route_uses_interface || command_succeeded_with_stdout(&synapse_route);
    let wireguard_active =
        mesh_runtime.wg_show_has_output || command_succeeded_with_stdout(&wireguard);
    let dns_ok = dns_checks
        .iter()
        .all(|item| command_succeeded_with_stdout(&item.command));
    let critical_tcp_open = tcp_results.iter().all(tcp_open);

    let wireguard_transport = wireguard_transport_surface(
        synapse_route_active,
        provider_fallback_active,
        wireguard_active,
        command_reports_active(&wg_quick_wg0),
        &wireguard.stdout,
    );
    let dns_projection =
        dns_projection_surface(&dns_checks, provider_fallback_active, &systemd_resolved);
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
        hostname: hostname(),
        binary_attestation: runtime_binary_attestation(registry_path),
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
            interfaces,
            routes,
            provider_fallback_route,
            synapse_route,
            wireguard,
        },
        dns: DnsObservation {
            critical_hosts: dns_checks,
        },
        wireguard_transport,
        dns_projection,
        services: ServiceObservation {
            systemd: SystemdObservation {
                wg_quick_wg0,
                network_manager,
                systemd_resolved,
            },
            tcp: tcp_results,
        },
    }
}
