// Binary crate test exemption — same pattern as cynic-kernel lib.rs
#![allow(dead_code)]
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

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use cynic_node::cli::Cli;
use cynic_node::cli_dispatch;
use cynic_node::config;
use cynic_node::node_control_runtime;
use cynic_node::runtime_supervision;
use cynic_node::shutdown::install_signal_handler;
use cynic_node::watchdog;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return cli_dispatch::dispatch_command(cmd).await;
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
        return watchdog::execute(config_path).await;
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

    let node_tasks = if let Some(control) = cfg.node_control.clone() {
        match node_control_runtime::start_tasks(control, shutdown.clone()).await {
            Ok(tasks) => tasks,
            Err(error) => {
                tracing::error!("{error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        tracing::warn!("node_control is not configured; desired-state reconciliation is inactive");
        Vec::new()
    };

    let exit = runtime_supervision::run_node(&client, config_path, cfg, &shutdown).await;
    shutdown.cancel();
    for task in node_tasks {
        let _ = task.await;
    }
    exit
}
