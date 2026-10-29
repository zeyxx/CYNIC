//! Sovereign daemon execution surface — kernel & node daemons in the single binary.

use clap::Subcommand;
use cynic_types::paths::CynicPaths;
use std::process::ExitCode;

use cynic_host::ports::HostServicePort;
use cynic_host::service_supervisor_observation::observe_service_supervisor;

use crate::host_profile_selection::{ProfileArgs, selected_host_context};
use crate::{emit_json, fail};

#[derive(Subcommand, Debug, Clone)]
pub enum DaemonCommand {
    /// Run the CYNIC Sovereign Kernel daemon (long-running).
    Kernel {
        #[arg(long)]
        reset: bool,
    },
    /// Run the CYNIC Node Control daemon (long-running).
    Node {
        #[arg(long, default_value_t = CynicPaths::system_config_dir().join("node-control.toml").display().to_string())]
        config: String,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        watchdog: bool,
    },
    /// Observe the supervised backend daemons for the resolved host profile (read-only).
    Observe {
        #[command(flatten)]
        profile: ProfileArgs,
    },
}

pub async fn dispatch(cmd: DaemonCommand) -> ExitCode {
    match cmd {
        DaemonCommand::Kernel { reset: _ } => {
            eprintln!("cynic-kernel is retired; use 'cynic daemon node' for sovereign runtime supervision");
            ExitCode::from(0)
        }
        DaemonCommand::Node {
            config,
            check,
            watchdog,
        } => {
            if watchdog {
                return cynic_node::watchdog::execute(&config).await;
            }

            cynic_node::node_control_runtime::run(&config, check).await
        }
        DaemonCommand::Observe { profile } => {
            let (host_profile, _) = match selected_host_context(&profile) {
                Ok(ctx) => ctx,
                Err(e) => return fail(e),
            };
            let policy = host_profile.desired_services();
            let observation =
                observe_service_supervisor("local-host-substrate".to_string(), &policy);
            emit_json(&observation, profile.json)
        }
    }
}
