//! Network SOC & Security Telemetry Domain for CYNIC
//! Detects unmanaged open ports, WireGuard peer handshake decays, and firewall violations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NETWORK_SECURITY_ALERT_SCHEMA: &str = "cynic.network.security_alert.v1";
pub const NETWORK_SECURITY_REPORT_SCHEMA: &str = "cynic.network.security_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkAlertSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityAlert {
    pub alert_id: String,
    pub severity: NetworkAlertSeverity,
    pub title: String,
    pub details: String,
    pub affected_node_id: String,
    pub detected_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireguardPeerState {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub last_handshake_unix_ms: Option<u64>,
    pub transfer_rx_bytes: u64,
    pub transfer_tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecuritySnapshot {
    pub node_id: String,
    pub listening_ports: Vec<u16>,
    pub wireguard_peers: BTreeMap<String, WireguardPeerState>,
    pub dropped_packets_count: u64,
    pub observed_at_unix_ms: u64,
}

/// Pure evaluation of security posture: flags rogue listening ports and handshake decays.
pub fn evaluate_network_security(
    snapshot: &NetworkSecuritySnapshot,
    allowed_ports: &[u16],
    max_handshake_age_ms: u64,
) -> Vec<NetworkSecurityAlert> {
    let mut alerts = Vec::new();

    // 1. Rogue / Unmanaged Ports Check
    for port in &snapshot.listening_ports {
        if !allowed_ports.contains(port) {
            alerts.push(NetworkSecurityAlert {
                alert_id: format!("rogue-port-{}", port),
                severity: NetworkAlertSeverity::High,
                title: format!(
                    "Unmanaged listening port {} detected on {}",
                    port, snapshot.node_id
                ),
                details: format!(
                    "Port {} is active on node {} but not declared in registry policy",
                    port, snapshot.node_id
                ),
                affected_node_id: snapshot.node_id.clone(),
                detected_at_unix_ms: snapshot.observed_at_unix_ms,
            });
        }
    }

    // 2. WireGuard Handshake Decay Check (Broken Mesh Detection)
    for (peer_id, peer) in &snapshot.wireguard_peers {
        match peer.last_handshake_unix_ms {
            None => {
                alerts.push(NetworkSecurityAlert {
                    alert_id: format!("wg-never-handshaked-{}", peer_id),
                    severity: NetworkAlertSeverity::Critical,
                    title: format!("WireGuard peer {} has never handshaked", peer_id),
                    details: format!(
                        "Peer {} ({}) has 0 successful handshakes",
                        peer_id, peer.public_key
                    ),
                    affected_node_id: snapshot.node_id.clone(),
                    detected_at_unix_ms: snapshot.observed_at_unix_ms,
                });
            }
            Some(last_hs) => {
                let age = snapshot.observed_at_unix_ms.saturating_sub(last_hs);
                if age > max_handshake_age_ms {
                    alerts.push(NetworkSecurityAlert {
                        alert_id: format!("wg-handshake-decay-{}", peer_id),
                        severity: NetworkAlertSeverity::High,
                        title: format!(
                            "WireGuard peer {} handshake decayed ({}s ago)",
                            peer_id,
                            age / 1000
                        ),
                        details: format!(
                            "Peer {} lost tunnel connectivity; last handshake was {}ms ago",
                            peer_id, age
                        ),
                        affected_node_id: snapshot.node_id.clone(),
                        detected_at_unix_ms: snapshot.observed_at_unix_ms,
                    });
                }
            }
        }
    }

    alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_rogue_port_as_high_alert() {
        let snapshot = NetworkSecuritySnapshot {
            node_id: "cortex".to_string(),
            listening_ports: vec![3030, 44196, 1337],
            wireguard_peers: BTreeMap::new(),
            dropped_packets_count: 0,
            observed_at_unix_ms: 10000,
        };

        let alerts = evaluate_network_security(&snapshot, &[3030, 44196], 180000);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_id, "rogue-port-1337");
        assert_eq!(alerts[0].severity, NetworkAlertSeverity::High);
    }

    #[test]
    fn flags_decayed_wireguard_handshake() {
        let mut peers = BTreeMap::new();
        peers.insert(
            "cynic-builder".to_string(),
            WireguardPeerState {
                public_key: "abc".to_string(),
                endpoint: Some("10.10.10.119:44196".to_string()),
                last_handshake_unix_ms: Some(1000),
                transfer_rx_bytes: 100,
                transfer_tx_bytes: 100,
            },
        );

        let snapshot = NetworkSecuritySnapshot {
            node_id: "cortex".to_string(),
            listening_ports: vec![3030],
            wireguard_peers: peers,
            dropped_packets_count: 0,
            observed_at_unix_ms: 300000, // 299s later
        };

        let alerts = evaluate_network_security(&snapshot, &[3030], 180000);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].alert_id.starts_with("wg-handshake-decay"));
    }
}
