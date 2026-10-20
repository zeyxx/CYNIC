//! Node CLI surface — unified node command dispatch.
//!
//! The node surface is organised in two levels: a noun family and a verb.
//! `cynic node cert inspect` is a family (`cert`) and a command (`inspect`);
//! it replaces the flat `cynic node cert-inspect`, which contracts/foundation.json
//! forbids because a hyphen hides the missing noun level. The families and
//! their commands are declared in `subcommands.rs`; the execution stays in the
//! domain modules below.
//!
//! `Control` is excluded — it belongs to the daemon, not the CLI.

pub mod activation;
pub mod agent;
pub mod compute;
pub mod consensus;
pub mod host;
pub mod identity;
pub mod observation;
pub mod source;
pub mod storage;
pub mod subcommands;

use std::process::ExitCode;

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

use subcommands::{
    NodeAgentCommand, NodeCertCommand, NodeConsensusCommand, NodeGrantCommand,
    NodeIdentityCommand, NodeRegistryCommand, NodeSourceCommand, NodeStorageCommand,
    NodeWorkloadCommand,
};

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
    /// Node-local Ed25519 identity, enrollment and CSR surfaces.
    Identity {
        #[command(subcommand)]
        command: NodeIdentityCommand,
    },
    /// Node enrollment certificate surfaces.
    Cert {
        #[command(subcommand)]
        command: NodeCertCommand,
    },
    /// Host crash forensics, boot timeline, and post-mortem evidence.
    Forensics {
        #[command(subcommand)]
        command: crate::forensics_surface::ForensicsCommand,
    },

    // === Identity (from node_surface/cli.rs) ===

    // === Consensus ===
    /// Fleet epoch consensus and clock skew.
    Consensus {
        #[command(subcommand)]
        command: NodeConsensusCommand,
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
    /// Registry observation and projection.
    Registry {
        #[command(subcommand)]
        command: NodeRegistryCommand,
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
    /// Sovereign agent worktree and lifecycle surfaces.
    Agent {
        #[command(subcommand)]
        command: NodeAgentCommand,
    },

    // === Workload ===
    /// Node-local workload records and state transitions.
    Workload {
        #[command(subcommand)]
        command: NodeWorkloadCommand,
    },

    // === Grant ===
    /// Machine-signed build grants.
    Grant {
        #[command(subcommand)]
        command: NodeGrantCommand,
    },

    // === Storage ===
    /// Node-local storage services.
    Storage {
        #[command(subcommand)]
        command: NodeStorageCommand,
    },

    // === Source ===
    /// Source admission, materialization and bootstrap.
    Source {
        #[command(subcommand)]
        command: NodeSourceCommand,
    },
}

/// Execute a unified node command.
pub async fn execute_node(command: NodeCommand) -> ExitCode {
    match command {
        // === Activation ===
        NodeCommand::Status { node_id, json } => activation::execute_status(node_id, json),
        NodeCommand::Activate { node_id, yes, json } => {
            activation::execute_activate(node_id, yes, json)
        }
        NodeCommand::Forensics { command } => crate::forensics_surface::execute_forensics(command),

        // === Identity and certificates ===
        NodeCommand::Identity { command } => execute_node_identity(command),
        NodeCommand::Cert { command } => execute_node_cert(command),

        // === Consensus ===
        NodeCommand::Consensus { command } => execute_node_consensus(command),

        // === Observation ===
        NodeCommand::Observe {
            json: _,
            registry,
            fail_on_warning,
            fail_on_critical,
        } => observation::execute_observe(&registry, fail_on_warning, fail_on_critical),
        NodeCommand::Registry { command } => execute_node_registry(command),

        // === Host Probes ===
        NodeCommand::Host { command } => host::execute_node_host(command),

        // === Compute Substrates & Hybrid Mesh ===
        NodeCommand::Compute { command } => compute::dispatch(command).await,

        // === Peer ===
        NodeCommand::Peer { command } => {
            cynic_node::adapters::peer_storage::ingress::execute_peer_command(&command)
        }

        // === Agent, workload and grants ===
        NodeCommand::Agent { command } => execute_node_agent(command).await,
        NodeCommand::Workload { command } => execute_node_workload(command),
        NodeCommand::Grant { command } => execute_node_grant(command),

        // === Storage ===
        NodeCommand::Storage { command } => execute_node_storage(command).await,

        // === Source ===
        NodeCommand::Source { command } => execute_node_source(command).await,
    }
}

fn execute_node_identity(command: NodeIdentityCommand) -> ExitCode {
    match command {
        NodeIdentityCommand::Init { key_path, json } => {
            activation::execute_identity_init(key_path, json)
        }
        NodeIdentityCommand::Observe {
            node_id,
            signing_key_path,
            apply,
            json,
        } => identity::execute_node_identity(&node_id, &signing_key_path, apply, json),
        NodeIdentityCommand::Csr {
            node_id,
            role,
            key_path,
            json,
        } => activation::execute_csr_create(node_id, role, key_path, json),
    }
}

fn execute_node_cert(command: NodeCertCommand) -> ExitCode {
    match command {
        NodeCertCommand::Inspect { cert_path, json } => {
            activation::execute_cert_inspect(cert_path, json)
        }
        NodeCertCommand::Verify {
            kernel_pubkey_hex,
            cert_path,
            json,
        } => activation::execute_cert_verify(kernel_pubkey_hex, cert_path, json),
    }
}

fn execute_node_consensus(command: NodeConsensusCommand) -> ExitCode {
    match command {
        NodeConsensusCommand::Status {
            json: _,
            expected_epoch,
            expected_commit,
            max_skew_ms,
        } => consensus::execute_consensus_status(expected_epoch, &expected_commit, max_skew_ms),
    }
}

fn execute_node_registry(command: NodeRegistryCommand) -> ExitCode {
    match command {
        NodeRegistryCommand::Observe {
            json: _,
            registry,
            output,
        } => observation::execute_registry_observation(&registry, output.as_deref()),
    }
}

async fn execute_node_agent(command: NodeAgentCommand) -> ExitCode {
    match command {
        NodeAgentCommand::Workspace {
            request,
            result_file,
            registry,
            state_root,
            kernel_endpoint,
            max_claim_ttl_ms,
            max_retention_ms,
            apply,
            json: _,
        } => {
            agent::execute_agent_workspace(
                &request,
                result_file.as_deref(),
                &registry,
                &state_root,
                &kernel_endpoint,
                max_claim_ttl_ms,
                max_retention_ms,
                apply,
            )
            .await
        }
        NodeAgentCommand::Lifecycle {
            workload_id,
            registry,
            state_root,
            kernel_endpoint,
            agent_id,
            renewal_window_ms,
            renewal_ttl_ms,
            apply,
            json: _,
        } => {
            agent::execute_agent_workspace_lifecycle(
                &workload_id,
                &registry,
                &state_root,
                &kernel_endpoint,
                &agent_id,
                renewal_window_ms,
                renewal_ttl_ms,
                apply,
            )
            .await
        }
        NodeAgentCommand::Scheduler {
            observation,
            state_root,
            json: _,
        } => agent::execute_agent_scheduler_step(&observation, &state_root),
    }
}

fn execute_node_workload(command: NodeWorkloadCommand) -> ExitCode {
    match command {
        NodeWorkloadCommand::Status {
            workload_id,
            state_root,
            json: _,
        } => agent::execute_agent_workload_status(&workload_id, &state_root),
        NodeWorkloadCommand::Transition {
            request,
            state_root,
            apply,
            json: _,
        } => agent::execute_agent_workload_transition(&request, &state_root, apply),
    }
}

fn execute_node_grant(command: NodeGrantCommand) -> ExitCode {
    match command {
        NodeGrantCommand::Verify {
            grant,
            workload_record,
            authority_public_key,
            node_id,
            source_commit,
            action,
            json: _,
        } => agent::execute_build_workload_grant_verify(
            &grant,
            &workload_record,
            &authority_public_key,
            &node_id,
            &source_commit,
            &action,
        ),
    }
}

async fn execute_node_storage(command: NodeStorageCommand) -> ExitCode {
    match command {
        NodeStorageCommand::Reconcile {
            state_dir,
            warmup_seconds,
            restart_window_seconds,
            max_restarts_per_window,
            apply,
            json: _,
        } => {
            storage::execute_storage_reconcile(
                &state_dir,
                warmup_seconds,
                restart_window_seconds,
                max_restarts_per_window,
                apply,
            )
            .await
        }
    }
}

async fn execute_node_source(command: NodeSourceCommand) -> ExitCode {
    match command {
        NodeSourceCommand::Admission {
            request,
            observation,
            json: _,
        } => source::execute_source_admission_plan(&request, &observation),
        NodeSourceCommand::Materialize {
            request,
            state_root,
            snapshot_root,
            snapshot_base_url,
            node_id,
            apply,
            json: _,
        } => source::execute_source_materialize(
            &request,
            &state_root,
            &snapshot_root,
            snapshot_base_url.as_deref(),
            &node_id,
            apply,
        ),
        NodeSourceCommand::Bootstrap {
            node,
            repository,
            source,
            revision,
            registry,
            state_root,
            agent_id,
            kernel_endpoint,
            apply,
            json: _,
        } => {
            source::execute_source_bootstrap(
                &node,
                &repository,
                &source,
                &revision,
                &registry,
                &state_root,
                &agent_id,
                &kernel_endpoint,
                apply,
            )
            .await
        }
    }
}
