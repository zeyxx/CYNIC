use std::time::Duration;

use reqwest::Client;
use serde_json::json;

use crate::config::WatchdogConfig;

pub(super) async fn post_observation(
    client: &Client,
    cfg: &WatchdogConfig,
    value: serde_json::Value,
) -> Result<(), String> {
    let status = value
        .get("status")
        .and_then(|status| status.as_str())
        .unwrap_or("degraded")
        .to_string();
    let payload = json!({
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
    });
    let url = format!("{}/observe", cfg.kernel.url.trim_end_matches('/'));
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
    let url = format!("{}/health", kernel_url.trim_end_matches('/'));
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
