use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cynic-node", about = "CYNIC inference backend supervisor")]
pub(crate) struct Cli {
    /// Path to node config file
    #[arg(short, long)]
    pub(crate) config: Option<String>,

    /// Run L0 node watchdog only; does not require a Dog backend process.
    #[arg(long)]
    pub(crate) watchdog: bool,

    /// Also serve MCP proxy on stdio (requires --features mcp)
    #[arg(long)]
    pub(crate) mcp: bool,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Emit local node observation JSON and exit.
    Observe {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = "infra/registry.json")]
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
pub(crate) enum NetworkCommands {
    /// Emit canonical Synapse DNS activation bundle.
    DnsBundle {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive Synapse topology.
        #[arg(long, default_value = "infra/registry.json")]
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
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Hosts path to validate.
        #[arg(long, default_value = "/etc/coredns/synapse.hosts")]
        hosts_path: String,
        /// Corefile path to validate.
        #[arg(long, default_value = "/etc/coredns/Corefile")]
        corefile_path: String,
        /// Service path to validate.
        #[arg(long, default_value = "/etc/systemd/system/coredns-synapse.service")]
        service_path: String,
        /// Resolved drop-in path to validate.
        #[arg(long, default_value = "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf")]
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
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
    },
    /// Emit the least-privilege root policy required for bounded network mutation.
    Policy {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
    },
    /// Emit local activation status for the bounded network-actuator surface.
    ActuatorStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
    },
    /// Emit a non-mutating network repair plan.
    Repair {
        /// Required guard: this command only plans; it never mutates the host.
        #[arg(long)]
        dry_run: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
    },
    /// Emit canonical provider fallback status for resurrection-only routing.
    ProviderFallbackStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive fallback route and service checks.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Provider fallback hosts file to validate.
        #[arg(long, default_value = "/etc/cynic/provider-fallback.hosts")]
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
        #[arg(long, default_value = "infra/registry.json")]
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
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Resolve DNS in the Python mesh status surface.
        #[arg(long)]
        resolve_dns: bool,
        /// Include runtime observation in the Python mesh status surface.
        #[arg(long)]
        observe_runtime: bool,
    },
}
