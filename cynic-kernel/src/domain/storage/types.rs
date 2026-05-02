//! Storage domain types — pure data. No I/O, no async, no adapter logic.

use serde::{Deserialize, Serialize};

/// A development workflow observation — tool usage, file edits, errors.
/// Captured automatically by hooks, stored with TTL, feeds CCM.
/// Extended: ledger system with hash chain, consensus, multi-observer support.
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

    // ── Ledger system ──
    #[serde(default)]
    pub value: Option<serde_json::Value>, // The fact being observed
    #[serde(default)]
    pub confidence: Option<String>, // observed|deduced|inferred|conjecture
    #[serde(default)]
    pub consumer: Option<String>, // K15: who must act on this
    #[serde(default)]
    pub action: Option<String>, // K15: what changes if true
    #[serde(default)]
    pub depends_on: Vec<String>, // Kairos: dependencies for ripeness
    #[serde(default)]
    pub maturity: Option<f64>, // Kairos: is it ripe (≥φ⁻¹)?
    #[serde(default)]
    pub hash: String, // SHA256 hash of this observation
    #[serde(default)]
    pub prev_hash: String, // Hash of previous observation (chain)
    #[serde(default)]
    pub observers: Vec<String>, // Observer agent_ids (for consensus)
    #[serde(default)]
    pub consensus_score: Option<f64>, // Weighted average of observer confidence
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

    // ── Ledger system ──
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub consumer: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub maturity: Option<f64>,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub prev_hash: String,
    #[serde(default)]
    pub observers: Vec<String>,
    #[serde(default)]
    pub consensus_score: Option<f64>,
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
    #[error("Invalid input: {0}")]
    InvalidInput(String),
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

/// Agent task — submitted to queue for execution by specialized agents (Hermes, future agents).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub kind: String,           // "hermes" | "nightshift" | "future-agent"
    pub domain: String,         // "twitter" | "token" | "on-chain"
    pub content: String,        // task payload (tweet, token_address, etc)
    pub status: String,         // "pending" | "processing" | "completed" | "failed"
    pub result: Option<String>, // agent result (verdict summary, action taken)
    pub created_at: String,
    pub completed_at: Option<String>,
    pub agent_id: Option<String>,
    pub error: Option<String>,
}

/// Infrastructure event: node latency, output size, success/fail.
/// Fire-and-forget — consumed by fleet_stats for routing intelligence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub tool: String,      // "mcp_tailscale" | "kernel_snapshot" | etc.
    pub node: String,      // target node: "cynic-gpu", "cynic-core", "kairos"
    pub elapsed_ms: u64,   // latency
    pub output_bytes: u64, // response size
    pub success: bool,     // operation succeeded
    pub metadata: String,  // optional context (error reason, command, etc.)
    pub agent_id: String,  // who triggered this
    pub timestamp: String, // RFC3339
    #[serde(default)]
    pub failure_reason: String, // from ts_introspect: "none" | "port_conflict" | "process_crash" | "config_error" | etc.
}

/// Raw event row — used by list_events and fleet_stats aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    #[serde(default)]
    pub id: String,
    pub tool: String,
    pub node: String,
    pub elapsed_ms: u64,
    pub output_bytes: u64,
    pub success: bool,
    #[serde(default)]
    pub metadata: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub failure_reason: String,
}
