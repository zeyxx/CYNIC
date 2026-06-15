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
    /// E1: Source tier — classified at write time via classify_source_tier().
    /// "permanent" (organs/services), "cron" (scheduled), "session" (ephemeral).
    #[serde(default)]
    pub source_tier: String,

    // ── Ledger system ──
    #[serde(default)]
    pub value: Option<serde_json::Value>, // K5-exempt: arbitrary JSON for heterogeneous observation facts; unavoidable for generic observation storage
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
    #[serde(default)]
    pub source_tier: String,

    // ── Ledger system ──
    #[serde(default)]
    pub value: Option<serde_json::Value>, // K5-exempt: arbitrary JSON for heterogeneous observation facts; unavoidable for generic observation storage
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

/// Content payload for a governance:submission task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionTaskContent {
    pub branch: String,
    pub commit_message: String,
    pub pr_metadata: PRMetadata,
    pub audit_context: AuditContext,
    #[serde(default = "default_true")]
    pub pre_submission_check: bool,
    #[serde(default = "default_three")]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRMetadata {
    pub title: String,
    pub body: String,
    pub target_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub session_id: String,
    pub compliance_score: f64,
}

fn default_true() -> bool {
    true
}
fn default_three() -> u32 {
    3
}

/// Agent task — submitted to queue for execution by specialized agents (Hermes, future agents).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub kind: String,           // "submission" | "hermes" | "nightshift" | ...
    pub domain: String,         // "governance:submission" | "twitter" | ...
    pub content: String,        // JSON serialized SubmissionTaskContent or other payload
    pub status: String,         // "pending" | "processing" | "completed" | "failed"
    pub result: Option<String>, // agent result (verdict summary, action taken)
    pub created_at: String,
    pub completed_at: Option<String>,
    pub agent_id: Option<String>,
    pub claimed_by: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub consumer: Option<String>,
    #[serde(default)]
    pub escalation_policy: Option<String>,
    #[serde(default)]
    pub overdue: bool,
    #[serde(default)]
    pub age_seconds: Option<i64>,
}

impl AgentTask {
    pub fn with_computed_metadata(mut self) -> Self {
        let now = chrono::Utc::now();
        if self.due_at.is_none() {
            self.due_at = self.content_string("due_at");
        }
        self.priority = self.priority.or_else(|| self.content_i64("priority"));
        if self.consumer.is_none() {
            self.consumer = self.content_string("consumer");
        }
        if self.escalation_policy.is_none() {
            self.escalation_policy = self.content_string("escalation_policy");
        }
        self.overdue = self.is_overdue_at(now);
        self.age_seconds = match chrono::DateTime::parse_from_rfc3339(&self.created_at) {
            Ok(created) => Some(
                now.signed_duration_since(created.with_timezone(&chrono::Utc))
                    .num_seconds(),
            ),
            Err(_) => None,
        };
        self
    }

    pub fn is_overdue_at(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        if self.status != "pending" {
            return false;
        }
        let due_at = self
            .due_at
            .clone()
            .or_else(|| self.content_string("due_at"));
        let Some(due_at) = due_at else {
            return false;
        };
        match chrono::DateTime::parse_from_rfc3339(&due_at) {
            Ok(due) => due.with_timezone(&chrono::Utc) < now,
            Err(_) => false,
        }
    }

    // WHY: gate-0 forbids silent `.ok()` here; keep the parse failure explicit.
    #[allow(clippy::manual_ok_err)]
    fn content_metadata(&self) -> Option<AgentTaskContentMetadata> {
        match serde_json::from_str::<AgentTaskContentMetadata>(&self.content) {
            Ok(metadata) => Some(metadata),
            Err(_) => None,
        }
    }

    fn content_string(&self, key: &str) -> Option<String> {
        let metadata = self.content_metadata()?;
        let value = match key {
            "due_at" => metadata.due_at,
            "consumer" => metadata.consumer,
            "escalation_policy" => metadata.escalation_policy,
            _ => None,
        }?;
        if value.is_empty() { None } else { Some(value) }
    }

    fn content_i64(&self, key: &str) -> Option<i64> {
        let metadata = self.content_metadata()?;
        match key {
            "priority" => metadata.priority,
            _ => None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct AgentTaskContentMetadata {
    #[serde(default)]
    due_at: Option<String>,
    #[serde(default)]
    priority: Option<i64>,
    #[serde(default)]
    consumer: Option<String>,
    #[serde(default)]
    escalation_policy: Option<String>,
}

/// Infrastructure event: node latency, output size, success/fail.
/// Fire-and-forget — consumed by fleet_stats for routing intelligence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub tool: String,      // "mcp_tailscale" | "kernel_snapshot" | etc.
    pub node: String,      // target node: "cynic-host-gpu", "cynic-host-proxmox", "kairos"
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

#[cfg(test)]
mod agent_task_tests {
    use super::AgentTask;

    fn task_with_content(content: &str) -> AgentTask {
        AgentTask {
            id: "agent_tasks:test".to_string(),
            kind: "cortex".to_string(),
            domain: "scaling-lab".to_string(),
            content: content.to_string(),
            status: "pending".to_string(),
            result: None,
            created_at: "2026-06-15T00:00:00Z".to_string(),
            completed_at: None,
            agent_id: Some("tester".to_string()),
            claimed_by: None,
            error: None,
            due_at: None,
            priority: None,
            consumer: None,
            escalation_policy: None,
            overdue: false,
            age_seconds: None,
        }
    }

    #[test]
    fn task_metadata_falls_back_to_json_content() {
        let task = task_with_content(
            r#"{"due_at":"2026-06-14T23:59:00Z","priority":7,"consumer":"human+cortex","escalation_policy":"page human"}"#,
        )
        .with_computed_metadata();

        assert_eq!(task.due_at.as_deref(), Some("2026-06-14T23:59:00Z"));
        assert_eq!(task.priority, Some(7));
        assert_eq!(task.consumer.as_deref(), Some("human+cortex"));
        assert_eq!(task.escalation_policy.as_deref(), Some("page human"));
    }

    #[test]
    fn pending_task_with_past_due_at_is_overdue() {
        let task = task_with_content(r#"{"due_at":"2026-06-14T23:59:00Z"}"#);
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert!(task.is_overdue_at(now));
    }
}

/// Multi-cortex dispatch record — coordinates work across Claude Code, Gemini, and Hermes.
/// Each cortex claims a scope and tracks status through the execution pipeline.
/// Hash chained for integrity (COMPLETED dispatch) → CLAIMED → PROCESSING → PROPOSED → COMPLETED.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDispatch {
    pub id: String,         // SurrealDB record ID
    pub scope: String,      // "auth" | "trading" | "validation" | etc.
    pub zone: String,       // "l0" | "l1" | "l2" | etc.
    pub claimed_by: String, // agent_id of executing cortex
    pub branch: String,     // git branch name for isolation
    pub status: String,     // "CLAIMED" | "PROCESSING" | "PROPOSED" | "COMPLETED"
    pub created_at: String, // RFC3339 timestamp
    #[serde(default)]
    pub completed_at: Option<String>, // RFC3339 timestamp when COMPLETED
    #[serde(default)]
    pub pr_number: Option<u32>, // GitHub PR number after PROPOSED
    pub hash: String,       // SHA256(scope|zone|claimed_by|branch|status)
    pub prev_hash: String,  // Chain: hash of previous COMPLETED dispatch for scope
}
