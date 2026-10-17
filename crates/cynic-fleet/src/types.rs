//! Fleet projection types. Plain data, no I/O.

use serde::{Deserialize, Serialize};

/// Minimal node view supplied by the adapter (parsed from the registry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    pub id: String,
    pub role: String,
    pub status: String,
    pub certificate_id: String,
    pub authority_id: String,
}

/// Minimal log view supplied by the adapter (parsed from the coordination log).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts_unix_s: u64,
    pub agent: String,
    pub kind: String,
    pub body: String,
}

/// One projected fleet row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetNodeRow {
    pub id: String,
    pub role: String,
    pub status: String,
    pub certificate_id: String,
    pub authority_id: String,
}

/// One projected log row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetLogRow {
    pub ts_unix_s: u64,
    pub agent: String,
    pub kind: String,
    pub body: String,
}

/// The fleet projection both surfaces render.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetReport {
    pub schema: String,
    pub generated_unix_s: u64,
    pub registry_path: String,
    pub nodes: Vec<FleetNodeRow>,
    pub debts: Vec<FleetLogRow>,
    pub infos: Vec<FleetLogRow>,
}
