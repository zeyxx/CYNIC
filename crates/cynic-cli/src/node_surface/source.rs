//! Source commands — admission plan, materialization, and bootstrap.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

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
    ).await
}

/// Source admission arms: compose a non-mutating admission plan.
#[derive(Subcommand)]
pub enum NodeSourceAdmissionCommand {
    /// Compose a non-mutating source-to-build-runner admission plan.
    Plan {
        #[arg(long)]
        request: String,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        json: bool,
    },
}

/// Node source arms: admission plan, materialization, and bootstrap.
#[derive(Subcommand)]
pub enum NodeSourceCommand {
    /// Source admission arms.
    Admission {
        #[command(subcommand)]
        command: NodeSourceAdmissionCommand,
    },
    /// Plan or apply one digest-addressed source materialization on a build runner.
    Materialize {
        #[arg(long)]
        request: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
        state_root: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().join("source/snapshots").display().to_string())]
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
    Bootstrap {
        #[arg(long)]
        node: String,
        #[arg(long)]
        repository: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        revision: String,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
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

