//! Node CLI surface — unified node command dispatch.
//!
//! Merged from:
//! - `node_activation_surface.rs` (8 variants: Status, Activate, IdentityInit, CsrCreate, CertInspect, CertVerify, HostAudit, Forensics)
//! - `node_surface/cli.rs` (16 variants: NodeIdentity, ConsensusStatus, Observe, RegistryObservation, Network, Peer, AgentWorkspace, AgentWorkspaceLifecycle, AgentSchedulerStep, AgentWorkloadStatus, AgentWorkloadTransition, BuildWorkloadGrantVerify, StorageReconcile, SourceAdmissionPlan, SourceMaterialize, SourceBootstrap)
//!
//! `Control` is excluded — it belongs to the daemon, not the CLI.

pub mod activation;
pub mod agent;
pub mod bootstrap;
pub mod consensus;
pub mod identity;
pub mod observation;
pub mod source;
pub mod storage;
pub mod host;
pub mod hypervisor;
pub mod compute;


use std::process::ExitCode;

use clap::Subcommand;

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
    /// Node identity arms: initialize/display the keypair, or observe/create
    /// the node-local identity record.
    Identity {
        #[command(subcommand)]
        command: identity::NodeIdentityCommand,
    },
    /// Node certificate signing request arms.
    Csr {
        #[command(subcommand)]
        command: activation::NodeCsrCommand,
    },
    /// Node certificate arms: inspect, or verify against a kernel authority.
    Cert {
        #[command(subcommand)]
        command: activation::NodeCertCommand,
    },
    /// Host crash forensics, boot timeline, and post-mortem evidence.
    Forensics {
        #[command(subcommand)]
        command: crate::forensics_surface::ForensicsCommand,
    },

    // === Identity === (merged into Identity arms above)

    // === Consensus ===
    /// Fleet consensus arms: epoch consensus and clock skew.
    Consensus {
        #[command(subcommand)]
        command: consensus::NodeConsensusCommand,
    },

    // === Observation ===
    /// Emit local node observation JSON and exit.
    Observe {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long)]
        fail_on_warning: bool,
        #[arg(long)]
        fail_on_critical: bool,
    },
    // === Registry ===
    /// Node registry arms: the observation envelope.
    Registry {
        #[command(subcommand)]
        command: observation::NodeRegistryCommand,
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

    /// Plan or observe hypervisor VM placement and preflight (node substrate family).
    Hypervisor {
        #[command(subcommand)]
        command: hypervisor::NodeHypervisorCommand,
    },

    // === Peer ===
    /// CynicPeer sovereign state engine, local storage and ingress commands.
    Peer {
        #[command(subcommand)]
        command: cynic_node::peer_commands::PeerCommands,
    },

    // === Agent ===
    /// Agent workspace, scheduler, workload, and build-grant arms.
    Agent {
        #[command(subcommand)]
        command: agent::NodeAgentCommand,
    },

    // === Storage ===
    /// Node storage arms: observe or reconcile a node-local storage service.
    Storage {
        #[command(subcommand)]
        command: storage::NodeStorageCommand,
    },

    // === Source ===
    /// Node source arms: admission plan, materialization, and bootstrap.
    Source {
        #[command(subcommand)]
        command: source::NodeSourceCommand,
    },

    // === Framework Bootstrap ===
    /// Render a bring-up bundle for a fresh node (no local install).
    Bootstrap {
        #[command(subcommand)]
        command: bootstrap::NodeBootstrapCommand,
    },
}

/// Execute a unified node command.
pub async fn execute_node(command: NodeCommand) -> ExitCode {
    match command {
        // === Activation ===
        NodeCommand::Status { node_id, json } => activation::execute_status(node_id, json),
        NodeCommand::Activate { node_id, yes, json } => activation::execute_activate(node_id, yes, json),
        NodeCommand::Identity { command } => identity::execute_node_identity_command(command),
        NodeCommand::Csr { command } => activation::execute_node_csr(command),
        NodeCommand::Cert { command } => activation::execute_node_cert(command),
        NodeCommand::Forensics { command } => crate::forensics_surface::execute_forensics(command),

        // === Identity ===
        // === Consensus ===
        NodeCommand::Consensus { command } => consensus::execute_node_consensus(command),

        // === Observation ===
        NodeCommand::Observe { json: _, registry, fail_on_warning, fail_on_critical } => {
            observation::execute_observe(&registry, fail_on_warning, fail_on_critical)
        }
        NodeCommand::Registry { command } => observation::execute_node_registry(command),

        // === Host Probes ===
        NodeCommand::Host { command } => host::execute_node_host(command),

        // === Compute Substrates & Hybrid Mesh ===
        NodeCommand::Compute { command } => compute::dispatch(command).await,

        // === Hypervisor Placement & Preflight (migrated from `agent host`) ===
        NodeCommand::Hypervisor { command } => hypervisor::execute_node_hypervisor(command),

        // === Peer ===
        NodeCommand::Peer { command } => {
            cynic_node::adapters::peer_storage::ingress::execute_peer_command(&command)
        }

        // === Agent ===
        NodeCommand::Agent { command } => agent::execute_node_agent(command).await,

        // === Storage ===
        NodeCommand::Storage { command } => storage::execute_node_storage(command).await,

        // === Source ===
        NodeCommand::Source { command } => source::execute_node_source(command).await,

        // === Framework Bootstrap ===
        NodeCommand::Bootstrap { command } => bootstrap::execute_node_bootstrap(command),
    }
}
