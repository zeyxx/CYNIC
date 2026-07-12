use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::transport;

use crate::emit_json;

#[derive(Subcommand)]
pub(crate) enum TransportCommand {
    Matrix,
}

pub(crate) fn execute_transport(command: TransportCommand) -> ExitCode {
    match command {
        TransportCommand::Matrix => emit_json(&transport::build_transport_matrix_report(), false),
    }
}
