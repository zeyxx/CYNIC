//! Source commands — admission plan, materialization, and bootstrap.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

pub(crate) fn execute_source_admission_plan(request: &str, observation: &str) -> ExitCode {
    cynic_node::usecases::source_admission_uc::execute_source_admission_plan(request, observation)
}

pub(crate) fn execute_source_materialize(
    request: &str,
    state_root: &str,
    snapshot_root: &str,
    snapshot_base_url: Option<&str>,
    node_id: &str,
    apply: bool,
) -> ExitCode {
    cynic_node::usecases::source_materialization_uc::execute_source_materialization(
        request,
        state_root,
        snapshot_root,
        snapshot_base_url,
        node_id,
        apply,
    )
}

pub(crate) async fn execute_source_bootstrap(
    node: &str,
    repository: &str,
    source: &str,
    revision: &str,
    registry: &str,
    state_root: &str,
    agent_id: &str,
    kernel_endpoint: &str,
    apply: bool,
) -> ExitCode {
    cynic_node::usecases::source_repository_bootstrap_uc::execute_source_repository_bootstrap(
        node,
        repository,
        source,
        revision,
        registry,
        state_root,
        agent_id,
        kernel_endpoint,
        apply,
    )
    .await
}
