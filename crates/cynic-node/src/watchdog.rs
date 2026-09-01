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
use crate::usecases::network_watchdog_uc::{ServiceProbeSpec, execute_network_watchdog};
use crate::usecases::notification_outbox_uc::InMemoryNotificationOutboxStore;
use crate::watchdog::peer_handshake::LocalPeerHandshakeObserver;
use crate::{config, shutdown::install_signal_handler};
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

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let observation = sampler.collect(&cfg).await;
                if let Err(e) = post_observation(client, &cfg, observation).await {
                    tracing::warn!(error = %e, "L0 watchdog observation failed");
                }

                // Dynamically resolve services and overlay peers from registry SSOT
                let services = resolve_watchdog_services(&cfg.watchdog.registry_path);
                let overlay_peers = resolve_overlay_peer_ips(&cfg.watchdog.registry_path, &cfg.node.name);

                let now_unix_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                if let Ok(net_report) = execute_network_watchdog(
                    "wg0",
                    &wg_observer,
                    &services,
                    None, // Freebox probe endpoint is optional / dynamically discovered
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

                        // Self-healing: if any peer is stale, trigger keepalive pings on dynamic peer IPs
                        if net_report.assessment.wireguard_mesh.stale_peers > 0 {
                            trigger_self_healing_keepalives(&overlay_peers);
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

/// Dynamically resolve probeable services from `infra/registry.json`.
///
/// Iterates over declared nodes and their services, finding the target IP
/// (preferring synapse plane, falling back to management plane) and port.
pub fn resolve_watchdog_services(registry_path: &str) -> Vec<ServiceProbeSpec> {
    let Ok(content) = std::fs::read_to_string(registry_path) else {
        return Vec::new();
    };
    let Ok(registry) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };

    let Some(nodes) = registry.get("nodes").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };

    let mut specs = Vec::new();
    for node in nodes {
        let node_id = node
            .get("id")
            .or_else(|| node.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let net = node.get("network");
        let ip = net
            .and_then(|n| n.get("synapse").or_else(|| n.get("management")))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        if ip.is_empty() {
            continue;
        }

        if let Some(services) = node.get("services").and_then(serde_json::Value::as_array) {
            for svc in services {
                let name = svc
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let port = svc
                    .get("port")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                if !name.is_empty() && port > 0 {
                    specs.push(ServiceProbeSpec {
                        service: name.to_string(),
                        node: node_id.to_string(),
                        endpoint: format!("{ip}:{port}"),
                    });
                }
            }
        }
    }

    specs
}

/// Dynamically resolve declared WireGuard overlay peer IPs from `infra/registry.json`.
pub fn resolve_overlay_peer_ips(registry_path: &str, local_node_id: &str) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(registry_path) else {
        return Vec::new();
    };
    let Ok(registry) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };

    let Some(nodes) = registry
        .pointer("/network_policy/wireguard/nodes")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };

    nodes
        .iter()
        .filter_map(|n| {
            let node_id = n
                .get("node_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let overlay_ip = n
                .get("overlay_ip")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if !overlay_ip.is_empty() && node_id != local_node_id {
                Some(overlay_ip.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Send async keepalive ping packets across dynamically resolved peer IPs.
fn trigger_self_healing_keepalives(peer_ips: &[String]) {
    for ip in peer_ips {
        let _ = std::process::Command::new("ping")
            .args(["-c", "1", "-W", "1", ip])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

#[cfg(test)]
mod tests;
