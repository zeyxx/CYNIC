//! MCP tool parameter types — proxy projection over canonical host/runtime contracts.

use std::sync::atomic::{AtomicU64, Ordering};

pub use types_split::task::JudgmentParams;

use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SolanaPlanParams {
    pub cluster: String,
    pub target: String,
    pub helius_plan: Option<String>,
    pub signature_page_limit: Option<u16>,
    pub max_signature_pages: Option<u32>,
    pub max_transactions: Option<u64>,
    pub expected_transactions: Option<u64>,
    pub commitment: Option<String>,
}

// ── API discovery ─────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApiContractsParams {
    /// Which API contract to fetch: manifest, openapi, topology, or layer_alignment.
    pub contract: Option<String>,
}

// ── Docker ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DockerContainerListParams {
    /// Include stopped containers. Defaults to true.
    pub all: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DockerContainerLogsParams {
    /// Container name or id.
    pub container: String,
    /// Return the last N log lines. Defaults to 200.
    pub tail: Option<u32>,
    /// Include timestamps in the output. Defaults to true.
    pub timestamps: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DockerContainerInspectParams {
    /// Container name or id.
    pub container: String,
}

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

pub fn validate_agent_task_content(
    kind: &str,
    domain: &str,
    content: &str,
) -> Result<(), McpError> {
    cynic_host::lifecycle::forge_task_contract::validate_repo_affecting_hermes_task(
        kind,
        domain,
        content,
        load_registry().as_ref(),
    )
    .map_err(|message| McpError::invalid_params(message, None))?;
    Ok(())
}

fn load_registry() -> Option<serde_json::Value> {
    cynic_host::runtime_registry::load_runtime_registry(None)
}

// ── MCP Tool Parameters ─────────────────────────────────────

// --- Coordination / transport-agnostic payloads -------------------------------

pub fn default_coordination_contract_version() -> String {
    "coordination-event-v1".to_string()
}

// ── Coordination / transport-agnostic payloads (canonical home: types.rs) ──

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationThreadReplyParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub thread_id: String,
    /// Message to post in the thread.
    pub message: String,
    pub branch: Option<String>,
    pub reply_to: Option<String>,
    /// If set, keeps a link to artifacts referenced in this reply.
    pub artifacts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationArtifactShareParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub artifacts: Vec<String>,
    /// Optional thread context to continue discussion around shared artifacts.
    pub thread_id: Option<String>,
    pub message: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationDecisionRecordParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    /// Decision topic.
    pub topic: String,
    /// Decision outcome.
    pub decision: String,
    /// Decision rationale.
    pub rationale: String,
    /// Source artifact path or commit anchor.
    pub source_artifact: String,
    pub branch: Option<String>,
    pub thread_id: Option<String>,
    pub open_points: Option<Vec<String>>,
    pub artifacts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationContractManifestParams {
    pub client: Option<String>,
    pub project: Option<String>,
    pub operation: Option<String>,
    pub include_default_project_profile: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InfraRegistryValidateParams {
    /// Validation dimensions. Defaults to ["network_policy"].
    pub dimensions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimAgentTaskParams {
    pub task_id: String,
    pub agent_id: Option<String>,
}

// ── Solana Pay (canonical home: types.rs) ──────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SolanaPayParams {
    pub recipient: String,
    pub amount: f64,
    pub spl_token: Option<String>,
    pub reference: Option<String>,
    pub label: Option<String>,
    pub message: Option<String>,
    pub memo: Option<String>,
    pub allowance: Option<serde_json::Value>,
    pub context_tokens: Option<u64>,
    pub actions_budget: Option<u32>,
}

// ── Coord pub params (moved from orphan types_split) ───────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationStatusPublishParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub branch: Option<String>,
    /// Canonical event types: workstream.started|workstream.blocked|workstream.updated|interface.changed|run.completed|decision.recorded.
    pub event: String,
    /// Short machine state label, e.g. "stable", "blocked", "requires-review".
    pub status: Option<String>,
    /// Human summary for humans and transport renderers.
    pub summary: String,
    /// Optional artifact references for source of truth.
    pub artifacts: Option<Vec<String>>,
    /// Optional open points.
    pub open_points: Option<Vec<String>>,
    /// Coordination thread identifier.
    pub thread_id: Option<String>,
    /// Optional payload extension (repo-specific fields).
    pub payload: Option<serde_json::Value>,
    /// Optional transport hint: "public", "team", "restricted".
    pub sensitivity: Option<String>,
    /// Contract version in payload metadata.
    #[serde(default = "default_coordination_contract_version")]
    pub contract_version: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationStatusReadParams {
    pub project: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub workstream: Option<String>,
    pub thread_id: Option<String>,
    pub limit: Option<u32>,
    pub domain: Option<String>,
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
    fn observe_params_accepts_k15_ledger_fields() {
        let r = serde_json::from_value::<ObserveParams>(serde_json::json!({
            "tool": "repo_edit",
            "domain": "mempool",
            "status": "critical",
            "consumer": "organ-forge",
            "action": "verify lifecycle dispatch read-back",
            "confidence": "observed",
            "value": {"task_id": "agent_tasks:test"},
            "depends_on": ["agent_tasks:test"],
            "maturity": 0.9
        }));

        let params = r.expect("ObserveParams should accept K15 ledger fields");
        assert_eq!(params.consumer.as_deref(), Some("organ-forge"));
        assert_eq!(params.confidence.as_deref(), Some("observed"));
        assert_eq!(
            params.depends_on.as_deref(),
            Some(&["agent_tasks:test".to_string()][..])
        );
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

    fn activation_evidence_contract() -> serde_json::Value {
        serde_json::json!({
            "patch_field": "activation_evidence_patch",
            "node_field": "node",
            "runtime_paths_field": "activation_evidence.runtime_paths",
            "services_field": "activation_evidence.services",
            "verified_requires": ["marker_exists", "sentinels_verified"],
            "service_active_requires": ["declared", "active"],
            "ingestion_mode": "validated_merge",
        })
    }

    #[test]
    fn dispatch_agent_task_content_accepts_coord_targets_for_forge() {
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "objective": "build kernel release",
            "orchestrator_organ": "organ-forge",
            "build_target": "local-builder",
            "build_runner_role": "build_runner",
            "deploy_target": "local-kernel",
            "deploy_target_role": "kernel_primary",
            "runtime_paths": {"repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let r = cynic_host::lifecycle::forge_task_contract::validate_repo_affecting_hermes_task(
            "hermes",
            "organ-forge",
            &content,
            None,
        );

        assert!(r.is_ok());
    }

    #[test]
    fn dispatch_agent_task_content_rejects_forge_without_targets() {
        let content = serde_json::json!({
            "objective": "build kernel release"
        })
        .to_string();

        let r = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(
            r.is_err(),
            "repo-affecting forge tasks should declare coordination targets"
        );
    }

    #[test]
    fn dispatch_agent_task_content_rejects_forge_without_lifecycle_contract() {
        let content = serde_json::json!({
            "objective": "build kernel release",
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let r = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(
            r.is_err(),
            "repo-affecting forge tasks should declare lifecycle contract"
        );
    }

    #[test]
    fn dispatch_agent_task_content_follows_dynamic_registry_contract() {
        let registry = serde_json::json!({
            "lifecycle": {
                "kernel_release": {
                    "orchestrator_organ": "organ-build",
                    "build_runner_role": "builder",
                    "deploy_target_role": "primary",
                    "deploy_target_flag": "ships_kernel",
                    "runtime_paths": {"repo_root": "%h/.cynic/repos/CYNIC"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
                    "activation_evidence_contract": activation_evidence_contract()
                }
            },
            "nodes": [
                {"name": "node-builder", "role": "builder"},
                {"name": "node-primary", "role": "primary", "ships_kernel": true}
            ]
        });
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "orchestrator_organ": "organ-build",
            "build_target": "node-builder",
            "build_runner_role": "builder",
            "deploy_target": "node-primary",
            "deploy_target_role": "primary",
            "runtime_paths": {"repo_root": "%h/.cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["infra/registry.json"]
        });

        let result = cynic_host::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(
            &content,
            Some(&registry),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_agent_task_content_allows_non_forge_without_targets() {
        let r = validate_agent_task_content("hermes", "telegram", r#"{"objective":"observe"}"#);

        assert!(r.is_ok());
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
    fn poison_watch_pending_params_rejects_empty() {
        let r = serde_json::from_value::<WatchPendingAgentTasksParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "WatchPendingAgentTasksParams should require 'kind'"
        );
    }

    #[test]
    fn poison_claim_task_params_rejects_empty() {
        let r = serde_json::from_value::<ClaimAgentTaskParams>(serde_json::json!({}));
        assert!(r.is_err(), "ClaimAgentTaskParams should require 'task_id'");
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
    fn poison_validate_params_requires_source_commit() {
        let r = serde_json::from_value::<ValidateParams>(serde_json::json!({}));
        assert!(
            r.is_err(),
            "ValidateParams must bind a sovereign source commit"
        );
    }

    #[test]
    fn validate_params_defaults_to_check_and_accepts_release_build() {
        let default = serde_json::from_value::<ValidateParams>(serde_json::json!({
            "source_commit": "a".repeat(40)
        }))
        .expect("default validate action");
        assert_eq!(default.action.canonical_action(), "make:check");

        let release = serde_json::from_value::<ValidateParams>(serde_json::json!({
            "source_commit": "b".repeat(40),
            "action": "build_release"
        }))
        .expect("release action");
        assert_eq!(release.action.canonical_action(), "make:build-release");
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
