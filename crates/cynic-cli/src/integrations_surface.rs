use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use cynic_host::adapters::local_integration::{apply_local_integration, connect_local_integration};

use crate::integrations_readonly_surface::{
    execute_integrations_plan, execute_integrations_status,
};
use crate::integrations_surface_render::{
    print_integration_adopt_human, print_integration_connect_human,
};

#[derive(Subcommand)]
pub(crate) enum IntegrationsCommand {
    /// Observe managed services and external CLI connection state without secrets.
    Status {
        /// Emit the machine-readable proof report.
        #[arg(long)]
        json: bool,
    },
    /// Build the read-only adoption plan for managed services and provider CLIs.
    Plan {
        /// Emit the machine-readable proof report.
        #[arg(long)]
        json: bool,
    },
    /// Apply a concrete integration adoption recipe where CYNIC has one.
    Adopt {
        /// Integration id to adopt (`huggingface`, `github`).
        id: String,
        /// Execute the adoption plan. Without this flag the report remains read-only.
        #[arg(long)]
        apply: bool,
        /// Emit the machine-readable proof report.
        #[arg(long)]
        json: bool,
    },
    /// Connect an already-present integration without treating installation as connection.
    Connect {
        /// Integration id to connect (`huggingface`, `github`).
        id: String,
        /// Execute the connection boundary. Without this flag the report remains read-only.
        #[arg(long)]
        apply: bool,
        /// Emit the machine-readable proof report.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_integrations(command: IntegrationsCommand) -> ExitCode {
    match command {
        IntegrationsCommand::Status { json } => execute_integrations_status(json),
        IntegrationsCommand::Plan { json } => execute_integrations_plan(json),
        IntegrationsCommand::Adopt { id, apply, json } => match apply_local_integration(&id, apply)
        {
            Ok(report) => {
                if json {
                    return if report.status == "failed" || report.status == "blocked" {
                        let code = emit_pretty_json(&report);
                        if code == ExitCode::SUCCESS {
                            ExitCode::FAILURE
                        } else {
                            code
                        }
                    } else {
                        emit_pretty_json(&report)
                    };
                } else {
                    print_integration_adopt_human(&report);
                }
                if report.status == "failed" || report.status == "blocked" {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        IntegrationsCommand::Connect { id, apply, json } => {
            match connect_local_integration(&id, apply) {
                Ok(report) => {
                    if json {
                        return if report.status == "completed" || report.status == "planned" {
                            emit_pretty_json(&report)
                        } else {
                            let code = emit_pretty_json(&report);
                            if code == ExitCode::SUCCESS {
                                ExitCode::FAILURE
                            } else {
                                code
                            }
                        };
                    } else {
                        print_integration_connect_human(&report);
                    }
                    if report.status == "completed" || report.status == "planned" {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
