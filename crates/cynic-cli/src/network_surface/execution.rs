use std::path::Path;

use clap::Subcommand;
use cynic_host::runtime_layout::current_runtime_layout;
use cynic_types::paths::CynicPaths;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Subcommand)]
pub enum NetworkCommands {
    /// Network SOC security audit (rogue listening ports, WireGuard handshake decay).
    SecurityAudit {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Maximum allowed WireGuard handshake age in seconds before warning.
        #[arg(long, default_value_t = 180)]
        max_handshake_age_secs: u64,
        /// Registry path used to derive allowed services and topology.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
    },
    /// Reconcile discovered cluster network observations into registry.json observed section.
    ReconcileObserved {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive desired state and write observed status.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        /// TCP probe timeout in milliseconds.
        #[arg(long, default_value_t = 500)]
        timeout_ms: u64,
        /// Atomically write updated observed state to the registry file.
        #[arg(long)]
        apply: bool,
    },
    /// Derive declarative Zero-Trust nftables firewall policy and audit against host.
    FirewallPolicy {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive firewall policy.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        /// Target node ID to compile policy for (defaults to local node).
        #[arg(long)]
        node_id: Option<String>,
        /// Audit live host nftables ruleset against compiled policy.
        #[arg(long)]
        audit: bool,
    },
    /// Emit canonical Synapse DNS activation bundle.
    DnsBundle {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive Synapse topology.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
    /// Plan or apply the exact digest-bound Synapse DNS bundle on this node.
    DnsActivate {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long)]
        bundle_dir: String,
        /// Required before any privileged file or service mutation.
        #[arg(long)]
        yes: bool,
    },
    /// Emit network evidence: current observation, dry-run plan, and DevSecOps gaps.
    Evidence {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
    },
    /// Emit the host-owned network actuation observation projection.
    ActuationObservation {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
    },
    /// Emit local activation status for the bounded network-actuator surface.
    ActuatorStatus {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive network checks.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
    },
    /// Emit the install bundle for the bounded network-actuator surface.
    ActuatorBundle {
        /// Registry path used to derive runtime checks and allowed profiles.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        /// Output directory containing helper, policy, sudoers and manifest.
        #[arg(long)]
        output_dir: String,
    },
    /// Emit a non-mutating activation plan for the bounded network-actuator surface.
    ActuatorPlan {
        /// Registry path used to derive runtime checks and allowed profiles.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        /// Directory expected to contain the actuator bundle.
        #[arg(long, default_value_t = current_runtime_layout().network_actuator_bundle_dir)]
        bundle_dir: String,
    },
    /// Execute one bounded network actuator profile. Intended for the root-owned helper path.
    Apply {
        /// Registry path used to derive runtime checks and concrete commands.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        /// Allowed profile name.
        #[arg(long)]
        profile: String,
        /// Required guard for mutation.
        #[arg(long)]
        apply: bool,
    },
    /// Emit canonical provider fallback status for resurrection-only routing.
    MeshPlan {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive mesh intent.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
    /// Observe or reconcile mesh coordination (membership and health).
    MeshCoordination {
        /// Emit JSON. Present for explicit operator intent; currently always JSON.
        #[arg(long)]
        json: bool,
        /// Registry path used to derive declared mesh peers.
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        /// Optional explicit coordination control plane endpoint (e.g. http://10.10.10.40:8080).
        #[arg(long)]
        endpoint: Option<String>,
        /// Optional API bearer token for coordination control plane.
        #[arg(long)]
        api_key: Option<String>,
        /// Probe health only.
        #[arg(long)]
        check_health: bool,
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
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

use cynic_node::adapters::libvirt_harness_target_attestor::LibvirtHarnessTargetAttestor;
use cynic_node::adapters::local_harness_target_attestation_reader::LocalHarnessTargetAttestationReader;
use cynic_node::adapters::local_network_actuation_request_reader::read_network_actuation_request;
use cynic_node::adapters::qemu_guest_agent_network_runner::QemuGuestAgentNetworkRunner;
use cynic_node::dns_activation_status;
use cynic_node::dns_bundle;
use cynic_node::dns_plan;
use cynic_node::domain::network_execution::build_harness_target_attestation_report;
use cynic_node::network_activation_chain;
use cynic_node::network_actuator;
use cynic_node::network_actuator_status;
use cynic_node::network_evidence;
use cynic_node::network_mesh_apply_plan;
use cynic_node::network_mesh_bundle;
use cynic_node::network_mesh_plan;
use cynic_node::network_runtime_status;
use cynic_node::network_shadow_proof;
use cynic_node::observe;
use cynic_node::ports::harness_target_attestation_port::{
    HarnessTargetAttestationPort, HarnessTargetAttestationRequest,
};
use cynic_node::ports::harness_target_attestation_reader_port::HarnessTargetAttestationReaderPort;
use cynic_node::provider_fallback_hosts;
use cynic_node::provider_fallback_status;
use cynic_node::usecases::network_actuation_target_uc::{
    QemuNetworkActuationConfig, execute_qemu_network_actuation,
};
use cynic_host::runtime_harness::runtime_harness_catalog;
use cynic_lifecycle::LocalArgvRunner;

fn record_node_telemetry_heartbeat(observation: &observe::NodeObservation) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let telemetry = cynic_traces::tabular::node_telemetry_record(
        &observation.hostname,
        &observation.hostname,
        None,
        &observation.status.as_str(),
        &observation.severity.as_str(),
        observation.capabilities.provider_fallback_active,
        observation.capabilities.synapse_route_active,
        observation.capabilities.wireguard_active,
        observation.capabilities.critical_dns_ok,
        observation.capabilities.critical_tcp_open,
        observation.blockers.len(),
        now_ms,
    );

    let path = std::path::Path::new("evidence/node_telemetry.jsonl");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(&telemetry) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{line}");
        }
    }

    let evidence_root = std::path::Path::new("evidence");
    if let Ok(store) = cynic_storage::evidence_journal_file::EvidenceJournalStore::open(
        evidence_root,
        &observation.hostname,
    ) {
        if let Ok(record_bytes) = serde_json::to_vec(&telemetry) {
            let digest = cynic_protocol::digest::digest_bytes(&record_bytes);
            let now_s = (now_ms / 1000) as i64;
            let _ = store.append("node.telemetry", &digest, now_s);
        }
    }
}

