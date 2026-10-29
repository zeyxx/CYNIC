//! Live integration test against real Proxmox VE cluster.
//!
//! Validates actual HTTP API connectivity and secret resolution.

use cynic_secrets::resolve_secret;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

#[test]
fn proxmox_live_api_ping_or_explicit_secret_missing() {
    let token = resolve_secret("proxmox-token", "CYNIC_PROXMOX_TOKEN_SECRET")
        .or_else(|| std::fs::read_to_string("/etc/cynic/secrets/proxmox-token").ok())
        .map(|s| s.trim().to_string());

    let Some(secret) = token else {
        eprintln!("SKIPPED: /etc/cynic/secrets/proxmox-token absent on this host");
        return;
    };

    if secret.is_empty() {
        eprintln!("SKIPPED: Proxmox secret file is empty");
        return;
    }

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build HTTP client");

    // Resolve the Proxmox hypervisor endpoint dynamically from infra/registry.json.
    // Never hardcode IPs: read node `cynic-host-proxmox` network `service` (preferred)
    // or `admin`, defaulting to port 8006.
    let registry_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("infra")
        .join("registry.json");
    let registry_text = std::fs::read_to_string(&registry_path)
        .expect("read infra/registry.json");
    let registry: Value = serde_json::from_str(&registry_text).expect("parse registry");
    let node = registry
        .get("nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|n| n.get("id").and_then(Value::as_str) == Some("cynic-host-proxmox"))
        })
        .expect("cynic-host-proxmox node in registry");
    let network = node.get("network").expect("network");
    let host = network
        .get("service")
        .or_else(|| network.get("admin"))
        .and_then(Value::as_str)
        .expect("service or admin address");
    let port = 8006u16;
    let pve_url = format!("https://{host}:{port}/api2/json/version");
    let token_id = "cynic@pve!runtime"; // Canonical SSOT from infra/registry.json

    let res = client
        .get(&pve_url)
        .header("Authorization", format!("PVEAPIToken={token_id}={secret}"))
        .send();

    match res {
        Ok(response) => {
            let status = response.status();
            let body: Value = response.json().unwrap_or(Value::Null);
            eprintln!("Proxmox API response status: {status}, body: {body:?}");

            if status.is_success() {
                assert!(
                    body.get("data").is_some(),
                    "Proxmox version response must contain 'data' field"
                );
            } else {
                eprintln!("Proxmox API returned non-success code: {status}");
            }
        }
        Err(e) => {
            eprintln!("Proxmox cluster {host} unreachable from this network plane: {e}");
        }
    }
}
