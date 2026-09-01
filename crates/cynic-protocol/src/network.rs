use serde::{Deserialize, Serialize};

use crate::task_pipeline::EnergyBudget;

pub const NETWORK_MESH_PLAN_SCHEMA: &str = "cynic.network_mesh_plan.v1";
pub const MESH_ROLE_MAP_SCHEMA: &str = "cynic.mesh_role_map.v1";

/// What a mesh provider is actually responsible for.
///
/// These are separate concerns and one provider does not imply the other:
/// carrying packets is not the same job as deciding who may join. Conflating
/// them is how a coordination provider ends up treated as network truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshRole {
    /// Moves packets between nodes. Owns interfaces, keys, routes.
    Transport,
    /// Decides membership: enrolment, key distribution, revocation, discovery.
    /// Holds no packet path of its own.
    Coordination,
}

/// Where a provider's control plane runs, and therefore who can revoke it.
///
/// This is the distinction that decides whether a provider may be kept at all.
/// `Sovereign` means the control plane runs on hardware the operator owns.
/// `Saas` means a third party can revoke, rate-limit, observe, or price the
/// organism's ability to form its own network — which is disqualifying for a
/// runtime dependency regardless of how convenient the client is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshSovereignty {
    /// Control plane on operator-owned substrate (WireGuard keys, self-hosted
    /// Headscale). Failure is the operator's to repair.
    Sovereign,
    /// Third-party-hosted control plane. Disqualified as a runtime dependency.
    Saas,
    /// No control plane: static declarative configuration only.
    None,
}

/// A provider CYNIC knows about, with its role and sovereignty pinned.
///
/// The CLI advertises `wireguard|headscale|tailscale|static` as a bare string
/// with no type behind it, so nothing prevented a SaaS control plane from being
/// selected as if it were equivalent to a sovereign one. This table is the
/// missing constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshProviderBinding {
    pub provider: String,
    pub role: MeshRole,
    pub sovereignty: MeshSovereignty,
    /// True when this provider is the declared owner of its role today.
    pub canonical_for_role: bool,
    /// Why this provider is admitted, retained pending work, or refused.
    pub rationale: &'static str,
}

/// Every mesh provider CYNIC recognises, and its standing.
///
/// Observed on this fleet 2026-08-20: `wg0` is up on the hypervisor with four
/// live peers; no tailscale interface exists on any node and `tailscaled` is
/// inactive. Headscale is running on LXC 100 and answers on :8080 with zero
/// clients — it is retained for the coordination role, which has no other
/// owner, not for transport.
pub fn mesh_provider_bindings() -> Vec<MeshProviderBinding> {
    vec![
        MeshProviderBinding {
            provider: "wireguard".to_string(),
            role: MeshRole::Transport,
            sovereignty: MeshSovereignty::Sovereign,
            canonical_for_role: true,
            rationale: "keys and interfaces live on operator substrate; no external control plane",
        },
        MeshProviderBinding {
            provider: "headscale".to_string(),
            role: MeshRole::Coordination,
            sovereignty: MeshSovereignty::Sovereign,
            canonical_for_role: true,
            rationale: "self-hosted control plane; owns enrolment and revocation, never the packet path",
        },
        MeshProviderBinding {
            provider: "tailscale".to_string(),
            role: MeshRole::Coordination,
            sovereignty: MeshSovereignty::Saas,
            canonical_for_role: false,
            rationale: "third-party control plane may revoke or observe mesh formation; refused as a runtime dependency",
        },
        MeshProviderBinding {
            provider: "static".to_string(),
            role: MeshRole::Transport,
            sovereignty: MeshSovereignty::None,
            canonical_for_role: false,
            rationale: "declarative fallback with no control plane; no discovery, no revocation",
        },
    ]
}

