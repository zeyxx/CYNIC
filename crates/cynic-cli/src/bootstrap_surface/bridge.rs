use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::bridge as host_exec_bridge;

use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum BridgeCommand {
    Bundle {
        #[arg(long)]
        output_dir: String,
    },
    Plan {
        #[arg(long, default_value = "/tmp/cynic-host-exec")]
        bundle_dir: String,
    },
    Install {
        #[arg(long, default_value = "/tmp/cynic-host-exec")]
        bundle_dir: String,
        #[arg(long)]
        apply: bool,
    },
    RootlessSmoke {
        #[arg(long, default_value = "/tmp/cynic-host-exec-rootless")]
        sandbox_root: String,
        #[arg(long)]
        apply: bool,
    },
    Status,
}

pub(crate) fn execute_bridge(command: BridgeCommand) -> ExitCode {
    match command {
        BridgeCommand::Bundle { output_dir } => {
            match host_exec_bridge::write_bridge_bundle(output_dir) {
                Ok(report) => emit_json(&report, false),
                Err(error) => fail(error.to_string()),
            }
        }
        BridgeCommand::Plan { bundle_dir } => {
            emit_json(&host_exec_bridge::bridge_activation_plan(bundle_dir), false)
        }
        BridgeCommand::Install { bundle_dir, apply } => {
            let report = host_exec_bridge::bridge_install_report(bundle_dir, apply);
            emit_json(&report, report.status != "completed")
        }
        BridgeCommand::RootlessSmoke {
            sandbox_root,
            apply,
        } => match host_exec_bridge::bridge_rootless_smoke(sandbox_root, apply) {
            Ok(report) => emit_json(&report, report.status != "completed"),
            Err(error) => fail(error.to_string()),
        },
        BridgeCommand::Status => {
            let report = host_exec_bridge::bridge_status();
            emit_json(&report, report.status != "active")
        }
    }
}
