//! Observation domain types for CCM and epistemic ingestion.
//! Pure types — no I/O, no async.

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
    pub timestamp: i64,
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_tier: String,

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

/// Raw observation row — used by list_observations_raw and get_session_observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawObservation {
    #[serde(default)]
    pub id: String,
    pub tool: String,
    pub target: String,
    pub domain: String,
    pub status: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_tier: String,

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
