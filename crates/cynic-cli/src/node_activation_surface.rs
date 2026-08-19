use std::process::ExitCode;

use clap::Subcommand;

use cynic_host::node_activation::NodeActivationPort;
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
    /// Activate a node (bounded action with read-back proof).
    Activate {
        #[arg(long)]
        node_id: String,
        /// Explicit consent — required for mutation.
        #[arg(long, default_value_t = false)]
        yes: bool,
        #[arg(long, default_value_t = false)]
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
        NodeActivationCommand::Activate { node_id, yes, json } => {
            let registry_path = resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let adapter = cynic_host::node_activation::NodeActivationAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let report = adapter.activate(&node_id, yes);
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                match &report.result {
                    Some(Ok(proof)) => {
                        match proof {
                            cynic_protocol::node_activation::ActivationProof::Activated { evidence_refs, .. } => {
                                println!("ACTIVATED: {} (evidence: {})", node_id, evidence_refs.join(", "));
                            }
                            cynic_protocol::node_activation::ActivationProof::Failed { blockers, .. } => {
                                println!("FAILED: {} (blockers: {})", node_id, blockers.join(", "));
                                return ExitCode::FAILURE;
                            }
                            cynic_protocol::node_activation::ActivationProof::RolledBack { rollback_evidence_refs, .. } => {
                                println!("ROLLED BACK: {} (evidence: {})", node_id, rollback_evidence_refs.join(", "));
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        println!("BLOCKED: {}", e);
                        return ExitCode::FAILURE;
                    }
                    None => {
                        println!("BLOCKED: --yes consent required (mutates host)");
                        return ExitCode::FAILURE;
                    }
                }
                ExitCode::SUCCESS
            }
        }
    }
}
