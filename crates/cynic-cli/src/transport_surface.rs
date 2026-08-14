use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::adapters::local_operator_transport::LocalOperatorTransportObserver;
use cynic_host::transport;

use crate::emit_json;

#[derive(Subcommand)]
pub(crate) enum TransportCommand {
    Matrix,
    SecurityAudit,
    SshInspect {
        #[arg(long)]
        host: String,
    },
}

pub(crate) fn execute_transport(command: &TransportCommand) -> ExitCode {
    match command {
        TransportCommand::Matrix => {
            let observer = LocalOperatorTransportObserver;
            emit_json(&transport::build_transport_matrix_report(&observer), false)
        }
        TransportCommand::SecurityAudit => {
            emit_json(&transport::build_transport_security_report(), false)
        }
        TransportCommand::SshInspect { host } => {
            let observer = LocalOperatorTransportObserver;
            emit_json(&transport::inspect_ssh_target(&observer, host), false)
        }
    }
}
