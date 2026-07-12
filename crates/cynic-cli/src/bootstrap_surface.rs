use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::bootstrap_chain;
use cynic_host::domain::shared::SurfaceStatus;

use crate::{emit_json, fail};

mod bridge;
mod constellation;
mod framework;
mod host;
mod host_e2e;
pub(crate) mod selection;

pub(crate) use selection::{
    ProfileArgs, observed_blocked, selected_host_context, selected_profile, selected_resolution,
};

use bridge::{BridgeCommand, execute_bridge};
use constellation::execute_constellation;
use framework::{BootstrapFrameworkCommand, execute_framework};
use host::{BootstrapHostCommand, execute_bootstrap_host};

#[derive(Subcommand)]
pub(crate) enum BootstrapCommand {
    Chain(ProfileArgs),
    Constellation {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    Bridge {
        #[command(subcommand)]
        command: BridgeCommand,
    },
    Framework {
        #[command(subcommand)]
        command: BootstrapFrameworkCommand,
    },
    Host {
        #[command(subcommand)]
        command: BootstrapHostCommand,
    },
}

pub(crate) fn execute_bootstrap(command: BootstrapCommand) -> ExitCode {
    match command {
        BootstrapCommand::Chain(args) => {
            let selected = match selected_profile(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let resolution = selected_resolution(&args);
            match bootstrap_chain::build_bootstrap_chain_report(selected, resolution) {
                Ok(report) => emit_json(&report, report.status != SurfaceStatus::Active),
                Err(error) => fail(error.to_string()),
            }
        }
        BootstrapCommand::Constellation { json, registry } => execute_constellation(json, registry),
        BootstrapCommand::Bridge { command } => execute_bridge(command),
        BootstrapCommand::Framework { command } => execute_framework(command),
        BootstrapCommand::Host { command } => execute_bootstrap_host(command),
    }
}
