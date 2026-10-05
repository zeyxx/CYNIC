//! Thin CLI facade for CynicPeer commands.
//!
//! Conforms to `contracts/framework.json` §472 (cli_is_adapter).
//! ZÉRO policy, ZÉRO logic: parse -> delegate to cynic-node.

use cynic_node::peer_commands::PeerCommands;
use std::process::ExitCode;

pub(crate) fn execute_peer(command: PeerCommands) -> ExitCode {
    cynic_node::execute_peer_command(&command)
}