pub fn execute_observe(
    registry: &str,
    fail_on_warning: bool,
    fail_on_critical: bool,
    record: bool,
) -> ExitCode {
    let observation = observe::observe_payload(Some(registry));
    if record {
        record_node_telemetry_heartbeat(&observation);
    }
    let should_fail = (fail_on_warning && observation.is_warning_or_worse())
        || (fail_on_critical && observation.is_critical());
    emit_json("observe", &observation, should_fail)
}

pub fn execute_registry_observation(registry: &str, output: Option<&str>) -> ExitCode {
    let observation = observe::observe_payload(Some(registry));
    let envelope = observation.registry_observation(registry);
    if let Some(output) = output {
        let bytes = match serde_json::to_vec_pretty(&envelope) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::error!("registry observation serialization failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(error) = std::fs::write(output, bytes) {
            tracing::error!("registry observation write failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    emit_json("registry observation", &envelope, false)
}

pub fn execute_network_command(command: &NetworkCommands) -> ExitCode {
    match command {
        NetworkCommands::SecurityAudit {
            json: _,
            max_handshake_age_secs,
            registry,
        } => {
            let hostname = cynic_host::runtime_layout::observed_hostname();
            let node_id =
                cynic_host::registry::resolve_host_profile_resolution(&hostname, Some(registry))
                    .ok()
                    .and_then(|r| r.matched_node)
                    .unwrap_or(hostname);
            let adapter = cynic_host::adapters::local_network::LocalNetworkAdapter::new();
            let snapshot = adapter.observe_live_telemetry(&node_id, Some("wg0"));

            let registry_json = std::fs::read_to_string(registry)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            let allowed_ports = registry_json
                .as_ref()
                .map(|reg| cynic_network::extract_allowed_ports_from_registry(reg, Some(&node_id)))
                .unwrap_or_else(|| vec![22, 3030, 44196, 51820, 9090]);

            let alerts = cynic_intel::soc::analyze_network_telemetry(
                &snapshot,
                &allowed_ports,
                max_handshake_age_secs * 1000,
            );
            let report = serde_json::json!({
                "schema": "cynic.intel.soc_security_report.v1",
                "status": if alerts.is_empty() { "clean" } else { "threats_detected" },
                "node_id": snapshot.node_id,
                "listening_ports_observed": snapshot.listening_ports,
                "allowed_ports_policy": allowed_ports,
                "wireguard_peers_count": snapshot.wireguard_peers.len(),
                "dropped_packets_count": snapshot.dropped_packets_count,
                "alerts_count": alerts.len(),
                "alerts": alerts,
                "observed_at_unix_ms": snapshot.observed_at_unix_ms,
            });
            emit_json("network security audit", &report, false)
        }
        NetworkCommands::ReconcileObserved {
            json: _,
            registry,
            timeout_ms,
            apply,
        } => {
            let registry_path = registry;
            let content = match std::fs::read_to_string(registry_path) {
                Ok(c) => c,
                Err(error) => {
                    tracing::error!("failed to read registry at {registry_path}: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let mut registry_val: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(error) => {
                    tracing::error!("failed to parse registry JSON: {error}");
                    return ExitCode::FAILURE;
                }
            };

            let hostname = cynic_host::runtime_layout::observed_hostname();
            let observer_node_id = cynic_host::registry::resolve_host_profile_resolution(
                &hostname,
                Some(registry_path),
            )
            .ok()
            .and_then(|r| r.matched_node)
            .unwrap_or(hostname);

            let specs = cynic_network::discovery::extract_desired_network_specs(&registry_val);
            let probe = cynic_host::adapters::local_network::LocalNetworkDiscoveryProbeAdapter;
            let now_unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let report = cynic_network::discovery::discover_cluster_nodes(
                &observer_node_id,
                &specs,
                &probe,
                *timeout_ms,
                now_unix_ms,
            );

            let updated_count =
                match cynic_network::registry_maintenance::reconcile_registry_observations(
                    &mut registry_val,
                    &report,
                ) {
                    Ok(cnt) => cnt,
                    Err(error) => {
                        tracing::error!("failed to reconcile registry observations: {error}");
                        return ExitCode::FAILURE;
                    }
                };

            if *apply {
                let serialized = match serde_json::to_string_pretty(&registry_val) {
                    Ok(s) => s,
                    Err(error) => {
                        tracing::error!("failed to serialize updated registry: {error}");
                        return ExitCode::FAILURE;
                    }
                };
                if let Err(error) = std::fs::write(registry_path, serialized) {
                    tracing::error!("failed to write updated registry: {error}");
                    return ExitCode::FAILURE;
                }
            }

            let result = serde_json::json!({
                "schema": "cynic.network.registry_reconciliation_report.v1",
                "status": if report.drift_detected { "drift_detected" } else { "ok" },
                "observer_node_id": observer_node_id,
                "registry_path": registry_path,
                "nodes_evaluated": specs.len(),
                "nodes_updated": updated_count,
                "drift_detected": report.drift_detected,
                "drift_summary": report.drift_summary,
                "applied": *apply,
                "observed_nodes": registry_val.pointer("/observed/nodes"),
                "observed_at_unix_ms": now_unix_ms,
            });

            emit_json("registry reconciliation", &result, false)
        }
        NetworkCommands::FirewallPolicy {
            json: _,
            registry,
            node_id,
            audit,
        } => {
            let registry_path = registry;
            let content = match std::fs::read_to_string(registry_path) {
                Ok(c) => c,
                Err(error) => {
                    tracing::error!("failed to read registry at {registry_path}: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let registry_val: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(error) => {
                    tracing::error!("failed to parse registry JSON: {error}");
                    return ExitCode::FAILURE;
                }
            };

            let target_node = match node_id {
                Some(id) => id.clone(),
                None => {
                    let hostname = cynic_host::runtime_layout::observed_hostname();
                    cynic_host::registry::resolve_host_profile_resolution(
                        &hostname,
                        Some(registry_path),
                    )
                    .ok()
                    .and_then(|r| r.matched_node)
                    .unwrap_or(hostname)
                }
            };

            let now_unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let policy = match cynic_network::firewall_policy::derive_firewall_policy_from_registry(
                &registry_val,
                &target_node,
                now_unix_ms,
            ) {
                Ok(p) => p,
                Err(error) => {
                    tracing::error!("failed to derive firewall policy: {error}");
                    return ExitCode::FAILURE;
                }
            };

            let rendered_ruleset = policy.render_nftables_ruleset();

            let audit_findings = if *audit {
                let adapter = cynic_host::adapters::local_network::LocalNetworkAdapter::new();
                let live_ruleset = adapter.observe_nftables_ruleset().unwrap_or_default();
                Some(cynic_network::firewall_policy::audit_firewall_ruleset(
                    &policy,
                    &live_ruleset,
                ))
            } else {
                None
            };

            let is_compliant = audit_findings
                .as_ref()
                .map(|f| f.is_empty())
                .unwrap_or(true);
            let result = serde_json::json!({
                "schema": "cynic.network.firewall_policy_report.v1",
                "status": if is_compliant { "compliant" } else { "drift_detected" },
                "node_id": policy.node_id,
                "registry_path": registry_path,
                "bindings_count": policy.bindings.len(),
                "bindings": policy.bindings,
                "nftables_ruleset": rendered_ruleset,
                "audit_enabled": *audit,
                "audit_findings": audit_findings,
                "generated_at_unix_ms": now_unix_ms,
            });

            emit_json("firewall policy", &result, false)
        }
        NetworkCommands::DnsBundle {
            json: _,
            registry,
            output_dir,
        } => match dns_bundle::build_dns_bundle(registry, output_dir) {
            Ok(report) => emit_json("dns bundle", &report, false),
            Err(error) => {
                tracing::error!("dns bundle failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::DnsPlan { json: _ } => {
            let report = dns_plan::build_dns_plan();
            emit_json("dns plan", &report, false)
        }
        NetworkCommands::DnsActivationStatus {
            json: _,
            registry,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
            resolve_dns,
        } => match dns_activation_status::build_dns_activation_status(
            registry,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
            *resolve_dns,
        ) {
            Ok(report) => {
                let should_fail = report.status != "active";
                emit_json("dns activation status", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("dns activation status failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::DnsActivate {
            json: _,
            registry,
            bundle_dir,
            yes,
        } => match cynic_node::dns_activation::apply_dns_activation_value(registry, bundle_dir, *yes) {
            Ok(report) => {
                let should_fail = report["status"].as_str() != Some("proved")
                    && report["status"].as_str() != Some("converged");
                emit_json("dns activation apply", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("dns activation apply failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::Evidence { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let evidence = network_evidence::build_evidence(&observation);
            emit_json("network evidence", &evidence, false)
        }
        NetworkCommands::ActuationObservation { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let projection = observation.network_actuation_observation(
                cynic_host::runtime_layout::current_runtime_layout()
                    .capabilities()
                    .wireguard_tool
                    .mesh_interface,
            );
            emit_json("network actuation observation", &projection, false)
        }
        NetworkCommands::ActuatorStatus { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let status = network_actuator_status::activation_status(&observation);
            emit_json("network actuator status", &status, false)
        }
        NetworkCommands::ActuatorBundle {
            registry,
            output_dir,
        } => {
            let observation = observe::observe_payload(Some(registry));
            match network_actuator::write_install_bundle(&observation, output_dir) {
                Ok(bundle) => emit_json("network actuator bundle", &bundle, false),
                Err(error) => {
                    tracing::error!("network actuator bundle failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::ActuatorPlan {
            registry,
            bundle_dir,
        } => {
            let observation = observe::observe_payload(Some(registry));
            let plan = network_actuator::activation_plan(&observation, bundle_dir);
            emit_json("network actuator plan", &plan, false)
        }
        NetworkCommands::Apply {
            registry,
            profile,
            apply,
        } => {
            let observation = observe::observe_payload(Some(registry));
            let report = network_actuator::apply_profile(&observation, profile, *apply);
            let should_fail = report.status != "completed";
            emit_json("network actuator apply", &report, should_fail)
        }
        NetworkCommands::MeshPlan {
            json: _,
            registry,
            provider,
        } => match network_mesh_plan::build_network_mesh_plan(registry, provider.as_deref()) {
            Ok(report) => emit_json("network mesh plan", &report, false),
            Err(error) => {
                tracing::error!("network mesh plan failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::MeshBundle {
            json: _,
            registry,
            provider,
            output_dir,
        } => match network_mesh_bundle::build_network_mesh_bundle(
            registry,
            provider.as_deref(),
            output_dir,
        ) {
            Ok(report) => emit_json("network mesh bundle", &report, false),
            Err(error) => {
                tracing::error!("network mesh bundle failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::MeshApplyPlan {
            json: _,
            bundle_dir,
        } => match network_mesh_apply_plan::build_network_mesh_apply_plan(bundle_dir) {
            Ok(report) => emit_json("network mesh apply plan", &report, false),
            Err(error) => {
                tracing::error!("network mesh apply plan failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::MeshTargetAttestation {
            json: _,
            harness_kind,
            target_id,
            transport,
            transport_target,
            expected_network_boundary,
            virsh_bin,
            ip_bin,
        } => {
            let context = match runtime_harness_catalog(Some(harness_kind)) {
                Ok(mut catalog) => match catalog.contexts.pop() {
                    Some(context) => context,
                    None => {
                        tracing::error!("runtime harness catalog returned no selected context");
                        return ExitCode::FAILURE;
                    }
                },
                Err(error) => {
                    tracing::error!("runtime harness selection failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let attestor = LibvirtHarnessTargetAttestor::new(
                LocalArgvRunner,
                virsh_bin.clone(),
                ip_bin.clone(),
            );
            match attestor.attest_target(
                &context,
                &HarnessTargetAttestationRequest {
                    target_id: target_id.clone(),
                    provider: "qemu-libvirt".to_string(),
                    transport: transport.clone(),
                    transport_target: transport_target.clone(),
                    expected_network_boundary: expected_network_boundary.clone(),
                },
            ) {
                Ok(attestation) => {
                    let report = build_harness_target_attestation_report(&context, attestation);
                    let should_fail = report.status != "attested";
                    emit_json("network mesh target attestation", &report, should_fail)
                }
                Err(error) => {
                    tracing::error!("network mesh target attestation failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::MeshApplyTarget {
            json: _,
            bundle_dir,
            profile,
            apply,
            harness_kind,
            target_attestation,
            virsh_bin,
            qga_timeout_seconds,
            qga_max_status_polls,
        } => {
            let context = match selected_harness_context(harness_kind) {
                Ok(context) => context,
                Err(error) => {
                    tracing::error!("runtime harness selection failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let attestation = match LocalHarnessTargetAttestationReader
                .read_attestation(Path::new(target_attestation))
            {
                Ok(attestation) => attestation,
                Err(error) => {
                    tracing::error!("network mesh target attestation load failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let plan = match network_mesh_apply_plan::build_network_mesh_apply_plan(bundle_dir) {
                Ok(plan) => plan,
                Err(error) => {
                    tracing::error!("network mesh apply plan failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let allowed_argv = network_mesh_apply_plan::target_transaction_argv(&plan, profile);
            let runner = match QemuGuestAgentNetworkRunner::new(
                LocalArgvRunner,
                context.clone(),
                attestation,
                virsh_bin.clone(),
                allowed_argv,
                *qga_timeout_seconds,
                *qga_max_status_polls,
                Duration::from_millis(50),
            ) {
                Ok(runner) => runner,
                Err(error) => {
                    tracing::error!("network mesh target transport refused: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let report = network_mesh_apply_plan::apply_network_mesh_target_transaction_with_runner(
                bundle_dir, profile, *apply, &context, &runner, plan,
            );
            let should_fail = report.status != "completed";
            emit_json("network mesh apply target", &report, should_fail)
        }
        NetworkCommands::MeshCoordination {
            json: _,
            registry,
            endpoint,
            api_key,
            check_health,
        } => {
            let ep = endpoint
                .clone()
                .unwrap_or_else(|| {
                    use cynic_node::adapters::local_network_mesh_registry::LocalNetworkMeshRegistryReader;
                    use cynic_node::ports::network_mesh_registry_port::NetworkMeshRegistryPort;
                    LocalNetworkMeshRegistryReader
                        .load_mesh_registry(Path::new(registry))
                        .ok()
                        .and_then(|reg| {
                            reg.nodes.into_iter().find(|n| {
                                n.role.as_deref() == Some("mesh_coordinator")
                                    || n.id.as_deref() == Some("cynic-lxc-headscale")
                            }).and_then(|node| {
                                let ip = cynic_network::plane_vocabulary::best_plane_address_in_map(
                                        &node.network,
                                    )?;
                                let port = node.services.iter()
                                    .find(|s| s.name.as_deref() == Some("headscale"))
                                    .and_then(|s| s.port)
                                    .unwrap_or(8080);
                                Some(format!("http://{ip}:{port}"))
                            })
                        })
                        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string())
                });
            let mut adapter = cynic_node::adapters::headscale::HeadscaleAdapter::new(&ep);
            let resolved_key = api_key.clone().or_else(|| {
                cynic_secrets::resolve_secret("headscale-api-key", "CYNIC_HEADSCALE_KEY").or_else(
                    || cynic_secrets::resolve_secret("headscale-api-key", "HEADSCALE_API_KEY"),
                )
            });
            if let Some(key) = resolved_key {
                adapter = cynic_node::adapters::headscale::HeadscaleAdapter::with_api_key(&ep, key);
            }

            if *check_health {
                match adapter.check_health() {
                    Ok(health) => emit_json("mesh coordination health", &health, !health.healthy),
                    Err(error) => {
                        tracing::error!("mesh coordination health probe failed: {error}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                use cynic_node::ports::mesh_coordination_port::MeshCoordinationPort;
                let health = adapter.check_health().ok();
                let members = adapter.list_members().unwrap_or_default();
                let plan = network_mesh_plan::build_network_mesh_plan(registry, Some("headscale"));
                let desired_peers: Vec<cynic_node::domain::mesh_coordination::DesiredMeshPeer> = plan
                    .map(|p| {
                        p.desired_peers
                            .into_iter()
                            .map(|peer| {
                                let mut aliases = peer.dns_names;
                                if let Some(short) = peer.id.strip_prefix("cynic-") {
                                    aliases.push(short.to_string());
                                }
                                if let Some(short) = peer.id.strip_prefix("cynic-lxc-") {
                                    aliases.push(short.to_string());
                                }
                                cynic_node::domain::mesh_coordination::DesiredMeshPeer {
                                    id: peer.id.clone(),
                                    name: Some(peer.id),
                                    aliases,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let self_node_id =
                    cynic_host::process_observation::capture_command("hostname", &[])
                        .ok()
                        .filter(|o| o.success)
                        .map(|o| o.stdout.trim().to_string());

                let report = cynic_node::domain::mesh_coordination::reconcile_mesh_coordination(
                    "headscale",
                    &desired_peers,
                    &members,
                    self_node_id.as_deref(),
                );
                let payload = serde_json::json!({
                    "schema": report.schema,
                    "provider": report.provider,
                    "endpoint": ep,
                    "health": health,
                    "self_node": report.self_node,
                    "desired_peers": report.desired_peers,
                    "observed_members": report.observed_members,
                    "active_members": report.active_members,
                    "missing_members": report.missing_members,
                    "unmanaged_members": report.unmanaged_members,
                });
                emit_json("mesh coordination report", &payload, false)
            }
        }
        NetworkCommands::ActuateTarget {
            json: _,
            request,
            apply,
            harness_kind,
            target_attestation,
            observer_bin,
            virsh_bin,
            qga_timeout_seconds,
            qga_max_status_polls,
        } => {
            if !apply {
                tracing::error!("network actuation target requires --apply");
                return ExitCode::FAILURE;
            }
            let context = match selected_harness_context(harness_kind) {
                Ok(context) => context,
                Err(error) => {
                    tracing::error!("runtime harness selection failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let attestation = match LocalHarnessTargetAttestationReader
                .read_attestation(Path::new(target_attestation))
            {
                Ok(attestation) => attestation,
                Err(error) => {
                    tracing::error!("network actuation target attestation load failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let request = match read_network_actuation_request(Path::new(request)) {
                Ok(request) => request,
                Err(error) => {
                    tracing::error!("network actuation request load failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let config = QemuNetworkActuationConfig {
                virsh_bin: virsh_bin.clone(),
                observation_argv: vec![
                    observer_bin.clone(),
                    "network".to_string(),
                    "actuation".to_string(),
                    "observation".to_string(),
                    "--json".to_string(),
                ],
                timeout_seconds: *qga_timeout_seconds,
                max_status_polls: *qga_max_status_polls,
                poll_interval: Duration::from_millis(50),
            };
            match execute_qemu_network_actuation(
                &request,
                context,
                attestation,
                LocalArgvRunner,
                config,
            ) {
                Ok(report) => {
                    let should_fail = report.status != "proved";
                    emit_json("network actuation target", &report, should_fail)
                }
                Err(error) => {
                    tracing::error!("network actuation target failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::MeshApplyProfile {
            json: _,
            bundle_dir,
            profile,
            apply,
            closed_environment,
        } => match network_mesh_apply_plan::apply_network_mesh_profile(
            bundle_dir,
            profile,
            *apply,
            *closed_environment,
        ) {
            Ok(report) => {
                let should_fail = report.status != "completed";
                emit_json("network mesh apply profile", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("network mesh apply profile failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::ProviderFallbackStatus {
            json: _,
            registry,
            hosts_path,
            check_connectivity,
        } => match provider_fallback_status::build_provider_fallback_status(
            registry,
            hosts_path,
            *check_connectivity,
        ) {
            Ok(report) => {
                let should_fail = report.status != "active";
                emit_json("provider fallback status", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("provider fallback status failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::ProviderFallbackHosts {
            json: _,
            registry,
            output_dir,
        } => match provider_fallback_hosts::build_provider_fallback_hosts(registry, output_dir) {
            Ok(report) => {
                let should_fail = report.status == "empty";
                emit_json("provider fallback hosts", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("provider fallback hosts failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::ShadowProof {
            json: _,
            registry,
            resolve_dns,
            observe_runtime,
        } => {
            match network_shadow_proof::build_shadow_proof(registry, *resolve_dns, *observe_runtime)
            {
                Ok(report) => {
                    let should_fail = report.status == "drift_detected";
                    emit_json("network shadow proof", &report, should_fail)
                }
                Err(error) => {
                    tracing::error!("network shadow proof failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::RuntimeStatus {
            json: _,
            registry,
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        } => match network_runtime_status::build_network_runtime_status(
            registry,
            *resolve_dns,
            *check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        ) {
            Ok(report) => {
                let should_fail = should_fail_runtime_status(&report.status);
                emit_json("network runtime status", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("network runtime status failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::RuntimeProof {
            json: _,
            registry,
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        } => match cynic_node::network_runtime_proof::build_network_runtime_proof_bundle(
            registry,
            *resolve_dns,
            *check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        ) {
            Ok(report) => emit_json(
                "network runtime proof",
                &report,
                report.status != "satisfied",
            ),
            Err(error) => {
                tracing::error!("network runtime proof failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::ActivationChain {
            json: _,
            registry,
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        } => match network_activation_chain::build_activation_chain_report(
            registry,
            *resolve_dns,
            *check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        ) {
            Ok(report) => {
                let should_fail = report.status == "blocked";
                emit_json("network activation chain", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("network activation chain failed: {error}");
                ExitCode::FAILURE
            }
        },
    }
}

fn selected_harness_context(
    harness_kind: &str,
) -> Result<cynic_host::runtime_harness::RuntimeHarnessContext, String> {
    runtime_harness_catalog(Some(harness_kind))?
        .contexts
        .pop()
        .ok_or_else(|| "runtime harness catalog returned no selected context".to_string())
}

fn should_fail_runtime_status(status: &str) -> bool {
    status == "blocked"
}

fn emit_json<T: serde::Serialize>(label: &str, value: &T, should_fail: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => {
            println!("{payload}");
            if should_fail {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            tracing::error!("{label} serialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A registry the network arms can read, written by the test.
    ///
    /// These arms read the fleet registry, so a test that points them at
    /// `CynicPaths::registry_path()` reads whatever
    /// `/etc/cynic/registry.json` holds on the machine running it. That
    /// made the outcome a property of the host, not of the code: the arm
    /// passed on a workstation with a populated registry and failed on a
    /// CI runner where the file does not exist, and on the builder it
    /// failed again for a third reason - the installed registry there
    /// has no network_policy plane interface, and the firewall
    /// derivation fails closed without one.
    ///
    /// The registry is an input to these arms, so the test supplies it.
    fn registry_fixture() -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().expect("temp registry");
        std::fs::write(
            file.path(),
            serde_json::json!({
                "network_policy": {
                    "canonical_service_plane": "service",
                    "planes": {
                        "service": { "cidr": "10.10.10.0/24" },
                        "admin": { "cidr": "192.168.100.0/24" },
                        "mesh": { "cidr": "100.64.0.0/10", "interface": "wg0" }
                    }
                },
                "nodes": [
                    { "id": "probe-node", "services": [] }
                ]
            })
            .to_string(),
        )
        .expect("write registry fixture");
        file
    }

    #[test]
    fn runtime_status_only_fails_when_blocked() {
        assert!(should_fail_runtime_status("blocked"));
        assert!(!should_fail_runtime_status("provider_fallback_active"));
        assert!(!should_fail_runtime_status("mesh_active"));
    }

    #[test]
    fn security_audit_executes_live_telemetry_without_panic() {
        let registry = registry_fixture();
        let cmd = NetworkCommands::SecurityAudit {
            json: true,
            max_handshake_age_secs: 180,
            registry: registry.path().display().to_string(),
        };
        let code = execute_network_command(&cmd);
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn reconcile_observed_dry_run_executes_without_panic() {
        let registry = registry_fixture();
        let cmd = NetworkCommands::ReconcileObserved {
            json: true,
            registry: registry.path().display().to_string(),
            timeout_ms: 100,
            apply: false,
        };
        let code = execute_network_command(&cmd);
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn firewall_policy_executes_without_panic() {
        let registry = registry_fixture();
        let cmd = NetworkCommands::FirewallPolicy {
            json: true,
            registry: registry.path().display().to_string(),
            node_id: Some("probe-node".to_string()),
            audit: false,
        };
        let code = execute_network_command(&cmd);
        assert_eq!(code, ExitCode::SUCCESS);
    }
}
