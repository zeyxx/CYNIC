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
    /// Full SSH configuration supervision: parse ~/.ssh/config, correlate
    /// with declared policy aliases, check key presence/permissions,
    /// and report connectivity state.
    SshSupervision {
        /// Explicit ssh config path (default: ~/.ssh/config)
        #[arg(long)]
        config: Option<String>,
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
        TransportCommand::SshSupervision { config } => {
            let report = match config {
                Some(path) => {
                    transport::build_ssh_local_supervision_report_from(std::path::Path::new(path))
                }
                None => transport::build_ssh_local_supervision_report(),
            };
            let code = if report.status == "converged" { 0 } else { 1 };
            crate::cli_json::emit_pretty_json(&report);
            ExitCode::from(code as u8)
        }
    }
}
