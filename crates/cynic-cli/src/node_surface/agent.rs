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

pub(crate) fn execute_agent_scheduler_observe(
    node_id: &str,
    scheduler_epoch: u64,
    step_id: &str,
    budgets: &str,
    lifecycle: &str,
    output: &str,
) -> ExitCode {
    use cynic_host::adapters::local_capacity::LocalCapacityAdapter;
    use cynic_protocol::capacity::LoadPolicy;
    use cynic_protocol::AgentBudget;
    use cynic_workload::{
        AGENT_SCHEDULER_STEP_OBSERVATION_SCHEMA, AgentSchedulerStepObservation,
        AgentWorkspaceLifecyclePlan, SchedulerCapacitySnapshot,
    };

    #[derive(serde::Deserialize)]
    struct BudgetsFile {
        limit: AgentBudget,
        reserved: AgentBudget,
    }

    let fail = |reason: String| {
        eprintln!("scheduler_observe_failed: {reason}");
        ExitCode::FAILURE
    };
    let read_file = |path: &str, what: &str| match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Err(fail(format!("{what}_unreadable: {e}"))),
    };
    let budgets_raw = match read_file(budgets, "budgets") {
        Ok(content) => content,
        Err(code) => return code,
    };
    let budgets: BudgetsFile = match serde_json::from_str(&budgets_raw) {
        Ok(parsed) => parsed,
        Err(e) => return fail(format!("budgets_unparsable: {e}")),
    };
    let lifecycle_raw = match read_file(lifecycle, "lifecycle") {
        Ok(content) => content,
        Err(code) => return code,
    };
    let lifecycle: AgentWorkspaceLifecyclePlan = match serde_json::from_str(&lifecycle_raw) {
        Ok(parsed) => parsed,
        Err(e) => return fail(format!("lifecycle_unparsable: {e}")),
    };
    let policy_raw = match read_file(&(output.to_string() + ".load-policy.json"), "load_policy") {
        Ok(content) => content,
        Err(code) => return code,
    };
    let policy: LoadPolicy = match serde_json::from_str(&policy_raw) {
        Ok(parsed) => parsed,
        Err(e) => return fail(format!("load_policy_unparsable: {e}")),
    };

    // Live capacity, never invented: a failed probe records unmeasured
    // (None) and the step decides on budgets alone.
    let capacity = match LocalCapacityAdapter::new().assess_capacity(&policy) {
        Ok(assessment) => Some(SchedulerCapacitySnapshot {
            free_ram_bytes: assessment.available_ram_bytes,
            tier: assessment.state,
        }),
        Err(reason) => {
            eprintln!("scheduler_observe_capacity_unmeasured: {reason}");
            None
        }
    };

    let observation = AgentSchedulerStepObservation {
        schema: AGENT_SCHEDULER_STEP_OBSERVATION_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        scheduler_epoch,
        step_id: step_id.to_string(),
        aggregate_budget_limit: budgets.limit,
        aggregate_budget_reserved: budgets.reserved,
        lifecycle,
        capacity,
    };
    let rendered = match serde_json::to_string_pretty(&observation) {
        Ok(rendered) => rendered,
        Err(e) => return fail(format!("observation_unrenderable: {e}")),
    };
    match std::fs::write(output, format!("{rendered}\n")) {
        Ok(()) => (),
        Err(e) => return fail(format!("observation_unwritable: {e}")),
    };
    println!("{output}");
    ExitCode::SUCCESS
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