/// Whether a provider may be a runtime dependency, and why not when it may not.
///
/// Refusal is a typed verdict rather than a filtered-out entry so the reason
/// survives into reports instead of a provider silently disappearing.
pub fn admit_mesh_provider(provider: &str) -> Result<MeshProviderBinding, String> {
    match mesh_provider_bindings()
        .into_iter()
        .find(|b| b.provider == provider)
    {
        None => Err(format!("unknown mesh provider: {provider}")),
        Some(binding) if binding.sovereignty == MeshSovereignty::Saas => Err(format!(
            "{provider} refused: {} control plane is not operator-owned",
            binding.provider
        )),
        Some(binding) => Ok(binding),
    }
}

/// The provider that owns a role today, if any is canonical for it.
pub fn canonical_provider_for_role(role: MeshRole) -> Option<MeshProviderBinding> {
    mesh_provider_bindings()
        .into_iter()
        .find(|b| b.role == role && b.canonical_for_role)
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub provider: String,
    pub registry_path: String,
    pub service_plane: String,
    pub cidr: Option<String>,
    pub dns_zone: Option<String>,
    pub desired_peer_count: usize,
    pub desired_peers: Vec<NetworkMeshPeer>,
    pub routes: Vec<NetworkMeshRoute>,
    pub dns: Vec<NetworkMeshDnsRecord>,
    pub acl: Vec<NetworkMeshAclRule>,
    pub actuator_required: bool,
    pub can_apply: bool,
    pub apply_requires: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshPeer {
    pub id: String,
    pub role: Option<String>,
    pub status: Option<String>,
    pub address: String,
    pub local_address: Option<String>,
    pub dns_names: Vec<String>,
    pub services: Vec<NetworkMeshService>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshService {
    pub name: String,
    pub port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshRoute {
    pub destination: String,
    pub via_peer: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshDnsRecord {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshAclRule {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub action: String,
}

/// Observation result for network substrate, following the 4-layer model
/// (physical / volume / usage / pressure) that disk+memory reuse.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NetworkReport {
    /// Schema identity as an owned string — sibling contracts (`cpu`, `memory`)
    /// own it the same way; `&'static str` + derived `Deserialize` does not
    /// compile (nightly rustc: `'de` must outlive `'static`).
    pub schema: String,
    /// L1 — raw interface hardware/device.
    pub interfaces: Vec<PhysicalInterface>,
    /// L2 — configured addresses and routes (volume of addressable substrate).
    pub addresses: Vec<NetworkAddress>,
    pub routes: Vec<NetworkRoute>,
    /// L3 — throughput counters sampled from /proc/net/dev.
    pub usage: Vec<InterfaceUsage>,
    /// L4 — backpressure signalling (drops, errors, queue length).
    pub pressure: Vec<InterfacePressure>,
    /// Residual unknowns that could not be observed.
    pub unknowns: Vec<String>,
}

pub const NETWORK_REPORT_SCHEMA: &str = "cynic.host_network_report.v1";

impl Default for NetworkReport {
    fn default() -> Self {
        Self {
            schema: NETWORK_REPORT_SCHEMA.to_string(),
            interfaces: Vec::new(),
            addresses: Vec::new(),
            routes: Vec::new(),
            usage: Vec::new(),
            pressure: Vec::new(),
            unknowns: Vec::new(),
        }
    }
}

/// L1 Physical — a NIC as exposed by `ip -j link` / `/sys/class/net/<iface>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PhysicalInterface {
    pub name: String,
    pub mac: Option<String>,
    pub mtu: u32,
    pub state: String,
    /// Operational speed in Mbps (e.g. 1000), when reported by the kernel.
    pub speed_mbps: Option<u32>,
    /// Duplex mode when reported ("full", "half", "unknown").
    pub duplex: Option<String>,
    pub driver: Option<String>,
}

/// L2 Volume — an address bound to an interface (ip -j addr).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NetworkAddress {
    pub iface: String,
    pub family: String,
    pub address: String,
    pub prefix_len: u8,
}

/// L2 Volume — a routing entry (ip -j route).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NetworkRoute {
    pub dst: String,
    pub gateway: Option<String>,
    pub device: String,
    pub protocol: Option<String>,
    pub metric: Option<u32>,
    pub scope: Option<String>,
}

/// L3 Usage — per-interface packet/byte counters, sampled from `/proc/net/dev`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct InterfaceUsage {
    pub iface: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errs: u64,
    pub rx_drop: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errs: u64,
    pub tx_drop: u64,
}

