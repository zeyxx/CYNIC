//! Canonical entities for agent session traces.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/canonical/domain/entities.py`.
//! Design rule (operator invariant): harnesses carry DIFFERENT models. Every
//! entity keeps normalized fields (best effort, `None` when unobserved) beside
//! native fields (the exact raw payload the harness used). Missing data is
//! `None`, never a default. The normalized view is a convenience; the native
//! view is the truth.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Typed relation between trajectory nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Reply,
    ToolCall,
    ToolResult,
    Reasoning,
    Compaction,
    Sidechain,
    Subagent,
    Battle,
    ModelChange,
    Sequence,
}

/// Where an edge comes from. Native = the harness states it; derived = we infer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeProvenance {
    Native,
    Derived,
}

/// Classification of a human intervention. Unknown is the honest default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionKind {
    Unknown,
    Redirect,
    Constraint,
    Rejection,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub turn_uid: String,
    pub session_uid: String,
    pub index: i64,
    pub role: String,
    /// None and "" are DISTINCT raw states; keep the raw value, never collapse.
    pub content: Option<String>,
    pub timestamp_iso: Option<String>,
    pub native_model: Option<String>,
    pub native_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub exec_uid: String,
    pub session_uid: String,
    pub turn_uid: Option<String>,
    pub index: i64,
    pub tool_name: String,
    pub call_id: Option<String>,
    pub command_or_args: Option<String>,
    pub raw_output: Option<String>,
    pub exit_code: Option<i64>,
    pub is_error: bool,
    /// Links a result back to the call that produced it (harness-native when available).
    pub parent_exec_uid: Option<String>,
    pub timestamp_iso: Option<String>,
    pub native_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningTrace {
    pub trace_uid: String,
    pub session_uid: String,
    pub turn_uid: Option<String>,
    pub content: String,
    pub subject: Option<String>,
    pub model: Option<String>,
    pub timestamp_iso: Option<String>,
}

/// A typed relation. `source_uids` allows n-ary relations (one compaction
/// summarizing N turns, one battle between M agents). An edge requires a
/// non-empty `source_uids` — enforced by `NormalizedSession::check`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub edge_uid: String,
    pub session_uid: String,
    pub kind: EdgeKind,
    pub source_uids: Vec<String>,
    pub target_uid: Option<String>,
    pub provenance: EdgeProvenance,
    pub payload: HashMap<String, serde_json::Value>,
    pub timestamp_iso: Option<String>,
}

/// The state that evolves at a point in the trajectory. All fields optional —
/// a harness that does not expose state simply leaves them `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub snapshot_uid: String,
    pub session_uid: String,
    pub turn_uid: Option<String>,
    pub index: i64,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub sandbox_policy: Option<String>,
    pub approval_policy: Option<String>,
    pub context_prompt_tokens: Option<i64>,
    pub context_nonmessage_tokens: Option<i64>,
    pub cumulative_tokens: Option<i64>,
    pub compacted: Option<bool>,
    pub native_fields: HashMap<String, serde_json::Value>,
    pub timestamp_iso: Option<String>,
}

/// A human turn. First-class: the human signal is the richest thing the
/// corpus holds, not a plain dialogue row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanSignal {
    pub human_uid: String,
    pub session_uid: String,
    pub turn_uid: String,
    /// None and "" are DISTINCT raw states; keep the raw value, never collapse.
    pub text: Option<String>,
    pub kind: CorrectionKind,
    pub markers: Vec<String>,
    pub timestamp_iso: Option<String>,
}
