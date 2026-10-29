//! Introspection Types — anomaly thresholds and alert shapes.
//!
//! Canonical owner: `cynic-intel` (SoC foundation §481).

use serde::{Deserialize, Serialize};

pub const NETWORK_SECURITY_REPORT_SCHEMA: &str = "cynic.intel.soc_security_report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAuditStatus {
    Clean,
    ThreatsDetected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct NetworkSecurityAuditReport {
    pub schema: String,
    pub status: SecurityAuditStatus,
    pub node_id: String,
    pub listening_ports_observed: Vec<u16>,
    pub allowed_ports_policy: Vec<u16>,
    pub wireguard_peers_count: usize,
    pub dropped_packets_count: u64,
    pub alerts_count: usize,
    pub alerts: Vec<Alert>,
    pub observed_at_unix_ms: u64,
}

/// Anomaly thresholds — conservative defaults.
pub const FAILURE_RATE_THRESHOLD: f64 = 0.20; // > 20% failure rate
pub const EMBED_FAILURE_THRESHOLD: f64 = 0.50; // > 50% embedding failures
/// phi^-2 = 0.382 — minimum acceptable savings rate (RTK gauge is 0-100).
pub const SAVINGS_PCT_THRESHOLD: f64 = 38.2;

/// Alert produced by introspection — serialized into /health "alerts" section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    pub kind: &'static str,
    pub message: String,
    pub severity: &'static str, // "warning" | "critical"
}

impl Alert {
    pub fn new(kind: &'static str, message: impl Into<String>, severity: &'static str) -> Self {
        Self {
            kind,
            message: message.into(),
            severity,
        }
    }
}
