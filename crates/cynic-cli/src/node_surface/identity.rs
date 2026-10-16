//! Node identity commands — observe or create the node-local Ed25519 identity.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

use clap::Subcommand;

pub(crate) fn execute_node_identity(
    node_id: &str,
    signing_key_path: &str,
    apply: bool,
    _json: bool,
) -> ExitCode {
    cynic_node::usecases::node_identity_uc::execute_node_identity(
        node_id,
        signing_key_path,
        apply,
    )
}

/// Node identity arms: initialize/display the keypair, or observe/create the
/// node-local identity record.
#[derive(Subcommand)]
pub enum NodeIdentityCommand {
    /// Initialize or display the local node Ed25519 signing keypair.
    Init {
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Observe or create the node-local Ed25519 identity used for machine enrollment.
    Node {
        #[arg(long)]
        node_id: String,
        #[arg(long, default_value = "/var/lib/cynic/identity/node-signing.key")]
        signing_key_path: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_node_identity_command(command: NodeIdentityCommand) -> ExitCode {
    match command {
        NodeIdentityCommand::Init { key_path, json } => {
            super::activation::execute_identity_init(key_path, json)
        }
        NodeIdentityCommand::Node {
            node_id,
            signing_key_path,
            apply,
            json,
        } => execute_node_identity(&node_id, &signing_key_path, apply, json),
    }
}
