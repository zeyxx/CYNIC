#![allow(dead_code)]
//! L0 node watchdog: publish local node evidence without requiring a Dog backend.

mod host_runtime;
mod kernel_bridge;
pub mod peer_handshake;
mod sampler;

use std::process::ExitCode;
use std::time::Duration;

use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::config::WatchdogConfig;
use crate::{config, shutdown::install_signal_handler};

use crate::usecases::network_watchdog_uc::{ServiceProbeSpec, execute_network_watchdog};
use crate::usecases::notification_outbox_uc::InMemoryNotificationOutboxStore;
use crate::watchdog::peer_handshake::LocalPeerHandshakeObserver;
use cynic_host::domain::network::freebox::HttpFreeboxProbeAdapter;
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

    let wg_observer = LocalPeerHandshakeObserver;
    let freebox_probe = HttpFreeboxProbeAdapter::default();
    let outbox_store = InMemoryNotificationOutboxStore::default();

    let services = vec![
        ServiceProbeSpec {
            service: "forgejo".to_string(),
            node: "forgejo".to_string(),
            endpoint: "10.10.10.101:3000".to_string(),
        },
        ServiceProbeSpec {
            service: "surrealdb".to_string(),
            node: "cynic-lxc-surrealdb".to_string(),
            endpoint: "10.10.10.103:8000".to_string(),
        },
        ServiceProbeSpec {
            service: "kernel".to_string(),
            node: "local-kernel".to_string(),
            endpoint: "192.168.100.2:3030".to_string(),
        },
    ];

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let observation = sampler.collect(&cfg).await;
                if let Err(e) = post_observation(client, &cfg, observation).await {
                    tracing::warn!(error = %e, "L0 watchdog observation failed");
                }

                let now_unix_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                if let Ok(net_report) = execute_network_watchdog(
                    "wg0",
                    &wg_observer,
                    &services,
                    Some("http://192.168.0.254"),
                    Some(&freebox_probe),
                    vec![],
                    &outbox_store,
                    now_unix_ms,
                ) {
                    if net_report.alerts_enqueued_count > 0 {
                        tracing::warn!(
                            status = %net_report.status,
                            alerts = net_report.alerts_enqueued_count,
                            stale_peers = net_report.assessment.wireguard_mesh.stale_peers,
                            "L0 network watchdog alert condition detected"
                        );

                        // Self-healing: if any peer is stale, trigger keepalive pings
                        if net_report.assessment.wireguard_mesh.stale_peers > 0 {
                            trigger_self_healing_keepalives();
                        }
                    }
                }
            }
            _ = shutdown.cancelled() => {
                tracing::info!(node = %cfg.node.name, "L0 watchdog stopped");
                return ExitCode::SUCCESS;
            }
        }
    }
}

/// Send async keepalive ping packets across the overlay to unstick stale handshakes.
fn trigger_self_healing_keepalives() {
    let peers = [
        "10.20.0.1",
        "10.20.0.2",
        "10.20.0.4",
        "10.20.0.5",
        "10.20.0.6",
    ];
    for ip in peers {
        let _ = std::process::Command::new("ping")
            .args(["-c", "1", "-W", "1", ip])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

#[cfg(test)]
mod tests;
