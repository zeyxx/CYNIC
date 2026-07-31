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
    /// Emit the node observation envelope consumed by registry observation projection.
    RegistryObservation {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        #[arg(long)]
        output: Option<String>,
    },
    /// Network diagnostics and guarded remediation plans.
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },
    /// Reconcile versioned runtime artifacts on this machine.
    Release {
        #[command(subcommand)]
        command: ReleaseCommands,
    },
    /// Plan or apply one lease-fenced source change in a sovereign agent worktree.
    AgentWorkspace {
        /// Path to a JSON cynic.agent_source_change_transaction_request file.
        #[arg(long)]
        request: String,
        /// Result content for create/replace. Omit for delete.
        #[arg(long)]
        result_file: Option<String>,
        /// Runtime registry containing the node-local portable workspace layout.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Injected node-local CYNIC state root.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Kernel REST authority used for claims; Bearer comes from CYNIC_KERNEL_API_KEY.
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        /// Node policy upper bound for one workspace claim.
        #[arg(long, default_value_t = 3_600_000)]
        max_claim_ttl_ms: u64,
        /// Node policy upper bound for completed workspace retention.
        #[arg(long, default_value_t = 604_800_000)]
        max_retention_ms: u64,
        /// Required mutation consent.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON. This command always emits JSON.
        #[arg(long)]
        json: bool,
    },
    /// Observe, plan, or collect one completed sovereign agent workspace.
    AgentWorkspaceLifecycle {
        /// Workload whose append-once completion lifecycle record is consumed.
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        /// Agent identity authorized to observe repository claims.
        #[arg(long)]
        agent_id: String,
        #[arg(long, default_value_t = 300_000)]
        renewal_window_ms: u64,
        #[arg(long, default_value_t = 3_600_000)]
        renewal_ttl_ms: u64,
        /// Execute only an already-admitted Collect plan.
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile and durably journal one scheduler step without executing it.
    AgentSchedulerStep {
        /// cynic.agent_scheduler_step_observation.v1 JSON input.
        #[arg(long)]
        observation: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
    /// Observe or reconcile one node-local storage service with a bounded restart budget.
    StorageReconcile {
        #[arg(long, default_value = "/var/lib/cynic/storage-lifecycle")]
        state_dir: String,
        #[arg(long, default_value_t = 30)]
        warmup_seconds: u64,
        #[arg(long, default_value_t = 300)]
        restart_window_seconds: u64,
        #[arg(long, default_value_t = 1)]
        max_restarts_per_window: u32,
        /// Required consent for a bounded service restart.
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Compose a non-mutating source-to-build-runner admission plan.
    SourceAdmissionPlan {
        /// cynic.source.release_request.v1 JSON.
        #[arg(long)]
        request: String,
        /// cynic.source.release_observation.v1 JSON.
        #[arg(long)]
        observation: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan or apply one digest-addressed source materialization on a build runner.
    SourceMaterialize {
        /// cynic.source.materialization_request.v1 JSON.
        #[arg(long)]
        request: String,
        /// Existing principal-owned node state root.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Node-local content-addressed snapshot store.
        #[arg(long, default_value = "/var/lib/cynic/source/snapshots")]
        snapshot_root: String,
        /// Optional HTTPS base URL for digest-derived `.bundle` acquisition.
        #[arg(long)]
        snapshot_base_url: Option<String>,
        /// Registry-observed identity expected from hostname read-back.
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Observe, plan or materialize one registry-declared source repository.
    SourceBootstrap {
        /// Registry node that owns the source workspace layout.
        #[arg(long)]
        node: String,
        /// Repository identifier declared in the node source workspace layout.
        #[arg(long)]
        repository: String,
        /// Explicit Git source without embedded credentials.
        #[arg(long)]
        source: String,
        /// Exact commit to check out and read back.
        #[arg(long)]
        revision: String,
        /// Runtime registry containing the portable source workspace layout.
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Existing principal-owned CYNIC state root.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Agent principal bound to the kernel Bearer token.
        #[arg(long)]
        agent_id: String,
        /// Kernel REST authority for lease verification and proof commit.
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        /// Required mutation consent.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON. This command always emits JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ReleaseCommands {
    /// Plan or apply one desired artifact release with observation and rollback proof.
    Reconcile {
        /// Path to a cynic.runtime_artifact_release.spec.v1 JSON file.
        #[arg(long)]
        spec: String,
        /// Update lifecycle state controlling the allowed harness phase.
        #[arg(long)]
        lifecycle_state: Option<String>,
        /// Required guard for staging, activation, restart, observation and proof writes.
        #[arg(long)]
        yes: bool,
        /// Emit machine-readable JSON. The node release surface always emits JSON.
        #[arg(long)]
        json: bool,
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
    /// Emit the host-owned network actuation observation projection.
    ActuationObservation {
        #[arg(long)]
        json: bool,
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
    /// Observe and attest a libvirt guest before any isolated mesh mutation.
    MeshTargetAttestation {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Runtime harness kind whose isolation contract must be proven.
        #[arg(long, default_value = "closed_local_lab")]
        harness_kind: String,
        /// Declared guest/domain identity.
        #[arg(long)]
        target_id: String,
        /// Guest command transport proven by this adapter.
        #[arg(long, default_value = "qemu-guest-agent")]
        transport: String,
        /// Provider-specific transport locator; QEMU guest-agent requires the domain identity.
        #[arg(long)]
        transport_target: String,
        /// Libvirt network expected to contain the target without forwarding.
        #[arg(long)]
        expected_network_boundary: String,
        /// Injected libvirt observation binary.
        #[arg(long, default_value = "virsh")]
        virsh_bin: String,
        /// Injected host route observation binary.
        #[arg(long, default_value_t = current_runtime_layout().capabilities().route_tool.command)]
        ip_bin: String,
    },
    /// Execute a proved route transaction inside an attested libvirt guest through QEMU guest-agent.
    MeshApplyTarget {
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
        /// Runtime harness kind whose attestation must remain valid.
        #[arg(long, default_value = "closed_local_lab")]
        harness_kind: String,
        /// JSON file containing a direct attestation or target-attestation report.
        #[arg(long)]
        target_attestation: String,
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
    /// Execute a host-owned network-actuation request inside an attested QEMU guest.
    ActuateTarget {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Commands, NetworkCommands};

    #[test]
    fn parses_explicit_target_actuation_surface() {
        let cli = Cli::try_parse_from([
            "cynic-node",
            "network",
            "actuate-target",
            "--json",
            "--request",
            "/proof/request.json",
            "--target-attestation",
            "/proof/attestation.json",
            "--apply",
        ])
        .expect("target actuation command");

        let Some(Commands::Network {
            command:
                NetworkCommands::ActuateTarget {
                    request,
                    target_attestation,
                    apply,
                    ..
                },
        }) = cli.command
        else {
            panic!("expected network actuate-target")
        };
        assert_eq!(request, "/proof/request.json");
        assert_eq!(target_attestation, "/proof/attestation.json");
        assert!(apply);
    }
}