/// L4 Pressure — socket/queue backpressure signalling saturation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct InterfacePressure {
    pub iface: String,
    /// Packet receive drops — socket back-pressure / ring overflow.
    pub rx_drop: u64,
    /// Packet transmit drops.
    pub tx_drop: u64,
    /// Receive errors (CRC, frame, length, etc.).
    pub rx_errs: u64,
    /// Transmit errors (carrier, aborted, etc.).
    pub tx_errs: u64,
    /// Estimated queue length when observable (`ss -s` or rtnl). None = unknown.
    pub queue_length: Option<u32>,
    /// True when the link reports a saturation/backpressure signal.
    pub saturated: bool,
}

/// Fleet rollup for a single node's network observation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FleetNetworkNode {
    pub node_id: String,
    pub role: String,
    pub ip: String,
    /// `observed`, `failed`, or `skipped` (e.g. no ssh target).
    pub outcome: String,
    pub report: Option<NetworkReport>,
    pub error: Option<String>,
}

/// Fleet-wide network rollup (`cynic.fleet_network.v1`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FleetNetworkReport {
    pub schema: String,
    pub corpus_unix_s: i64,
    pub total_nodes: usize,
    pub observed_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<FleetNetworkNode>,
    pub residual_unknowns: Vec<String>,
    pub budget: Option<EnergyBudget>,
    #[serde(default)]
    pub poll_cost_ms: Option<u64>,
}

pub const FLEET_NETWORK_SCHEMA: &str = "cynic.fleet_network.v1";

pub const FREEBOX_LAN_OBSERVATION_SCHEMA: &str = "cynic.network.freebox_lan_observation.v1";
pub const NETWORK_WATCHDOG_ASSESSMENT_SCHEMA: &str = "cynic.network.watchdog_assessment.v1";
pub const FLEET_WATCHDOG_REPORT_SCHEMA: &str = "cynic.fleet.watchdog_report.v1";

/// Host detected on the local LAN by the Freebox router.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeboxHostEntry {
    pub id: String,
    pub primary_name: String,
    pub ip: Option<String>,
    pub mac: Option<String>,
    pub host_type: Option<String>,
    pub active: bool,
    pub reachable: bool,
    pub interface: Option<String>,
}

/// Observation of the local Freebox OS router (unauthenticated discovery or session).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FreeboxLanObservation {
    pub schema: String,
    pub endpoint: String,
    pub reachable: bool,
    pub auth_state: String,
    pub box_model_name: Option<String>,
    pub device_name: Option<String>,
    pub api_version: Option<String>,
    pub api_domain: Option<String>,
    pub https_port: Option<u16>,
    pub https_available: bool,
    pub uid: Option<String>,
    pub wan_status: Option<String>,
    pub wan_ipv4: Option<String>,
    pub wan_ipv6: Option<String>,
    pub rate_down_bps: Option<u64>,
    pub rate_up_bps: Option<u64>,
    pub bytes_down_total: Option<u64>,
    pub bytes_up_total: Option<u64>,
    pub hosts: Vec<FreeboxHostEntry>,
    pub observed_at_unix_ms: u64,
    pub unknowns: Vec<String>,
}

/// Alert level emitted by the network watchdog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAlertLevel {
    Ok,
    Warning,
    Critical,
}

/// Discrete alert emitted when a network condition breaches threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAlert {
    pub level: NetworkAlertLevel,
    pub component: String,
    pub message: String,
    pub recovery_hint: String,
}

