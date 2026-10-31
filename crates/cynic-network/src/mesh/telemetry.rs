//! Pure Network Health Telemetry for CYNIC
//! Zero policy, zero alerts: produces factual snapshots of ports, handshakes, and drops.
//! Analysis and threat classification are delegated to cynic-intel (SOC layer).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NETWORK_TELEMETRY_SNAPSHOT_SCHEMA: &str = "cynic.network.telemetry_snapshot.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireguardPeerMetrics {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub last_handshake_unix_ms: Option<u64>,
    pub transfer_rx_bytes: u64,
    pub transfer_tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTelemetrySnapshot {
    pub schema: String,
    pub node_id: String,
    pub listening_ports: Vec<u16>,
    pub wireguard_peers: BTreeMap<String, WireguardPeerMetrics>,
    pub dropped_packets_count: u64,
    pub observed_at_unix_ms: u64,
}
