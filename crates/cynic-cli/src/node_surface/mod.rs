//! Node CLI surface — unified node command dispatch.
//!
//! Merged from:
//! - `node_activation_surface.rs` (8 variants: Status, Activate, IdentityInit, CsrCreate, CertInspect, CertVerify, HostAudit, Forensics)
//! - `node_surface/cli.rs` (16 variants: NodeIdentity, ConsensusStatus, Observe, RegistryObservation, Network, Peer, AgentWorkspace, AgentWorkspaceLifecycle, AgentSchedulerStep, AgentWorkloadStatus, AgentWorkloadTransition, BuildWorkloadGrantVerify, StorageReconcile, SourceAdmissionPlan, SourceMaterialize, SourceBootstrap)
//!
//! `Control` is excluded — it belongs to the daemon, not the CLI.

pub mod activation;
pub mod agent;
pub mod consensus;
pub mod identity;
pub mod observation;
pub mod source;
pub mod storage;
pub mod host;
pub mod compute;


use std::process::ExitCode;

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

/// Unified node command — all node-level CLI surfaces.
#[derive(Subcommand)]
pub enum NodeCommand {
    // === Activation (from node_activation_surface.rs) ===
    /// Observe node activation state.
    Status {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Activate a node (bounded action with read-back proof).
    Activate {
        #[arg(long)]
        node_id: String,
        #[arg(long, default_value_t = false)]
        yes: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Initialize or display the local node Ed25519 signing keypair.
    IdentityInit {
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a self-signed Certificate Signing Request (CSR) for node enrollment.
    CsrCreate {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Inspect a local Node Enrollment Certificate.
    CertInspect {
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Cryptographically verify a Node Enrollment Certificate against a Kernel authority public key.
    CertVerify {
        #[arg(long)]
        kernel_pubkey_hex: String,
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Audit host commits, divergence, and node health state.
    HostAudit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 80)]
        max_commits: usize,
    },
    /// Host crash forensics, boot timeline, and post-mortem evidence.
    Forensics {
        #[command(subcommand)]
        command: crate::forensics_surface::ForensicsCommand,
    },

    // === Identity (from node_surface/cli.rs) ===
    /// Observe or create the node-local Ed25519 identity used for machine enrollment.
    NodeIdentity {
        #[arg(long)]
        node_id: String,
        #[arg(long, default_value = "/var/lib/cynic/identity/node-signing.key")]
        signing_key_path: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },

    // === Consensus ===
    /// Evaluate fleet epoch consensus and clock skew across cluster nodes.
    ConsensusStatus {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 1)]
        expected_epoch: u64,
        #[arg(long, default_value = "HEAD")]
        expected_commit: String,
        #[arg(long, default_value_t = 5000)]
        max_skew_ms: u64,
    },

    // === Observation ===
    /// Emit local node observation JSON and exit.
    Observe {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long)]
        fail_on_warning: bool,
        #[arg(long)]
        fail_on_critical: bool,
    },
    /// Emit the node observation envelope consumed by registry observation projection.
    RegistryObservation {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long)]
        output: Option<String>,
    },

    // === Host Probes ===
    /// Probe host hardware: CPU, memory, storage disks, and capacity.
    Host {
        #[command(subcommand)]
        command: host::NodeHostCommand,
    },

    // === Compute Substrates & Hybrid Mesh ===
    /// Run sovereign compute workloads or enroll remote hybrid runners.
    Compute {
        #[command(subcommand)]
        command: compute::NodeComputeCommand,
    },

    // === Peer ===
    /// CynicPeer sovereign state engine, local storage and ingress commands.
    Peer {
        #[command(subcommand)]
        command: cynic_node::peer_commands::PeerCommands,
    },

    // === Agent ===
    /// Plan or apply one lease-fenced source change in a sovereign agent worktree.
    AgentWorkspace {
        #[arg(long)]
        request: String,
        #[arg(long)]
        result_file: Option<String>,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        #[arg(long, default_value_t = 3_600_000)]
        max_claim_ttl_ms: u64,
        #[arg(long, default_value_t = 604_800_000)]
        max_retention_ms: u64,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Observe, plan, or collect one completed sovereign agent workspace.
    AgentWorkspaceLifecycle {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        #[arg(long)]
        agent_id: String,
        #[arg(long, default_value_t = 300_000)]
        renewal_window_ms: u64,
        #[arg(long, default_value_t = 3_600_000)]
        renewal_ttl_ms: u64,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile and durably journal one scheduler step without executing it.
    AgentSchedulerStep {
        #[arg(long)]
        observation: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
    /// Compose a scheduler observation from live host capacity plus the
    /// given budgets and lifecycle files. No decision, only evidence.
    AgentSchedulerObserve {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        scheduler_epoch: u64,
        #[arg(long)]
        step_id: String,
        /// JSON file: {"limit": AgentBudget, "reserved": AgentBudget}.
        #[arg(long)]
        budgets: String,
        /// JSON file: AgentWorkspaceLifecyclePlan.
        #[arg(long)]
        lifecycle: String,
        /// Where to write the observation JSON. The load policy is read
        /// from "<output>.load-policy.json".
        #[arg(long)]
        output: String,
        #[arg(long)]
        json: bool,
    },
    /// Read one append-only node-local workload record.
    AgentWorkloadStatus {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan or apply one revision-fenced workload state transition.
    AgentWorkloadTransition {
        #[arg(long)]
        request: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Verify a machine-signed build grant against its live durable workload record.
    BuildWorkloadGrantVerify {
        #[arg(long)]
        grant: String,
        #[arg(long)]
        workload_record: String,
        #[arg(long)]
        authority_public_key: String,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        json: bool,
    },

    // === Storage ===
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
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },

    // === Source ===
    /// Compose a non-mutating source-to-build-runner admission plan.
    SourceAdmissionPlan {
        #[arg(long)]
        request: String,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan or apply one digest-addressed source materialization on a build runner.
    SourceMaterialize {
        #[arg(long)]
        request: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "/var/lib/cynic/source/snapshots")]
        snapshot_root: String,
        #[arg(long)]
        snapshot_base_url: Option<String>,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Observe, plan or materialize one registry-declared source repository.
    SourceBootstrap {
        #[arg(long)]
        node: String,
        #[arg(long)]
        repository: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        revision: String,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        agent_id: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

/// Execute a unified node command.
pub async fn execute_node(command: NodeCommand) -> ExitCode {
    match command {
        // === Activation ===
        NodeCommand::Status { node_id, json } => activation::execute_status(node_id, json),
        NodeCommand::Activate { node_id, yes, json } => activation::execute_activate(node_id, yes, json),
        NodeCommand::IdentityInit { key_path, json } => activation::execute_identity_init(key_path, json),
        NodeCommand::CsrCreate { node_id, role, key_path, json } => activation::execute_csr_create(node_id, role, key_path, json),
        NodeCommand::CertInspect { cert_path, json } => activation::execute_cert_inspect(cert_path, json),
        NodeCommand::CertVerify { kernel_pubkey_hex, cert_path, json } => activation::execute_cert_verify(kernel_pubkey_hex, cert_path, json),
        NodeCommand::HostAudit { json, max_commits } => activation::execute_host_audit(json, max_commits),
        NodeCommand::Forensics { command } => crate::forensics_surface::execute_forensics(command),

        // === Identity ===
        NodeCommand::NodeIdentity { node_id, signing_key_path, apply, json } => {
            identity::execute_node_identity(&node_id, &signing_key_path, apply, json)
        }

        // === Consensus ===
        NodeCommand::ConsensusStatus { json: _, expected_epoch, expected_commit, max_skew_ms } => {
            consensus::execute_consensus_status(expected_epoch, &expected_commit, max_skew_ms)
        }

        // === Observation ===
        NodeCommand::Observe { json: _, registry, fail_on_warning, fail_on_critical } => {
            observation::execute_observe(&registry, fail_on_warning, fail_on_critical)
        }
        NodeCommand::RegistryObservation { json: _, registry, output } => {
            observation::execute_registry_observation(&registry, output.as_deref())
        }

        // === Host Probes ===
        NodeCommand::Host { command } => host::execute_node_host(command),

        // === Compute Substrates & Hybrid Mesh ===
        NodeCommand::Compute { command } => compute::dispatch(command).await,

        // === Peer ===
        NodeCommand::Peer { command } => {
            cynic_node::adapters::peer_storage::ingress::execute_peer_command(&command)
        }

        // === Agent ===
        NodeCommand::AgentWorkspace { request, result_file, registry, state_root, kernel_endpoint, max_claim_ttl_ms, max_retention_ms, apply, json: _ } => {
            agent::execute_agent_workspace(&request, result_file.as_deref(), &registry, &state_root, &kernel_endpoint, max_claim_ttl_ms, max_retention_ms, apply).await
        }
        NodeCommand::AgentWorkspaceLifecycle { workload_id, registry, state_root, kernel_endpoint, agent_id, renewal_window_ms, renewal_ttl_ms, apply, json: _ } => {
            agent::execute_agent_workspace_lifecycle(&workload_id, &registry, &state_root, &kernel_endpoint, &agent_id, renewal_window_ms, renewal_ttl_ms, apply).await
        }
        NodeCommand::AgentSchedulerStep { observation, state_root, json: _ } => {
            agent::execute_agent_scheduler_step(&observation, &state_root)
        }
        NodeCommand::AgentSchedulerObserve { node_id, scheduler_epoch, step_id, budgets, lifecycle, output, json: _ } => {
            agent::execute_agent_scheduler_observe(&node_id, scheduler_epoch, &step_id, &budgets, &lifecycle, &output)
        }
        NodeCommand::AgentWorkloadStatus { workload_id, state_root, json: _ } => {
            agent::execute_agent_workload_status(&workload_id, &state_root)
        }
        NodeCommand::AgentWorkloadTransition { request, state_root, apply, json: _ } => {
            agent::execute_agent_workload_transition(&request, &state_root, apply)
        }
        NodeCommand::BuildWorkloadGrantVerify { grant, workload_record, authority_public_key, node_id, source_commit, action, json: _ } => {
            agent::execute_build_workload_grant_verify(&grant, &workload_record, &authority_public_key, &node_id, &source_commit, &action)
        }

        // === Storage ===
        NodeCommand::StorageReconcile { state_dir, warmup_seconds, restart_window_seconds, max_restarts_per_window, apply, json: _ } => {
            storage::execute_storage_reconcile(&state_dir, warmup_seconds, restart_window_seconds, max_restarts_per_window, apply).await
        }

        // === Source ===
        NodeCommand::SourceAdmissionPlan { request, observation, json: _ } => {
            source::execute_source_admission_plan(&request, &observation)
        }
        NodeCommand::SourceMaterialize { request, state_root, snapshot_root, snapshot_base_url, node_id, apply, json: _ } => {
            source::execute_source_materialize(&request, &state_root, &snapshot_root, snapshot_base_url.as_deref(), &node_id, apply)
        }
        NodeCommand::SourceBootstrap { node, repository, source, revision, registry, state_root, agent_id, kernel_endpoint, apply, json: _ } => {
            source::execute_source_bootstrap(&node, &repository, &source, &revision, &registry, &state_root, &agent_id, &kernel_endpoint, apply).await
        }
    }
}
