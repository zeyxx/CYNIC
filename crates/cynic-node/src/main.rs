// Binary crate test exemption — same pattern as cynic-kernel lib.rs
#![cfg_attr(test, allow(dead_code, clippy::expect_used))]
#![allow(
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
mod source_change_transaction;
mod supervise;
mod system_probe;
#[allow(dead_code, unreachable_pub)]
mod verify;
mod watchdog;
mod websocket;

// Hexagonal Architecture Modules
pub mod adapters;
pub mod domain;
pub mod ports;
pub mod usecases;

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands, ReleaseCommands};
use crate::usecases::agent_scheduler_step_uc;
use crate::usecases::agent_source_change_uc;
use crate::usecases::agent_workspace_lifecycle_uc;
use crate::usecases::network_surfaces_uc;
use crate::usecases::runtime_artifact_release_uc;
use crate::usecases::source_repository_bootstrap_uc;

// ── Signal handler ────────────────────────────────────────────────────────────

/// Install OS signal handlers and cancel `shutdown` when a signal arrives.
///
/// On Unix: listens for SIGTERM and SIGINT.
/// On Windows: listens for Ctrl-C.
fn install_signal_handler(shutdown: CancellationToken) {
    #[cfg(unix)]
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Err(e) => {
                // WHY: If we cannot install SIGTERM, log the failure but continue.
                // SIGINT (ctrl-c) is still available. The process can still be stopped
                // via SIGKILL if needed. Aborting here would be worse than degraded mode.
                tracing::warn!("SIGTERM handler install failed ({e}) — only SIGINT will work");
                tokio::signal::ctrl_c().await.ok();
            }
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = sigterm.recv() => tracing::info!("SIGTERM received — shutting down"),
                    _ = tokio::signal::ctrl_c() => tracing::info!("SIGINT received — shutting down"),
                }
            }
        }
        shutdown.cancel();
    });

    #[cfg(windows)]
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Ctrl-C received — shutting down");
        shutdown.cancel();
    });
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Observe {
            json: _,
            registry,
            fail_on_warning,
            fail_on_critical,
        }) => {
            return network_surfaces_uc::execute_observe(
                &registry,
                fail_on_warning,
                fail_on_critical,
            );
        }
        Some(Commands::RegistryObservation {
            json: _,
            registry,
            output,
        }) => {
            return network_surfaces_uc::execute_registry_observation(&registry, output.as_deref());
        }
        Some(Commands::Network { command }) => {
            return network_surfaces_uc::execute_network_command(&command);
        }
        Some(Commands::Release { command }) => {
            return match command {
                ReleaseCommands::Reconcile {
                    spec,
                    lifecycle_state,
                    yes,
                    json: _,
                } => runtime_artifact_release_uc::execute_reconcile(
                    &spec,
                    lifecycle_state.as_deref(),
                    yes,
                ),
            };
        }
        Some(Commands::AgentWorkspace {
            request,
            result_file,
            registry,
            state_root,
            kernel_endpoint,
            max_claim_ttl_ms,
            max_retention_ms,
            apply,
            json: _,
        }) => {
            return agent_source_change_uc::execute_agent_source_change(
                &request,
                result_file.as_deref(),
                &registry,
                &state_root,
                &kernel_endpoint,
                max_claim_ttl_ms,
                max_retention_ms,
                apply,
            )
            .await;
        }
        Some(Commands::AgentWorkspaceLifecycle {
            workload_id,
            registry,
            state_root,
            kernel_endpoint,
            agent_id,
            renewal_window_ms,
            renewal_ttl_ms,
            apply,
            json: _,
        }) => {
            return agent_workspace_lifecycle_uc::execute_agent_workspace_lifecycle(
                &workload_id,
                &registry,
                &state_root,
                &kernel_endpoint,
                &agent_id,
                renewal_window_ms,
                renewal_ttl_ms,
                apply,
            )
            .await;
        }
        Some(Commands::AgentSchedulerStep {
            observation,
            state_root,
            json: _,
        }) => {
            return agent_scheduler_step_uc::execute_agent_scheduler_step(
                &observation,
                &state_root,
            );
        }
        Some(Commands::SourceBootstrap {
            node,
            repository,
            source,
            revision,
            registry,
            state_root,
            agent_id,
            kernel_endpoint,
            apply,
            json: _,
        }) => {
            return source_repository_bootstrap_uc::execute_source_repository_bootstrap(
                &node,
                &repository,
                &source,
                &revision,
                &registry,
                &state_root,
                &agent_id,
                &kernel_endpoint,
                apply,
            )
            .await;
        }
        None => {}
    }

    // Automatic Versioning Integration
    let git_hash = option_env!("CYNIC_BUILD_GIT_HASH").unwrap_or("unknown_hash");
    let git_branch = option_env!("CYNIC_BUILD_GIT_BRANCH").unwrap_or("unknown_branch");
    let build_date = option_env!("CYNIC_BUILD_DATE").unwrap_or("unknown_date");

    tracing::info!(
        "cynic-node v{} ({} - {}) starting up at {}",
        env!("CARGO_PKG_VERSION"),
        git_branch,
        git_hash,
        build_date
    );

    if cli.watchdog {
        let Some(config_path) = cli.config.as_deref() else {
            tracing::error!("--config is required for --watchdog");
            return ExitCode::FAILURE;
        };
        let cfg = match config::load_watchdog(config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("watchdog config error: {e}");
                return ExitCode::FAILURE;
            }
        };

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.watchdog.http_timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("HTTP client init failed: {e}");
                return ExitCode::FAILURE;
            }
        };

        let shutdown = CancellationToken::new();
        install_signal_handler(shutdown.clone());
        return watchdog::run_watchdog(&client, cfg, &shutdown).await;
    }

    let Some(config_path) = cli.config.as_deref() else {
        tracing::error!("--config is required unless using the observe command");
        return ExitCode::FAILURE;
    };

    let cfg = match config::load(config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("config error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.health.timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("HTTP client init failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let shutdown = CancellationToken::new();
    install_signal_handler(shutdown.clone());

    runtime_supervision::run_node(&client, config_path, cfg, &shutdown).await
}
