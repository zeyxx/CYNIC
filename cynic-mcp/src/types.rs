//! MCP tool parameter types — proxy's own contract (duplicated from kernel, free to diverge).

use std::sync::atomic::{AtomicU64, Ordering};

use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::Deserialize;

// ── MCP Rate Limiter ──────────────────────────────────────────

pub struct McpRateLimit {
    judge_calls: AtomicU64,
    other_calls: AtomicU64,
    window_start: std::sync::Mutex<std::time::Instant>,
}

impl std::fmt::Debug for McpRateLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpRateLimit")
            .field("judge_calls", &self.judge_calls.load(Ordering::Relaxed))
            .field("other_calls", &self.other_calls.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl McpRateLimit {
    pub fn new() -> Self {
        Self {
            judge_calls: AtomicU64::new(0),
            other_calls: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    pub fn check_judge(&self) -> Result<(), McpError> {
        self.maybe_reset();
        let count = self.judge_calls.fetch_add(1, Ordering::Relaxed);
        if count >= 10 {
            self.judge_calls.fetch_sub(1, Ordering::Relaxed);
            Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Rate limit: max 10 judge/infer calls per minute",
                None,
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_other(&self) -> Result<(), McpError> {
        self.maybe_reset();
        let count = self.other_calls.fetch_add(1, Ordering::Relaxed);
        if count >= 30 {
            self.other_calls.fetch_sub(1, Ordering::Relaxed);
            Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Rate limit: max 30 calls per minute",
                None,
            ))
        } else {
            Ok(())
        }
    }

    fn maybe_reset(&self) {
        if let Ok(mut start) = self.window_start.lock()
            && start.elapsed() > std::time::Duration::from_secs(60)
        {
            self.judge_calls.store(0, Ordering::Relaxed);
            self.other_calls.store(0, Ordering::Relaxed);
            *start = std::time::Instant::now();
        }
    }
}

impl Default for McpRateLimit {
    fn default() -> Self {
        Self::new()
    }
}

// ── Input Validation Helpers ──────────────────────────────────

pub fn validate_agent_id(agent_id: &Option<String>) -> Result<(), McpError> {
    if let Some(id) = agent_id {
        if id.is_empty() || id.chars().count() > 64 {
            return Err(McpError::invalid_params(
                "agent_id must be 1-64 characters",
                None,
            ));
        }
        if id.chars().any(|c| c.is_control()) {
            return Err(McpError::invalid_params(
                "agent_id contains invalid characters",
                None,
            ));
        }
    }
    Ok(())
}

// ── MCP Tool Parameters ─────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JudgeParams {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    pub dogs: Option<Vec<String>>,
    pub agent_id: Option<String>,
    pub crystals: Option<bool>,
    /// Optional: sensitivity level. "high" forces routing to sovereign (local) Dogs only.
    /// Use for private content: DMs, wallet seeds, API keys. Default: none (auto-detected).
    pub sensitivity: Option<String>,
    /// Optional: slot acquisition priority. Controls queuing on sovereign inference.
    /// Values: "user" (30s wait, default), "hermes" (15s), "nightshift" (skip if busy), "background" (skip).
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListParams {
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InferParams {
    pub system: Option<String>,
    pub prompt: String,
    pub agent_id: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetabolismParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComplianceParams {
    /// Agent ID to score compliance for. If omitted, returns recent trend.
    pub agent_id: Option<String>,
    /// Number of recent compliance records to return (trend mode). Default: 20.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditQueryParams {
    pub tool: Option<String>,
    pub agent_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthParams {
    pub api_key: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitParams {
    pub op: crate::local_tools::GitOp,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterParams {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimParams {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchClaimParams {
    pub agent_id: String,
    pub targets: Vec<String>,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseParams {
    pub agent_id: String,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhoParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    pub tool: String,
    pub target: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub context: Option<String>,
    pub project: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DispatchAgentTaskParams {
    pub kind: String,    // "hermes" | "nightshift" | future agent
    pub domain: String,  // "twitter" | "token" | "on-chain"
    pub content: String, // task payload
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPendingAgentTasksParams {
    pub kind: String,
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateAgentTaskResultParams {
    pub task_id: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub agent_id: Option<String>,
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- Required-field structs reject empty JSON --

    #[test]
    fn poison_judge_params_rejects_empty() {
        let r = serde_json::from_value::<JudgeParams>(serde_json::json!({}));
        assert!(r.is_err(), "JudgeParams should require 'content'");
    }

    #[test]
    fn poison_auth_params_rejects_empty() {
        let r = serde_json::from_value::<AuthParams>(serde_json::json!({}));
        assert!(r.is_err(), "AuthParams should require 'api_key'");
    }

    #[test]
    fn poison_infer_params_rejects_empty() {
        let r = serde_json::from_value::<InferParams>(serde_json::json!({}));
        assert!(r.is_err(), "InferParams should require 'prompt'");
    }

    #[test]
    fn poison_register_params_rejects_empty() {
        let r = serde_json::from_value::<RegisterParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "RegisterParams should require 'agent_id' + 'intent'"
        );
    }

    #[test]
    fn poison_claim_params_rejects_empty() {
        let r = serde_json::from_value::<ClaimParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "ClaimParams should require 'agent_id' + 'target'"
        );
    }

    #[test]
    fn poison_batch_claim_params_rejects_empty() {
        let r = serde_json::from_value::<BatchClaimParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "BatchClaimParams should require 'agent_id' + 'targets'"
        );
    }

    #[test]
    fn poison_release_params_rejects_empty() {
        let r = serde_json::from_value::<ReleaseParams>(serde_json::json!({}));
        assert!(r.is_err(), "ReleaseParams should require 'agent_id'");
    }

    #[test]
    fn poison_observe_params_rejects_empty() {
        let r = serde_json::from_value::<ObserveParams>(serde_json::json!({}));
        assert!(r.is_err(), "ObserveParams should require 'tool'");
    }

    #[test]
    fn poison_git_params_rejects_empty() {
        let r = serde_json::from_value::<GitParams>(serde_json::json!({}));
        assert!(r.is_err(), "GitParams should require 'op'");
    }

    #[test]
    fn poison_dispatch_task_params_rejects_empty() {
        let r = serde_json::from_value::<DispatchAgentTaskParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "DispatchAgentTaskParams should require 'kind' + 'domain' + 'content'"
        );
    }

    #[test]
    fn poison_list_pending_params_rejects_empty() {
        let r = serde_json::from_value::<ListPendingAgentTasksParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "ListPendingAgentTasksParams should require 'kind'"
        );
    }

    #[test]
    fn poison_update_task_result_params_rejects_empty() {
        let r = serde_json::from_value::<UpdateAgentTaskResultParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "UpdateAgentTaskResultParams should require 'task_id'"
        );
    }

    // -- All-optional structs accept empty JSON safely --

    #[test]
    fn poison_list_params_accepts_empty() {
        let r = serde_json::from_value::<ListParams>(serde_json::json!({}));
        assert!(r.is_ok(), "ListParams (all optional) should accept {{}}");
    }

    #[test]
    fn poison_audit_query_params_accepts_empty() {
        let r = serde_json::from_value::<AuditQueryParams>(serde_json::json!({}));
        assert!(
            r.is_ok(),
            "AuditQueryParams (all optional) should accept {{}}"
        );
    }

    #[test]
    fn poison_who_params_accepts_empty() {
        let r = serde_json::from_value::<WhoParams>(serde_json::json!({}));
        assert!(r.is_ok(), "WhoParams (all optional) should accept {{}}");
    }

    #[test]
    fn poison_validate_params_accepts_empty() {
        let r = serde_json::from_value::<ValidateParams>(serde_json::json!({}));
        assert!(
            r.is_ok(),
            "ValidateParams (all optional) should accept {{}}"
        );
    }

    // -- validate_agent_id logic --

    #[test]
    fn validate_agent_id_accepts_none() {
        assert!(validate_agent_id(&None).is_ok());
    }

    #[test]
    fn validate_agent_id_accepts_valid() {
        assert!(validate_agent_id(&Some("test-agent-123".into())).is_ok());
    }

    #[test]
    fn validate_agent_id_rejects_empty() {
        assert!(validate_agent_id(&Some(String::new())).is_err());
    }

    #[test]
    fn validate_agent_id_rejects_too_long() {
        let long = "a".repeat(65);
        assert!(validate_agent_id(&Some(long)).is_err());
    }

    #[test]
    fn validate_agent_id_rejects_control_chars() {
        assert!(validate_agent_id(&Some("agent\x00id".into())).is_err());
    }

    // -- Rate limiter --

    #[test]
    fn rate_limit_judge_allows_up_to_10() {
        let rl = McpRateLimit::new();
        for _ in 0..10 {
            assert!(rl.check_judge().is_ok());
        }
        assert!(rl.check_judge().is_err());
    }

    #[test]
    fn rate_limit_other_allows_up_to_30() {
        let rl = McpRateLimit::new();
        for _ in 0..30 {
            assert!(rl.check_other().is_ok());
        }
        assert!(rl.check_other().is_err());
    }
}