/// Assessment of WireGuard mesh peer status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireguardMeshAssessment {
    pub total_peers: usize,
    pub ok_peers: usize,
    pub stale_peers: usize,
    pub dead_peers: usize,
    pub never_seen_peers: usize,
}

/// Assessment of Synapse DNS resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsSynapseAssessment {
    pub zone: String,
    pub total_records: usize,
    pub resolved_records: usize,
    pub failed_records: usize,
    pub failures: Vec<String>,
}

/// Assessment of one critical service TCP port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicePortAssessment {
    pub service: String,
    pub node: String,
    pub endpoint: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
}

/// Complete network watchdog assessment across all 4 layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkWatchdogAssessment {
    pub schema: String,
    pub status: String,
    pub level: NetworkAlertLevel,
    pub wireguard_mesh: WireguardMeshAssessment,
    pub dns_synapse: DnsSynapseAssessment,
    pub service_ports: Vec<ServicePortAssessment>,
    pub pressure: Vec<InterfacePressure>,
    pub freebox_lan: Option<FreeboxLanObservation>,
    pub alerts: Vec<NetworkAlert>,
    pub observed_at_unix_ms: u64,
}

/// Comprehensive fleet-wide network watchdog report aggregating multi-node observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetWatchdogReport {
    pub schema: String,
    pub status: String,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub failed_nodes: usize,
    pub fleet_network: FleetNetworkReport,
    pub wireguard_mesh: WireguardMeshAssessment,
    pub dns_synapse: DnsSynapseAssessment,
    pub service_ports: Vec<ServicePortAssessment>,
    pub freebox_lan: Option<FreeboxLanObservation>,
    pub alerts: Vec<NetworkAlert>,
    pub observed_at_unix_ms: u64,
}

