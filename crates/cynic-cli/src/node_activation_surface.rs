use std::process::ExitCode;

use clap::Subcommand;

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

#[derive(Subcommand)]
pub(crate) enum NodeActivationCommand {
    /// Observe node activation state.
    Status {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_node_activation(command: NodeActivationCommand) -> ExitCode {
    match command {
        NodeActivationCommand::Status { node_id, json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path = resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let adapter = cynic_host::node_activation::NodeActivationAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let verdict = adapter.observe(&node_id);
            crate::cli_json::emit_pretty_json(&verdict)
        }
    }
}
