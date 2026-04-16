//! Storage domain types — pure data. No I/O, no async, no adapter logic.

use serde::{Deserialize, Serialize};

/// A development workflow observation — tool usage, file edits, errors.
/// Captured automatically by hooks, stored with TTL, feeds CCM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub project: String,
    pub agent_id: String,
    pub tool: String,
    pub target: String,
    pub domain: String,
    pub status: String,
    pub context: String,
    pub session_id: String,
    pub timestamp: String,
    pub tags: Vec<String>,
}

// ── TYPED QUERY RESULTS (Gate 3: zero serde_json::Value in domain/) ──

/// Aggregated observation frequency — result of GROUP BY target, tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationFrequency {
    pub target: String,
    pub tool: String,
    pub freq: u64,
}

/// Session × target pair for co-occurrence extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTarget {
    pub session_id: String,
    pub target: String,
}

/// Raw observation row — used by list_observations_raw and get_session_observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawObservation {
    /// SurrealDB record ID (e.g. "observation:abc123") — preserved for API compat.
    #[serde(default)]
    pub id: String,
    pub tool: String,
    pub target: String,
    pub domain: String,
    pub status: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Historical usage row loaded at boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRow {
    pub dog_id: String,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub failures: u64,
    #[serde(default)]
    pub total_latency_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Storage query failed: {0}")]
    QueryFailed(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Storage metrics snapshot — exposed via /health for observability.
/// Domain type — no dependency on specific DB adapter.
#[derive(Debug, Clone, Serialize)]
pub struct StorageMetrics {
    pub queries: u64,
    pub errors: u64,
    pub slow_queries: u64,
    pub avg_latency_ms: f64,
    pub uptime_secs: u64,
}
