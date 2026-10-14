//! Node identity commands — observe or create the node-local Ed25519 identity.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

pub(crate) fn execute_node_identity(
    node_id: &str,
    signing_key_path: &str,
    apply: bool,
    _json: bool,
) -> ExitCode {
    cynic_node::usecases::node_identity_uc::execute_node_identity(node_id, signing_key_path, apply)
}
