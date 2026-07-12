use std::time::Duration;

use cynic_protocol::{KernelEndpointPolicy, OBSERVE_PATH, kernel_url as build_kernel_url};
use reqwest::Client;
use serde_json::json;

use crate::config::WatchdogConfig;

pub(super) trait WatchdogKernelProbePort {
    async fn probe(&self, cfg: &WatchdogConfig) -> serde_json::Value;
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct LocalWatchdogKernelProbe;

pub(super) fn observation_payload(
    cfg: &WatchdogConfig,
    value: serde_json::Value,
) -> serde_json::Value {
    let status = value
        .get("status")
        .and_then(|status| status.as_str())
        .unwrap_or("degraded")
        .to_string();
    json!({
        "tool": "cynic-node-watchdog",
        "target": cfg.node.name,
        "domain": "fleet.constellation",
        "status": status,
        "context": format!(
            "L0 watchdog evidence for {} in constellation {}",
            cfg.node.name, cfg.node.constellation
        ),
        "agent_id": format!("cynic-node-watchdog-{}", cfg.node.name),
        "tags": ["cynic-node", "watchdog", "constellation", cfg.node.constellation.as_str()],
        "value": value,
        "confidence": "observed",
        "consumer": "organ-fleet",
        "action": "ingest_node_watchdog_evidence",
        "maturity": 1.0,
    })
}

pub(super) async fn post_observation(
    client: &Client,
    cfg: &WatchdogConfig,
    value: serde_json::Value,
) -> Result<(), String> {
    let payload = observation_payload(cfg, value);
    let url = build_kernel_url(
        &cfg.kernel.url,
        OBSERVE_PATH,
        KernelEndpointPolicy {
            allow_loopback: true,
            ..KernelEndpointPolicy::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.kernel.api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("POST /observe returned {status}: {body}"))
    }
}

pub(super) async fn probe_kernel(
    kernel_url: &str,
    api_key: &str,
    timeout_secs: u64,
) -> serde_json::Value {
    let url = match build_kernel_url(
        kernel_url,
        "/health",
        KernelEndpointPolicy {
            allow_loopback: true,
            ..KernelEndpointPolicy::default()
        },
    ) {
        Ok(url) => url,
        Err(error) => {
            return json!({
                "reachable": false,
                "error": error.to_string(),
            });
        }
    };
    let client = match Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return json!({
                "reachable": false,
                "error": format!("client_init_failed: {e}"),
            });
        }
    };
    match client.get(url).bearer_auth(api_key).send().await {
        Ok(resp) => json!({
            "reachable": true,
            "http_status": resp.status().as_u16(),
        }),
        Err(e) => json!({
            "reachable": false,
            "error": e.to_string(),
        }),
    }
}

impl WatchdogKernelProbePort for LocalWatchdogKernelProbe {
    async fn probe(&self, cfg: &WatchdogConfig) -> serde_json::Value {
        probe_kernel(
            &cfg.kernel.url,
            &cfg.kernel.api_key,
            cfg.watchdog.http_timeout_secs,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::observation_payload;
    use crate::config::{KernelConfig, NodeConfig, WatchdogConfig, WatchdogRuntimeConfig};
    use cynic_host::registry::DEFAULT_REGISTRY_PATH;
    use serde_json::json;

    fn config() -> WatchdogConfig {
        WatchdogConfig {
            node: NodeConfig {
                name: "node-a".to_string(),
                role: "kernel_primary".to_string(),
                constellation: "forge".to_string(),
                services: vec!["cynic-kernel".to_string()],
            },
            kernel: KernelConfig {
                url: "http://kernel:3030".to_string(),
                api_key_env: "CYNIC_API_KEY".to_string(),
                api_key: "secret".to_string(),
                heartbeat_interval_secs: 30,
            },
            watchdog: WatchdogRuntimeConfig {
                interval_secs: 30,
                http_timeout_secs: 5,
                registry_path: DEFAULT_REGISTRY_PATH.to_string(),
            },
        }
    }

    #[test]
    fn payload_builder_uses_value_status() {
        let payload = observation_payload(&config(), json!({"status": "ok"}));
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["target"], "node-a");
        assert_eq!(payload["consumer"], "organ-fleet");
    }
}
