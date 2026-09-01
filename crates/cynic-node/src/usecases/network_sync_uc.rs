//! Network Synchronization Use Case (Gap K1 / IaC convergence).
//!
//! Reconciles local WireGuard peers and Synapse DNS table from the declarative
//! SSOT (`infra/registry.json`).
//!
//! Follows the CYNIC Coherence Pattern:
use serde::{Deserialize, Serialize};
use std::process::Command;

pub const NETWORK_SYNC_REPORT_SCHEMA: &str = "cynic.network.sync_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireguardPeerSpec {
    pub node_id: String,
    pub overlay_ip: String,
    pub public_key: String,
    pub endpoint: String,
    pub keepalive_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsRecordSpec {
    pub ip: String,
    pub hostnames: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSyncPlan {
    pub local_node_id: String,
    pub overlay_ip: String,
    pub wireguard_interface: String,
    pub declared_peers: Vec<WireguardPeerSpec>,
    pub dns_records: Vec<DnsRecordSpec>,
    pub peers_to_add: Vec<String>,
    pub peers_to_remove: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSyncReport {
    pub schema: String,
    pub status: String,
    pub apply: bool,
    pub local_node_id: String,
    pub peers_synced: usize,
    pub dns_records_synced: usize,
    pub active_handshakes_count: usize,
    pub blockers: Vec<String>,
}

/// Pure parser extracting WireGuard and DNS specs from a registry JSON Value.
pub fn parse_registry_network_specs(
    registry: &serde_json::Value,
    local_node_id: &str,
) -> Result<(Vec<WireguardPeerSpec>, Vec<DnsRecordSpec>), String> {
    let nodes = registry
        .pointer("/network_policy/wireguard/nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "missing_network_policy_wireguard_nodes".to_string())?;

    let mut peers = Vec::new();
    for node in nodes {
        let node_id = node
            .get("node_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if node_id.is_empty() || node_id == local_node_id {
            continue; // skip self
        }
        let overlay_ip = node
            .get("overlay_ip")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let public_key = node
            .get("public_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let endpoint = node
            .get("endpoint")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        if !overlay_ip.is_empty() && !public_key.is_empty() {
            peers.push(WireguardPeerSpec {
                node_id: node_id.to_string(),
                overlay_ip: overlay_ip.to_string(),
                public_key: public_key.to_string(),
                endpoint: endpoint.to_string(),
                keepalive_secs: 25,
            });
        }
    }

    // Parse DNS records from synapse reservations
    let reservations = registry
        .pointer("/network_policy/reservations")
        .and_then(serde_json::Value::as_array);

    let mut dns_records = Vec::new();
    if let Some(res_arr) = reservations {
        for res in res_arr {
            let ip = res
                .get("synapse")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let name = res
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let mut hostnames = vec![name.to_string(), format!("{name}.synapse.cynic")];
            if let Some(dns_arr) = res.get("dns").and_then(serde_json::Value::as_array) {
                for d in dns_arr {
                    if let Some(s) = d.as_str() {
                        if !hostnames.contains(&s.to_string()) {
                            hostnames.push(s.to_string());
                        }
                    }
                }
            }
            if !ip.is_empty() {
                dns_records.push(DnsRecordSpec {
                    ip: ip.to_string(),
                    hostnames,
                });
            }
        }
    }

    Ok((peers, dns_records))
}

/// Execute network sync on the current host.
pub fn execute_network_sync(
    registry_path: &str,
    local_node_id: &str,
    iface: &str,
    apply: bool,
) -> Result<NetworkSyncReport, String> {
    let reg_content =
        std::fs::read_to_string(registry_path).map_err(|e| format!("cannot_read_registry:{e}"))?;
    let registry: serde_json::Value =
        serde_json::from_str(&reg_content).map_err(|e| format!("invalid_registry_json:{e}"))?;

    let (peers, dns_records) = parse_registry_network_specs(&registry, local_node_id)?;
    let mut blockers = Vec::new();
    let mut peers_synced = 0;
    let dns_records_synced = dns_records.len();

    if apply {
        // 1. Sync WireGuard peers dynamically
        for peer in &peers {
            let mut cmd = Command::new("sudo");
            cmd.args([
                "-n",
                "wg",
                "set",
                iface,
                "peer",
                &peer.public_key,
                "allowed-ips",
                &format!("{}/32", peer.overlay_ip),
                "persistent-keepalive",
                &peer.keepalive_secs.to_string(),
            ]);
            if !peer.endpoint.is_empty() {
                cmd.args(["endpoint", &peer.endpoint]);
            }
            match cmd.output() {
                Ok(out) if out.status.success() => {
                    peers_synced += 1;
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    blockers.push(format!("wg_set_peer_failed:{}:{err}", peer.node_id));
                }
                Err(e) => {
                    blockers.push(format!("wg_cmd_failed:{}:{e}", peer.node_id));
                }
            }
        }

        // 2. Render synapse hosts file
        let mut hosts_content = String::from("# AUTO-GENERATED BY CYNIC NETWORK SYNC\n");
        for record in &dns_records {
            hosts_content.push_str(&format!("{}\t{}\n", record.ip, record.hostnames.join(" ")));
        }
        let _ = std::fs::create_dir_all("/etc/cynic/dns");
        if let Err(e) = std::fs::write("/etc/cynic/dns/synapse.hosts", &hosts_content) {
            blockers.push(format!("write_synapse_hosts_failed:{e}"));
        }
    } else {
        peers_synced = peers.len();
    }

    let status = if !blockers.is_empty() {
        "degraded".to_string()
    } else if apply {
        "converged".to_string()
    } else {
        "dry_run".to_string()
    };

    Ok(NetworkSyncReport {
        schema: NETWORK_SYNC_REPORT_SCHEMA.to_string(),
        status,
        apply,
        local_node_id: local_node_id.to_string(),
        peers_synced,
        dns_records_synced,
        active_handshakes_count: peers_synced,
        blockers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_registry_network_specs_extracts_peers_and_skips_self() {
        let reg = json!({
            "network_policy": {
                "wireguard": {
                    "nodes": [
                        {
                            "node_id": "cortex",
                            "overlay_ip": "10.20.0.3",
                            "public_key": "key-cortex",
                            "endpoint": "192.168.100.2:44196"
                        },
                        {
                            "node_id": "agent-host",
                            "overlay_ip": "10.20.0.6",
                            "public_key": "key-agent-host",
                            "endpoint": "10.10.10.102:44196"
                        }
                    ]
                },
                "reservations": [
                    {
                        "name": "agent-host",
                        "synapse": "10.10.10.102",
                        "dns": ["agent-host.synapse.cynic"]
                    }
                ]
            }
        });

        let (peers, dns) = parse_registry_network_specs(&reg, "cortex").unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "agent-host");
        assert_eq!(peers[0].overlay_ip, "10.20.0.6");
        assert_eq!(dns.len(), 1);
        assert_eq!(dns[0].ip, "10.10.10.102");
    }
}
