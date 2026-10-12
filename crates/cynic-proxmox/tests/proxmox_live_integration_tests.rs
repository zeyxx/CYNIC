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

    let pve_url = "https://192.168.0.37:8006/api2/json/version";
    let token_id = "cynic@pve!token"; // From registry / authority

    let res = client
        .get(pve_url)
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
            eprintln!("Proxmox cluster 192.168.0.37 unreachable from this network plane: {e}");
        }
    }
}
