use std::process::ExitCode;

use clap::Subcommand;
use cynic_types::paths::CynicPaths;
pub mod execution;
pub use execution::NetworkCommands;

use crate::registry_path;
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

mod actuator;
mod base;
mod dns;
mod domain;
mod mesh;
mod provider_fallback;

/// The service probe specs declared in `infra/registry.json`.
///
/// Extracted so `network watchdog` and `network notify` evaluate the SAME
/// declared services. Two copies of this walk would be two places for the
/// registry to be read differently, which is the drift this repository keeps
/// paying for.
pub(crate) fn declared_service_probe_specs()
-> Vec<cynic_node::usecases::network_watchdog_uc::ServiceProbeSpec> {
    use cynic_node::usecases::network_watchdog_uc::ServiceProbeSpec;

    let reg_path = crate::registry_path(None);
    let mut services = Vec::new();
    let Ok(content) = std::fs::read_to_string(&reg_path) else {
        return services;
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) else {
        return services;
    };
    let canonical_plane = val
        .pointer("/network_policy/canonical_service_plane")
        .and_then(|v| v.as_str())
        .unwrap_or("synapse");
    let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) else {
        return services;
    };
    for node in nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let ip = node
            .get("network")
            .and_then(|n| n.get(canonical_plane).or_else(|| n.get("synapse")))
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1");
        let Some(svcs) = node.get("services").and_then(|s| s.as_array()) else {
            continue;
        };
        for svc in svcs {
            let svc_name = svc.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let port = svc.get("port").and_then(|p| p.as_u64()).unwrap_or(0);
            if port > 0 {
                services.push(ServiceProbeSpec {
                    service: svc_name.to_string(),
                    node: node_id.to_string(),
                    endpoint: format!("{ip}:{port}"),
                });
            }
        }
    }
    services
}

#[derive(Subcommand)]
pub(crate) enum NetworkCommand {
    Chain {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        check_provider_connectivity: bool,
    },
    Runtime {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        check_provider_connectivity: bool,
    },
    Proof {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        check_provider_connectivity: bool,
    },
    /// Shadow proof: drift between declared mesh and independently observed
    /// state. Fails when drift is detected — the honest counterpart of proof.
    ShadowProof {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        observe_runtime: bool,
    },
    /// Host-owned network actuation observation projection. This is the wire
    /// contract consumed by `mesh target actuate` (observer_bin argv) — the
    /// name is load-bearing, do not rename.
    ActuationObservation {
        #[arg(long)]
        registry: Option<String>,
    },
    Evidence {
        #[arg(long)]
        registry: Option<String>,
    },
    Dns {
        #[command(subcommand)]
        command: NetworkDnsCommand,
    },
    Mesh {
        #[command(subcommand)]
        command: NetworkMeshCommand,
    },
    /// Provider selection, including multi-provider fallback.
    Provider {
        #[command(subcommand)]
        command: NetworkProviderCommand,
    },
    Actuator {
        #[command(subcommand)]
        command: NetworkActuatorCommand,
    },
    /// Audit declared vs observed network state (DNS, WireGuard, services).
    Audit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "infra/registry.json")]
        registry: Option<String>,
    },
    /// Peer connectivity, observation, and consensus status.
    Peer {
        #[command(subcommand)]
        command: cynic_node::peer_commands::PeerCommands,
    },
    /// Host-level transport inspection and SSH configuration supervision.
    Transport {
        #[command(subcommand)]
        command: crate::transport_surface::TransportCommand,
    },
    /// Full 4-layer network observation (L1 physical, L2 addresses/routes, L3 usage, L4 pressure, Freebox router)
    Observe {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        freebox: Option<String>,
    },
    /// Network watchdog assessment with active alerts
    Watchdog {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "wg0")]
        interface: String,
        #[arg(long)]
        freebox: Option<String>,
    },
    /// Evaluate the watchdog and DELIVER its alerts through the configured
    /// transport. `watchdog` only enqueues; this is the wire that sends.
    Notify {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "wg0")]
        interface: String,
        /// Evaluate and enqueue, then stop before delivery.
        #[arg(long)]
        dry_run: bool,
    },
    /// Reconcile and synchronize WireGuard peers and Synapse DNS from registry desired state
    Sync {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long, default_value = "cynic-host-cortex")]
        node_id: String,
        #[arg(long, default_value = "wg0")]
        interface: String,
        #[arg(long)]
        apply: bool,
    },
    /// Network SOC security audit (rogue listening ports, WireGuard handshake decay)
    /// Transport and mesh security posture.
    Security {
        #[command(subcommand)]
        command: NetworkSecurityCommand,
    },
    /// Reconcile discovered cluster network observations into registry.json observed section
    /// Fold observed cluster state back into the registry.
    Reconcile {
        #[command(subcommand)]
        command: NetworkReconcileCommand,
    },
    /// Declarative Zero-Trust nftables firewall policy and audit
    Firewall {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        audit: bool,
    },
    /// External domain registration and DNS management (Porkbun + Cloudflare).
    Domain {
        #[command(subcommand)]
        command: NetworkDomainCommand,
    },
}

pub(crate) use dns::NetworkDnsCommand;
pub(crate) use domain::NetworkDomainCommand;

