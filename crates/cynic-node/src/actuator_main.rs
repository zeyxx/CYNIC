<<<<<<< HEAD
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
=======
#![cfg_attr(test, allow(clippy::expect_used))]
#![allow(
    dead_code,
    clippy::needless_borrows_for_generic_args,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    clippy::collapsible_if,
    clippy::implicit_clone,
    clippy::needless_pass_by_value,
    clippy::single_match,
    clippy::unused_self,
    missing_debug_implementations,
    async_fn_in_trait,
    clippy::too_many_arguments,
    clippy::redundant_clone,
    clippy::manual_contains,
    clippy::needless_borrow,
    clippy::print_stdout
)]

mod announce;
mod build_attestation;
mod cli;
mod config;
mod config_watch;
mod dns_activation_status;
mod dns_bundle;
mod dns_contract;
mod dns_plan;
mod network_activation_chain;
mod network_actuator;
mod network_actuator_contract;
mod network_actuator_status;
mod network_command_surface;
mod network_diagnostic_surface;
mod network_evidence;
mod network_host_substrate;
mod network_mesh_apply_plan;
mod network_mesh_bundle;
mod network_mesh_plan;
mod network_repair;
mod network_root_policy;
mod network_runtime_proof;
mod network_runtime_status;
mod network_shadow_proof;
mod network_surface_inventory;
mod observe;
mod provider_fallback_hosts;
mod provider_fallback_status;
mod runtime_supervision;
mod supervise;
mod system_probe;
#[allow(dead_code, unreachable_pub)]
mod verify;
mod watchdog;
mod websocket;

pub mod adapters;
pub mod domain;
pub mod ports;
pub mod usecases;

use clap::{Parser, Subcommand};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
use process_wrap::tokio::ChildWrapper;
use reqwest::Client;
use std::process::ExitCode;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::supervise::Backoff;

#[derive(Debug)]
pub(crate) enum ExitReason {
    Shutdown,
    Crashed,
    Expired,
    Mismatch,
    ConfigChanged,
    Fatal(String),
}

pub(crate) async fn run_lifecycle(
    _client: &Client,
    _cfg: &Config,
    _child: &mut Box<dyn ChildWrapper>,
    _shutdown: &CancellationToken,
    _backoff: &mut Backoff,
    _config_rx: &mut tokio::sync::watch::Receiver<()>,
) -> (ExitReason, Option<String>) {
    unreachable!("runtime supervision is not available in cynic-network-actuator")
}
>>>>>>> github/main

#[derive(Parser)]
#[command(
    name = "cynic-network-actuator",
    about = "Bounded root network actuator"
)]
struct ActuatorCli {
    #[command(subcommand)]
    command: ActuatorCommand,
}

#[derive(Subcommand)]
enum ActuatorCommand {
    Apply {
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> ExitCode {
    let cli = ActuatorCli::parse();
    match cli.command {
        ActuatorCommand::Apply {
            registry,
            profile,
            apply,
<<<<<<< HEAD
        } => cynic_node::execute_network_actuator_apply(&registry, &profile, apply),
=======
        } => {
            let observation = observe::observe_payload(Some(&registry));
            let report = network_actuator::apply_profile(&observation, &profile, apply);
            match serde_json::to_string_pretty(&report) {
                Ok(payload) => {
                    println!("{payload}");
                    if report.status == "completed" {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(error) => {
                    tracing::error!("network actuator apply serialization failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
>>>>>>> github/main
    }
}