/// Pure evaluation function compiling raw observations into a watchdog assessment with alerts.
pub fn evaluate_network_watchdog(
    wireguard: WireguardMeshAssessment,
    dns: DnsSynapseAssessment,
    services: Vec<ServicePortAssessment>,
    pressure: Vec<InterfacePressure>,
    freebox: Option<FreeboxLanObservation>,
    now_unix_ms: u64,
) -> NetworkWatchdogAssessment {
    let mut alerts = Vec::new();
    let mut max_level = NetworkAlertLevel::Ok;

    // 1. Check WireGuard mesh peers
    if wireguard.dead_peers > 0 {
        alerts.push(NetworkAlert {
            level: NetworkAlertLevel::Critical,
            component: "wireguard_mesh".to_string(),
            message: format!(
                "{} WireGuard peer(s) are dead (handshake > 200s)",
                wireguard.dead_peers
            ),
            recovery_hint:
                "Check remote WireGuard service status and endpoints via management plane"
                    .to_string(),
        });
        max_level = NetworkAlertLevel::Critical;
    } else if wireguard.stale_peers > 0 {
        alerts.push(NetworkAlert {
            level: NetworkAlertLevel::Warning,
            component: "wireguard_mesh".to_string(),
            message: format!(
                "{} WireGuard peer(s) are stale (handshake > 150s)",
                wireguard.stale_peers
            ),
            recovery_hint: "Verify route keepalive packets on synapse plane".to_string(),
        });
        if max_level == NetworkAlertLevel::Ok {
            max_level = NetworkAlertLevel::Warning;
        }
    }

    // 2. Check Synapse DNS
    if dns.failed_records > 0 {
        alerts.push(NetworkAlert {
            level: NetworkAlertLevel::Critical,
            component: "dns_synapse".to_string(),
            message: format!(
                "DNS resolution failed for {} record(s) in zone '{}'",
                dns.failed_records, dns.zone
            ),
            recovery_hint: "Inspect CoreDNS/dnsmasq on pve (10.10.10.1) or fallback hosts file"
                .to_string(),
        });
        max_level = NetworkAlertLevel::Critical;
    }

    // 3. Check critical services
    for svc in &services {
        if !svc.reachable {
            alerts.push(NetworkAlert {
                level: NetworkAlertLevel::Critical,
                component: format!("service:{}", svc.service),
                message: format!(
                    "Service '{}' on node '{}' ({}) is unreachable",
                    svc.service, svc.node, svc.endpoint
                ),
                recovery_hint: format!("Restart unit or inspect container for node '{}'", svc.node),
            });
            max_level = NetworkAlertLevel::Critical;
        }
    }

    // 4. Check interface pressure / drops
    for p in &pressure {
        if p.saturated || p.rx_drop > 1000 || p.tx_drop > 1000 {
            alerts.push(NetworkAlert {
                level: NetworkAlertLevel::Warning,
                component: format!("interface:{}", p.iface),
                message: format!(
                    "Interface '{}' reports backpressure or packet drops (rx_drop={}, tx_drop={})",
                    p.iface, p.rx_drop, p.tx_drop
                ),
                recovery_hint: "Inspect ring buffer size and socket queues with 'ss -s'"
                    .to_string(),
            });
            if max_level == NetworkAlertLevel::Ok {
                max_level = NetworkAlertLevel::Warning;
            }
        }
    }

    let status = match max_level {
        NetworkAlertLevel::Ok => "ok",
        NetworkAlertLevel::Warning => "warning",
        NetworkAlertLevel::Critical => "critical",
    }
    .to_string();

    NetworkWatchdogAssessment {
        schema: NETWORK_WATCHDOG_ASSESSMENT_SCHEMA.to_string(),
        status,
        level: max_level,
        wireguard_mesh: wireguard,
        dns_synapse: dns,
        service_ports: services,
        pressure,
        freebox_lan: freebox,
        alerts,
        observed_at_unix_ms: now_unix_ms,
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_mesh_plan_schema_is_stable() {
        let plan = NetworkMeshPlan {
            schema: NETWORK_MESH_PLAN_SCHEMA,
            status: "planned",
            provider: "wireguard".to_string(),
            registry_path: "registry.json".to_string(),
            service_plane: "synapse".to_string(),
            cidr: Some("10.0.0.0/24".to_string()),
            dns_zone: Some("mesh.test".to_string()),
            desired_peer_count: 0,
            desired_peers: Vec::new(),
            routes: Vec::new(),
            dns: Vec::new(),
            acl: Vec::new(),
            actuator_required: true,
            can_apply: false,
            apply_requires: vec!["network_actuator_bundle"],
        };
        assert_eq!(plan.schema, "cynic.network_mesh_plan.v1");
        assert!(!plan.can_apply);
    }

    #[test]
    fn tailscale_is_refused_as_a_runtime_dependency() {
        let err = admit_mesh_provider("tailscale").expect_err("SaaS must not be admitted");
        assert!(err.contains("not operator-owned"), "{err}");
    }

    #[test]
    fn sovereign_providers_are_admitted() {
        for provider in ["wireguard", "headscale"] {
            let binding = admit_mesh_provider(provider)
                .unwrap_or_else(|e| panic!("{provider} should be admitted: {e}"));
            assert_eq!(binding.sovereignty, MeshSovereignty::Sovereign);
        }
    }

    /// Headscale is self-hosted, so it survives the SaaS filter — but it must
    /// never be admitted as transport. That role belongs to WireGuard, which
    /// owns the only live interface on this fleet.
    #[test]
    fn headscale_coordinates_and_does_not_transport() {
        let headscale = admit_mesh_provider("headscale").expect("self-hosted");
        assert_eq!(headscale.role, MeshRole::Coordination);

        let transport =
            canonical_provider_for_role(MeshRole::Transport).expect("transport must have an owner");
        assert_eq!(transport.provider, "wireguard");
    }

    /// Each role has exactly one canonical owner: two owners is an unresolved
    /// authority conflict, zero is an unowned capability.
    #[test]
    fn every_role_has_exactly_one_canonical_owner() {
        for role in [MeshRole::Transport, MeshRole::Coordination] {
            let owners: Vec<_> = mesh_provider_bindings()
                .into_iter()
                .filter(|b| b.role == role && b.canonical_for_role)
                .collect();
            assert_eq!(owners.len(), 1, "role {role:?} has {} owners", owners.len());
        }
    }

    /// A canonical owner that is not sovereign would mean the organism depends on
    /// a third party to form its own network.
    #[test]
    fn no_canonical_owner_is_saas() {
        for binding in mesh_provider_bindings() {
            if binding.canonical_for_role {
                assert_ne!(
                    binding.sovereignty,
                    MeshSovereignty::Saas,
                    "{} is canonical but SaaS",
                    binding.provider
                );
            }
        }
    }

    #[test]
    fn an_unknown_provider_is_named_not_silently_dropped() {
        let err = admit_mesh_provider("zerotier").expect_err("unknown");
        assert!(err.contains("zerotier"), "{err}");
    }
    #[test]
    fn watchdog_all_healthy_emits_ok_status() {
        let assessment = evaluate_network_watchdog(
            WireguardMeshAssessment {
                total_peers: 4,
                ok_peers: 4,
                stale_peers: 0,
                dead_peers: 0,
                never_seen_peers: 0,
            },
            DnsSynapseAssessment {
                zone: "synapse.cynic".to_string(),
                total_records: 6,
                resolved_records: 6,
                failed_records: 0,
                failures: vec![],
            },
            vec![ServicePortAssessment {
                service: "forgejo".to_string(),
                node: "forgejo".to_string(),
                endpoint: "10.10.10.101:3000".to_string(),
                reachable: true,
                latency_ms: Some(2),
            }],
            vec![],
            None,
            1788267750000,
        );

        assert_eq!(assessment.status, "ok");
        assert_eq!(assessment.level, NetworkAlertLevel::Ok);
        assert!(assessment.alerts.is_empty());
    }

    #[test]
    fn watchdog_dead_peer_emits_critical_alert() {
        let assessment = evaluate_network_watchdog(
            WireguardMeshAssessment {
                total_peers: 4,
                ok_peers: 3,
                stale_peers: 0,
                dead_peers: 1,
                never_seen_peers: 0,
            },
            DnsSynapseAssessment {
                zone: "synapse.cynic".to_string(),
                total_records: 6,
                resolved_records: 6,
                failed_records: 0,
                failures: vec![],
            },
            vec![],
            vec![],
            None,
            1788267750000,
        );

        assert_eq!(assessment.status, "critical");
        assert_eq!(assessment.level, NetworkAlertLevel::Critical);
        assert_eq!(assessment.alerts.len(), 1);
        assert_eq!(assessment.alerts[0].component, "wireguard_mesh");
    }

    #[test]
    fn watchdog_service_unreachable_emits_critical_alert() {
        let assessment = evaluate_network_watchdog(
            WireguardMeshAssessment {
                total_peers: 4,
                ok_peers: 4,
                stale_peers: 0,
                dead_peers: 0,
                never_seen_peers: 0,
            },
            DnsSynapseAssessment {
                zone: "synapse.cynic".to_string(),
                total_records: 6,
                resolved_records: 6,
                failed_records: 0,
                failures: vec![],
            },
            vec![ServicePortAssessment {
                service: "surrealdb".to_string(),
                node: "cynic-lxc-surrealdb".to_string(),
                endpoint: "10.10.10.103:8000".to_string(),
                reachable: false,
                latency_ms: None,
            }],
            vec![],
            None,
            1788267750000,
        );

        assert_eq!(assessment.status, "critical");
        assert_eq!(assessment.alerts[0].component, "service:surrealdb");
    }
}
