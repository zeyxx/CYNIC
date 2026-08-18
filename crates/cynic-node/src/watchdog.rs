#![allow(dead_code)]
//! L0 node watchdog: publish local node evidence without requiring a Dog backend.

mod host_runtime;
mod kernel_bridge;
mod peer_handshake;
mod sampler;

use std::process::ExitCode;
use std::time::Duration;

use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::config::WatchdogConfig;
use crate::{config, shutdown::install_signal_handler};

use kernel_bridge::post_observation;
use sampler::LocalSampler;

pub(crate) async fn execute(config_path: &str) -> ExitCode {
    let cfg = match config::load_watchdog(config_path) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("watchdog config error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let client = match Client::builder()
        .timeout(Duration::from_secs(cfg.watchdog.http_timeout_secs))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::error!("HTTP client init failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let shutdown = CancellationToken::new();
    install_signal_handler(shutdown.clone());
    run_watchdog(&client, cfg, &shutdown).await
}

pub(crate) async fn run_watchdog(
    client: &Client,
    cfg: WatchdogConfig,
    shutdown: &CancellationToken,
) -> ExitCode {
    tracing::info!(
        node = %cfg.node.name,
        role = %cfg.node.role,
        constellation = %cfg.node.constellation,
        interval_secs = cfg.watchdog.interval_secs,
        "L0 watchdog starting"
    );

    let mut sampler = LocalSampler::new();
    let mut interval = tokio::time::interval(Duration::from_secs(cfg.watchdog.interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let observation = sampler.collect(&cfg).await;
                if let Err(e) = post_observation(client, &cfg, observation).await {
                    tracing::warn!(error = %e, "L0 watchdog observation failed");
                }
            }
            _ = shutdown.cancelled() => {
                tracing::info!(node = %cfg.node.name, "L0 watchdog stopped");
                return ExitCode::SUCCESS;
            }
        }
    }
}

#[cfg(test)]
mod tests;
