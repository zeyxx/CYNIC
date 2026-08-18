//! Build CLI Surface (`cynic build`).
//!
//! Thin facade over the `BuildRunnerPort`: an agent (or operator) triggers a
//! distributed build on the network runner (10.10.10.119) for an exact source
//! commit and target phase. No local `cargo` ad hoc, no stack hacks — the
//! runner owns the toolchain dispatch and emits a signed proof bundle.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_node::ports::build_runner::{BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode};
use cynic_node::ports::build_runner_adapter::{LocalCommandRunner, SshBuildRunner};

#[derive(Subcommand)]
pub(crate) enum BuildCommand {
    /// Trigger a distributed build on the network runner for an exact commit.
    Trigger {
        /// Exact source commit SHA (40 hex chars) to materialize on the runner.
        #[arg(long)]
        commit: String,
        /// Target build phase: check | build | activate.
        #[arg(long, default_value = "check")]
        phase: String,
        /// Network mode for the build: offline | online.
        #[arg(long, default_value = "offline")]
        network_mode: String,
        /// Build runner SSH host (defaults to the canonical network runner).
        #[arg(long, default_value = "10.10.10.119")]
        host: String,
        /// Build runner SSH user.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON proof instead of a human summary.
        #[arg(long)]
        json: bool,
    },
    /// Observe the network build runner health (reachability, toolchain).
    Observe {
        /// Build runner SSH host.
        #[arg(long, default_value = "10.10.10.119")]
        host: String,
        /// Build runner SSH user.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON instead of a human summary.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_build(command: BuildCommand) -> ExitCode {
    match command {
        BuildCommand::Trigger {
            commit,
            phase,
            network_mode,
            host,
            user,
            json,
        } => {
            let phase = match parse_phase(&phase) {
                Some(p) => p,
                None => {
                    eprintln!("Invalid --phase '{phase}' (expected check|build|activate)");
                    return ExitCode::FAILURE;
                }
            };
            let network_mode = match parse_network_mode(&network_mode) {
                Some(m) => m,
                None => {
                    eprintln!("Invalid --network-mode '{network_mode}' (expected offline|online)");
                    return ExitCode::FAILURE;
                }
            };
            let trigger = BuildTrigger {
                source_commit: commit,
                phase,
                network_mode,
            };
            let runner = SshBuildRunner {
                host,
                user,
                runner: Box::new(LocalCommandRunner),
            };
            match runner.trigger(&trigger) {
                Ok(proof) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&proof).unwrap_or_default());
                    } else {
                        println!(
                            "build {} -> status={} layers={}",
                            proof.source_commit,
                            proof.status,
                            proof.layers.len()
                        );
                        if let Some(broken) = &proof.broken_layer {
                            eprintln!("broken_layer: {broken}");
                        }
                    }
                    if proof.status.is_converged() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("build trigger error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        BuildCommand::Observe {
            host,
            user,
            json,
        } => {
            let runner = SshBuildRunner {
                host,
                user,
                runner: Box::new(LocalCommandRunner),
            };
            let health = runner.observe_runner();
            if json {
                println!("{}", serde_json::to_string_pretty(&health).unwrap_or_default());
            } else {
                println!(
                    "runner {} reachable={} toolchain={}",
                    health.node_id, health.reachable, health.toolchain
                );
            }
            if health.reachable {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn parse_phase(s: &str) -> Option<BuildPhase> {
    match s.to_lowercase().as_str() {
        "check" => Some(BuildPhase::Check),
        "build" => Some(BuildPhase::Build),
        "activate" => Some(BuildPhase::Activate),
        _ => None,
    }
}

fn parse_network_mode(s: &str) -> Option<NetworkMode> {
    match s.to_lowercase().as_str() {
        "offline" => Some(NetworkMode::Offline),
        "online" => Some(NetworkMode::Online),
        _ => None,
    }
}
