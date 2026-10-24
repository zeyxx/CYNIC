//! Node observation commands — local observation and registry observation envelope.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

pub(crate) fn execute_observe(
    registry: &str,
    fail_on_warning: bool,
    fail_on_critical: bool,
) -> ExitCode {
    crate::network_surface::execution::execute_observe(
        registry,
        fail_on_warning,
        fail_on_critical,
    )
}

pub(crate) fn execute_registry_observation(
    registry: &str,
    output: Option<&str>,
) -> ExitCode {
    crate::network_surface::execution::execute_registry_observation(
        registry,
        output,
    )
}

/// Node registry arms: the observation envelope consumed by the registry
/// observation projection.
#[derive(Subcommand)]
pub enum NodeRegistryCommand {
    /// Emit the node observation envelope consumed by registry observation projection.
    Observation {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())]
        registry: String,
        #[arg(long)]
        output: Option<String>,
    },
}

