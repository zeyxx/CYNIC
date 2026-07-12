use clap::{Parser, Subcommand};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
use cynic_host::runtime_layout::current_runtime_layout;

#[derive(Parser)]
#[command(name = "cynic-node", about = "CYNIC inference backend supervisor")]
pub(crate) struct Cli {
    /// Path to node config file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Run L0 node watchdog only; does not require a Dog backend process.
    #[arg(long)]
    pub watchdog: bool,

    /// Also serve MCP proxy on stdio (requires --features mcp)
    #[arg(long)]
    pub mcp: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Emit local node observation JSON and exit.
    Observe {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Return failure when the node is degraded or blocked.
        #[arg(long)]
        fail_on_warning: bool,
        /// Return failure only when the node is blocked.
        #[arg(long)]
        fail_on_critical: bool,
    },
    /// Network diagnostics and guarded remediation plans.
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },
}

#[derive(Subcommand)]
pub enum NetworkCommands {
    /// Emit canonical Synapse DNS activation bundle.
    DnsBundle {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive Synapse topology.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Output directory containing bundle files and manifest.
        #[arg(long)]
        output_dir: String,
    },
    /// Emit canonical Synapse DNS activation plan.
    DnsPlan {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
    },
    /// Emit canonical Synapse DNS activation status.
    DnsActivationStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive optional DNS resolution checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Hosts path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_hosts_path())]
        hosts_path: String,
        /// Corefile path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_corefile_path())]
        corefile_path: String,
        /// Service path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_service_path())]
        service_path: String,
        /// Resolved drop-in path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_resolved_dropin_path())]
        resolved_dropin_path: String,
        /// Also validate DNS resolution semantics.
        #[arg(long)]
        resolve_dns: bool,
    },
    /// Emit network evidence: current observation, dry-run plan, and DevSecOps gaps.
    Evidence {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
    },
    /// Emit the least-privilege root policy required for bounded network mutation.
    Policy {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
    },
    /// Emit local activation status for the bounded network-actuator surface.
    ActuatorStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
    },
    /// Emit the install bundle for the bounded network-actuator surface.
    ActuatorBundle {
        /// Registry path used to derive runtime checks and allowed profiles.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Output directory containing helper, policy, sudoers and manifest.
        #[arg(long)]
        output_dir: String,
    },
    /// Emit a non-mutating activation plan for the bounded network-actuator surface.
    ActuatorPlan {
        /// Registry path used to derive runtime checks and allowed profiles.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Directory expected to contain the actuator bundle.
        #[arg(long, default_value_t = current_runtime_layout().network_actuator_bundle_dir)]
        bundle_dir: String,
    },
    /// Execute one bounded network actuator profile. Intended for the root-owned helper path.
    Apply {
        /// Registry path used to derive runtime checks and concrete commands.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Allowed profile name.
        #[arg(long)]
        profile: String,
        /// Required guard for mutation.
        #[arg(long)]
        apply: bool,
    },
    /// Emit a non-mutating network repair plan.
    Repair {
        /// Required guard: this command only plans; it never mutates the host.
        #[arg(long)]
        dry_run: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
    },
    /// Emit canonical provider fallback status for resurrection-only routing.
    MeshPlan {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive mesh intent.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Override registry mesh provider for planning only.
        #[arg(long)]
        provider: Option<String>,
    },
    /// Emit a non-mutating provider-specific mesh activation bundle.
    MeshBundle {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive mesh intent.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Override registry mesh provider for bundling only.
        #[arg(long)]
        provider: Option<String>,
        /// Output directory containing mesh bundle files.
        #[arg(long)]
        output_dir: String,
    },
    /// Emit a non-mutating mesh apply plan from an existing bundle.
    MeshApplyPlan {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Directory containing manifest.json and mesh-plan.json.
        #[arg(long)]
        bundle_dir: String,
    },
    /// Execute one executable profile from an existing mesh apply-plan projection.
    MeshApplyProfile {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Directory containing manifest.json and mesh-plan.json.
        #[arg(long)]
        bundle_dir: String,
        /// Profile name from actuator_policy.allowed_profiles.
        #[arg(long)]
        profile: String,
        /// Required guard for runtime execution.
        #[arg(long)]
        apply: bool,
        /// Required for profiles that can mutate network state. Use only in an isolated lab/VM.
        #[arg(long)]
        closed_environment: bool,
    },
    /// Emit canonical provider fallback status for resurrection-only routing.
    ProviderFallbackStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive fallback route and service checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Provider fallback hosts file to validate.
        #[arg(long, default_value_t = current_runtime_layout().provider_fallback_hosts_path())]
        hosts_path: String,
        /// Probe critical service sockets through the fallback plane.
        #[arg(long)]
        check_connectivity: bool,
    },
    /// Emit provider fallback hosts evidence bundle.
    ProviderFallbackHosts {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive fallback host entries.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Output directory containing provider-fallback.hosts and manifest context.
        #[arg(long)]
        output_dir: String,
    },
    /// Compare Python and Rust network surfaces side by side and emit drift proof.
    ShadowProof {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used by both Python and Rust paths.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Resolve DNS in the Python mesh status surface.
        #[arg(long)]
        resolve_dns: bool,
        /// Include runtime observation in the Python mesh status surface.
        #[arg(long)]
        observe_runtime: bool,
    },
    /// Emit unified local network runtime status for lifecycle readiness.
    RuntimeStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive runtime checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Also validate DNS resolution semantics.
        #[arg(long)]
        resolve_dns: bool,
        /// Probe critical service sockets through the provider fallback plane.
        #[arg(long)]
        check_provider_connectivity: bool,
        /// Provider fallback hosts file to validate.
        #[arg(long, default_value_t = current_runtime_layout().provider_fallback_hosts_path())]
        provider_hosts_path: String,
        /// Hosts path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_hosts_path())]
        hosts_path: String,
        /// Corefile path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_corefile_path())]
        corefile_path: String,
        /// Service path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_service_path())]
        service_path: String,
        /// Resolved drop-in path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_resolved_dropin_path())]
        resolved_dropin_path: String,
    },
    /// Emit a typed runtime proof for the network_runtime_observed gate.
    RuntimeProof {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive runtime checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Also validate DNS resolution semantics.
        #[arg(long)]
        resolve_dns: bool,
        /// Probe critical service sockets through the provider fallback plane.
        #[arg(long)]
        check_provider_connectivity: bool,
        /// Provider fallback hosts file to validate.
        #[arg(long, default_value_t = current_runtime_layout().provider_fallback_hosts_path())]
        provider_hosts_path: String,
        /// Hosts path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_hosts_path())]
        hosts_path: String,
        /// Corefile path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_corefile_path())]
        corefile_path: String,
        /// Service path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_service_path())]
        service_path: String,
        /// Resolved drop-in path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_resolved_dropin_path())]
        resolved_dropin_path: String,
    },
    /// Emit the canonical machine activation chain for host baseline, DNS, actuator and proof.
    ActivationChain {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive runtime checks.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Also validate DNS resolution semantics.
        #[arg(long)]
        resolve_dns: bool,
        /// Probe critical service sockets through the provider fallback plane.
        #[arg(long)]
        check_provider_connectivity: bool,
        /// Provider fallback hosts file to validate.
        #[arg(long, default_value_t = current_runtime_layout().provider_fallback_hosts_path())]
        provider_hosts_path: String,
        /// Hosts path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_hosts_path())]
        hosts_path: String,
        /// Corefile path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_corefile_path())]
        corefile_path: String,
        /// Service path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_service_path())]
        service_path: String,
        /// Resolved drop-in path to validate.
        #[arg(long, default_value_t = current_runtime_layout().dns_resolved_dropin_path())]
        resolved_dropin_path: String,
    },
}
