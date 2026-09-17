use std::process::ExitCode;

use clap::Subcommand;
use cynic_node::NetworkCommands;

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

mod actuator;
mod base;
mod dns;
mod mesh;
mod provider_fallback;

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
    ProviderFallback {
        #[command(subcommand)]
        command: NetworkProviderFallbackCommand,
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
        command: cynic_node::cli::PeerCommands,
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
        #[arg(long, default_value = "http://192.168.0.254")]
        freebox: Option<String>,
    },
    /// Network watchdog assessment with active alerts
    Watchdog {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "wg0")]
        interface: String,
        #[arg(long, default_value = "http://192.168.0.254")]
        freebox: Option<String>,
    },
    /// Reconcile and synchronize WireGuard peers and Synapse DNS from registry desired state
    Sync {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "cynic-host-cortex")]
        node_id: String,
        #[arg(long, default_value = "wg0")]
        interface: String,
        #[arg(long)]
        apply: bool,
    },
    /// Network SOC security audit (rogue listening ports, WireGuard handshake decay)
    SecurityAudit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 180)]
        max_handshake_age_secs: u64,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Reconcile discovered cluster network observations into registry.json observed section
    ReconcileObserved {
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
pub(crate) enum NetworkDnsCommand {
    Status {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        hosts_path: Option<String>,
        #[arg(long)]
        corefile_path: Option<String>,
        #[arg(long)]
        service_path: Option<String>,
        #[arg(long)]
        resolved_dropin_path: Option<String>,
    },
    Bundle {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    Plan {
        #[arg(long)]
        json: bool,
    },
    Activate {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        yes: bool,
    },
}

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
    ApplyPlan {
        #[arg(long)]
        bundle_dir: String,
    },
    TargetAttestation {
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
    ApplyTarget {
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
    ApplyProfile {
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        closed_environment: bool,
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
        .unwrap_or_else(|cmd| match actuator::try_execute_actuator(cmd) {
            Ok(code) => code,
            Err(_) => unreachable!("all network commands must be handled"),
        })
}