#[derive(Subcommand)]
pub(crate) enum NetworkMeshCommand {
    Plan {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        provider: Option<String>,
    },
    Bundle {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    /// Mutating mesh intents: plan, target, or a stored profile.
    Apply {
        #[command(subcommand)]
        command: NetworkMeshApplyCommand,
    },
    /// Attest a target before any mutation reaches it.
    Target {
        #[command(subcommand)]
        command: NetworkMeshTargetCommand,
    },
    Status {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        include_records: bool,
        #[arg(long)]
        observe_runtime: bool,
    },
    /// Observe and reconcile mesh coordination members and health.
    Coordination {
        /// Registry path used to derive declared mesh peers.
        #[arg(long)]
        registry: Option<String>,
        /// Optional explicit coordination endpoint URL.
        #[arg(long)]
        endpoint: Option<String>,
        /// Optional API bearer token for coordination control plane.
        #[arg(long)]
        api_key: Option<String>,
        /// Probe coordination health only.
        #[arg(long)]
        check_health: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkMeshApplyCommand {
    /// Materialize the plan.
    Plan {
        #[arg(long)]
        bundle_dir: String,
    },
    /// Apply a plan to an attested target.
    Target {
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
        #[arg(long, default_value = "closed_local_lab")]
        harness_kind: String,
        #[arg(long)]
        target_attestation: String,
        #[arg(long, default_value = "virsh")]
        virsh_bin: String,
        #[arg(long, default_value_t = 10)]
        qga_timeout_seconds: u64,
        #[arg(long, default_value_t = 20)]
        qga_max_status_polls: usize,
    },
    /// Apply a stored profile to a target.
    Profile {
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        closed_environment: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkMeshTargetCommand {
    /// Capture and verify target attestation.
    Attestation {
        #[arg(long, default_value = "closed_local_lab")]
        harness_kind: String,
        #[arg(long)]
        target_id: String,
        #[arg(long, default_value = "qemu-guest-agent")]
        transport: String,
        #[arg(long)]
        transport_target: String,
        #[arg(long)]
        expected_network_boundary: String,
        #[arg(long, default_value = "virsh")]
        virsh_bin: String,
        #[arg(long)]
        ip_bin: Option<String>,
    },
    /// Execute network actuation on an attested target. Requires --apply;
    /// without it the command fails closed.
    Actuate {
        /// JSON file containing plan, admission, and execution authorization.
        #[arg(long)]
        request: String,
        /// Required guard for runtime execution.
        #[arg(long)]
        apply: bool,
        /// Runtime harness kind whose attestation must remain valid.
        #[arg(long, default_value = "closed_local_lab")]
        harness_kind: String,
        /// JSON file containing a direct attestation or target-attestation report.
        #[arg(long)]
        target_attestation: String,
        /// Guest executable used to produce typed postflight observation.
        #[arg(long, default_value = "cynic-node")]
        observer_bin: String,
        /// Injected libvirt QEMU guest-agent binary.
        #[arg(long, default_value = "virsh")]
        virsh_bin: String,
        /// Per-call virsh/QEMU guest-agent timeout.
        #[arg(long, default_value_t = 10)]
        qga_timeout_seconds: u64,
        /// Maximum guest-exec-status observations before failing closed.
        #[arg(long, default_value_t = 20)]
        qga_max_status_polls: usize,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkProviderCommand {
    /// Resilient provider chain.
    Fallback {
        #[command(subcommand)]
        command: NetworkProviderFallbackCommand,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkSecurityCommand {
    /// WireGuard handshake and topology audit.
    Audit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 180)]
        max_handshake_age_secs: u64,
        #[arg(long)]
        registry: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkReconcileCommand {
    /// Reconcile discovered observations into registry.json.
    Observed {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value_t = 500)]
        timeout_ms: u64,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkProviderFallbackCommand {
    Status {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        hosts_path: Option<String>,
        #[arg(long)]
        check_connectivity: bool,
    },
    Hosts {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkActuatorCommand {
    Status {
        #[arg(long)]
        registry: Option<String>,
    },
    Bundle {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    Plan {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value_t = cynic_host::runtime_layout::current_runtime_layout().network_actuator_bundle_dir)]
        bundle_dir: String,
    },
    Apply {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
    },
}

pub(crate) fn execute_network(command: NetworkCommand) -> ExitCode {
    let _network_command_type: &str = std::any::type_name::<NetworkCommands>();
    let _ = _network_command_type;
    let runtime = resolve_runtime_paths(&RuntimeContext::from_env(None));
    base::try_execute_base(command, &runtime)
        .or_else(|cmd| dns::try_execute_dns(cmd, &runtime))
        .or_else(|cmd| mesh::try_execute_mesh(cmd, &runtime))
        .or_else(|cmd| provider_fallback::try_execute_provider_fallback(cmd, &runtime))
        .or_else(|cmd| domain::try_execute_domain(cmd, &runtime))
        .map_or_else(
            |cmd| match actuator::try_execute_actuator(cmd) {
                Ok(code) => return code,
                Err(cmd) => match cmd {
                    NetworkCommand::Security { command } => execute_network_security(command),
                    NetworkCommand::Reconcile { command } => execute_network_reconcile(command),
                    other => unreachable!(
                        "all network commands must be handled, leaked: {}",
                        std::any::type_name_of_val(&other)
                    ),
                },
            },
            |code| code,
        )
}

fn execute_network_security(command: NetworkSecurityCommand) -> ExitCode {
    match command {
        NetworkSecurityCommand::Audit {
            json: _,
            max_handshake_age_secs,
            registry,
        } => {
            crate::network_surface::execution::execute_network_command(&crate::network_surface::execution::NetworkCommands::SecurityAudit {
                json: true,
                max_handshake_age_secs,
                registry: registry_path(registry),
            })
        }
    }
}

fn execute_network_reconcile(command: NetworkReconcileCommand) -> ExitCode {
    match command {
        NetworkReconcileCommand::Observed {
            json: _,
            registry,
            timeout_ms,
            apply,
        } => crate::network_surface::execution::execute_network_command(
            &crate::network_surface::execution::NetworkCommands::ReconcileObserved {
                json: true,
                registry: registry_path(registry),
                timeout_ms,
                apply,
            },
        ),
    }
}
