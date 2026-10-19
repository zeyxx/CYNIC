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

use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use cynic_node::config;
use cynic_node::node_control_runtime;
use cynic_node::runtime_supervision;
use cynic_node::shutdown::install_signal_handler;
use cynic_node::watchdog;
use cynic_types::paths::CynicPaths;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // The daemon owns exactly one subcommand: `control`. Every other
    // subcommand is a CLI surface that belongs in `cynic`, not in a daemon.
    // The systemd unit runs `cynic-node control --config ...`.
    //
    // We do NOT parse the full Cli here. Cli is a CLI type that now lives in
    // cynic-cli. cynic-node cannot depend on cynic-cli (DAG inversion).
    // Instead we check the first argument directly.
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] != "control" {
        eprintln!(
            "cynic-node is a daemon and only owns `control`. \
             Use `cynic <command>` for CLI surfaces."
        );
        return ExitCode::FAILURE;
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

    // Extract --config from args (after `control` if present)
    let config_path = args
        .iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            CynicPaths::system_config_dir()
                .join("node-control.toml")
                .display()
                .to_string()
        });

    // The daemon owns only `control`, and `control` consumes only
    // [node_control]. Parse narrowly: the full Config requires the
    // supervision sections ([kernel]/[dog]/[process]) that control never
    // reads, which fails closed on hosts that carry no dog (e.g. builder).
    if args.len() > 1 && args[1] == "control" {
        return control_main(&config_path).await;
    }

    if args.len() > 1 && args[1] == "--watchdog" {
        return watchdog::execute(&config_path).await;
    }

    let cfg = match config::load(&config_path) {
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

    let exit = runtime_supervision::run_node(&client, &config_path, cfg, &shutdown).await;
    shutdown.cancel();
    for task in node_tasks {
        let _ = task.await;
    }
    exit
}

/// `control` entry mode: scheduler tasks only, no dog/process supervision.
///
/// Loads only [node_control] (narrow loader), starts the control tasks,
/// then parks until shutdown. Supervision (`run_node`) is deliberately not
/// started here: it requires [kernel]/[dog]/[process], which control-mode
/// hosts (builder) do not carry.
async fn control_main(config_path: &str) -> ExitCode {
    let shutdown = CancellationToken::new();
    install_signal_handler(shutdown.clone());
    let control = match config::load_node_control(config_path) {
        Ok(control) => control,
        Err(error) => {
            tracing::error!("control config error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let tasks = match node_control_runtime::start_tasks(control, shutdown.clone()).await {
        Ok(tasks) => tasks,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };
    shutdown.cancelled().await;
    shutdown.cancel();
    for task in tasks {
        let _ = task.await;
    }
    ExitCode::SUCCESS
}
