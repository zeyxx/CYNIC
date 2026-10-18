//! Agent workspace, workload, and scheduler commands.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

/// Agent workspace arms: one lease-fenced source change, and its lifecycle.
#[derive(Subcommand)]
pub enum NodeAgentWorkspaceCommand {
    /// Plan or apply one lease-fenced source change in a sovereign agent worktree.
    Change {
        #[arg(long)]
        request: String,
        #[arg(long)]
        result_file: Option<String>,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
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
    Lifecycle {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
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
}

/// Agent scheduler arms: journal one step without executing it.
#[derive(Subcommand)]
pub enum NodeAgentSchedulerCommand {
    /// Reconcile and durably journal one scheduler step without executing it.
    Step {
        #[arg(long)]
        observation: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
}

/// Agent workload arms: read one record, or plan/apply one transition.
#[derive(Subcommand)]
pub enum NodeAgentWorkloadCommand {
    /// Read one append-only node-local workload record.
    Status {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan or apply one revision-fenced workload state transition.
    Transition {
        #[arg(long)]
        request: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
        state_root: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

/// Agent build-grant arms: verify a machine-signed grant against its record.
#[derive(Subcommand)]
pub enum NodeAgentBuildCommand {
    /// Build workload grant arms.
    Workload {
        #[command(subcommand)]
        command: NodeAgentBuildWorkloadCommand,
    },
}

/// Build workload arms.
#[derive(Subcommand)]
pub enum NodeAgentBuildWorkloadCommand {
    /// Build workload grant arms.
    Grant {
        #[command(subcommand)]
        command: NodeAgentBuildWorkloadGrantCommand,
    },
}

/// Build workload grant arms.
#[derive(Subcommand)]
pub enum NodeAgentBuildWorkloadGrantCommand {
    /// Verify a machine-signed build grant against its live durable workload record.
    Verify {
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
}

/// Agent command family: workspace, scheduler, workload, and build grants.
#[derive(Subcommand)]
pub enum NodeAgentCommand {
    /// Agent workspace arms (one source change, and its lifecycle).
    Workspace {
        #[command(subcommand)]
        command: NodeAgentWorkspaceCommand,
    },
    /// Agent scheduler arms.
    Scheduler {
        #[command(subcommand)]
        command: NodeAgentSchedulerCommand,
    },
    /// Agent workload arms.
    Workload {
        #[command(subcommand)]
        command: NodeAgentWorkloadCommand,
    },
    /// Agent build-grant arms.
    Build {
        #[command(subcommand)]
        command: NodeAgentBuildCommand,
    },
}

pub(crate) async fn execute_node_agent(command: NodeAgentCommand) -> ExitCode {
    match command {
        NodeAgentCommand::Workspace { command } => match command {
            NodeAgentWorkspaceCommand::Change {
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
                execute_agent_workspace(
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
            NodeAgentWorkspaceCommand::Lifecycle {
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
                execute_agent_workspace_lifecycle(
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
        },
        NodeAgentCommand::Scheduler { command } => match command {
            NodeAgentSchedulerCommand::Step {
                observation,
                state_root,
                json: _,
            } => execute_agent_scheduler_step(&observation, &state_root),
        },
        NodeAgentCommand::Workload { command } => match command {
            NodeAgentWorkloadCommand::Status {
                workload_id,
                state_root,
                json: _,
            } => execute_agent_workload_status(&workload_id, &state_root),
            NodeAgentWorkloadCommand::Transition {
                request,
                state_root,
                apply,
                json: _,
            } => execute_agent_workload_transition(&request, &state_root, apply),
        },
        NodeAgentCommand::Build { command } => match command {
            NodeAgentBuildCommand::Workload { command } => match command {
                NodeAgentBuildWorkloadCommand::Grant { command } => match command {
                    NodeAgentBuildWorkloadGrantCommand::Verify {
                        grant,
                        workload_record,
                        authority_public_key,
                        node_id,
                        source_commit,
                        action,
                        json: _,
                    } => execute_build_workload_grant_verify(
                        &grant,
                        &workload_record,
                        &authority_public_key,
                        &node_id,
                        &source_commit,
                        &action,
                    ),
                },
            },
        },
    }
}

pub(crate) async fn execute_agent_workspace(
    request: &str,
    result_file: Option<&str>,
    registry: &str,
    state_root: &str,
    kernel_endpoint: &str,
    max_claim_ttl_ms: u64,
    max_retention_ms: u64,
    apply: bool,
) -> ExitCode {
    cynic_node::usecases::agent_source_change_uc::execute_agent_source_change(
        request,
        result_file,
        registry,
        state_root,
        kernel_endpoint,
        max_claim_ttl_ms,
        max_retention_ms,
        apply,
    ).await
}

pub(crate) async fn execute_agent_workspace_lifecycle(
    workload_id: &str,
    registry: &str,
    state_root: &str,
    kernel_endpoint: &str,
    agent_id: &str,
    renewal_window_ms: u64,
    renewal_ttl_ms: u64,
    apply: bool,
) -> ExitCode {
    cynic_node::usecases::agent_workspace_lifecycle_uc::execute_agent_workspace_lifecycle(
        workload_id,
        registry,
        state_root,
        kernel_endpoint,
        agent_id,
        renewal_window_ms,
        renewal_ttl_ms,
        apply,
    ).await
}

pub(crate) fn execute_agent_scheduler_step(observation: &str, state_root: &str) -> ExitCode {
    cynic_node::usecases::agent_scheduler_step_uc::execute_agent_scheduler_step(observation, state_root)
}

pub(crate) fn execute_agent_workload_status(workload_id: &str, state_root: &str) -> ExitCode {
    cynic_node::usecases::agent_workload_store_uc::execute_agent_workload_status(workload_id, state_root)
}

pub(crate) fn execute_agent_workload_transition(
    request: &str,
    state_root: &str,
    apply: bool,
) -> ExitCode {
    cynic_node::usecases::agent_workload_store_uc::execute_agent_workload_transition(
        request, state_root, apply,
    )
}

pub(crate) fn execute_build_workload_grant_verify(
    grant: &str,
    workload_record: &str,
    authority_public_key: &str,
    node_id: &str,
    source_commit: &str,
    action: &str,
) -> ExitCode {
    cynic_node::usecases::build_workload_grant_uc::execute_build_workload_grant_verify(
        grant,
        workload_record,
        authority_public_key,
        node_id,
        source_commit,
        action,
    )
}
