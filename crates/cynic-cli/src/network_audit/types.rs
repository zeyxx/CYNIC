use serde::{Deserialize, Serialize};

pub const NETWORK_AUDIT_SCHEMA: &str = "cynic.cli.network_audit.v1";

/// Complete network audit report: declared (registry) vs observed (runtime).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NetworkAuditReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<NodeAudit>,
    pub wireguard: WireguardAudit,
    pub dns: DnsAudit,
    pub alerts: Vec<NetworkAuditAlert>,
    pub residual_unknowns: Vec<String>,
    pub observed_at_unix_ms: u64,
}

/// Per-node audit: declared services vs observed reachability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NodeAudit {
    pub node_id: String,
    pub node_name: String,
    pub role: String,
    pub declared_ip: Option<String>,
    pub observed_ip: Option<String>,
    pub outcome: &'static str,
    pub services: Vec<ServiceAudit>,
    pub error: Option<String>,
}

/// Single service audit: declared endpoint vs TCP probe result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ServiceAudit {
    pub service: String,
    pub node: String,
    pub declared_endpoint: String,
    pub observed_endpoint: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// WireGuard mesh audit: declared peers vs observed handshakes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WireguardAudit {
    pub interface: String,
    pub declared_peers: usize,
    pub observed_peers: usize,
    pub ok_peers: usize,
    pub stale_peers: usize,
    pub dead_peers: usize,
    pub peers: Vec<WireguardPeerAudit>,
}

/// Single WireGuard peer audit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WireguardPeerAudit {
    pub public_key: String,
    pub declared_endpoint: String,
    pub observed_endpoint: Option<String>,
    pub latest_handshake_s: Option<u64>,
    pub status: &'static str,
}

/// DNS audit: declared records vs observed resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DnsAudit {
    pub zone: String,
    pub total_records: usize,
    pub resolved_records: usize,
    pub failed_records: usize,
    pub records: Vec<DnsRecordAudit>,
}

/// Single DNS record audit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DnsRecordAudit {
    pub name: String,
    pub declared_address: String,
    pub observed_address: Option<String>,
    pub status: &'static str,
}

/// Alert emitted when declared vs observed diverges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkAuditAlert {
    pub level: NetworkAuditAlertLevel,
    pub component: String,
    pub message: String,
    pub recovery_hint: String,
}

/// Alert severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditAlertLevel {
    Ok,
    Warning,
    Critical,
}
