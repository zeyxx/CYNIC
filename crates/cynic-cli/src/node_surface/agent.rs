//! Agent workspace, workload, and scheduler commands.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

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
