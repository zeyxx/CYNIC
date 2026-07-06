//! MCP tool parameter types — proxy's own contract (duplicated from kernel, free to diverge).

use std::sync::atomic::{AtomicU64, Ordering};

use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::Deserialize;

// ── API discovery ─────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApiContractsParams {
    /// Which API contract to fetch: manifest, openapi, topology, or layer_alignment.
    pub contract: Option<String>,
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
    if kind != "hermes" || !matches!(domain, "organ-forge" | "repo-lifecycle") {
        return Ok(());
    }
    let parsed: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        McpError::invalid_params(
            format!("agent task content must be valid JSON for {domain}: {e}"),
            None,
        )
    })?;
    let has_declared_targets = ["coord_targets", "targets", "repo_targets", "files"]
        .iter()
        .any(|key| {
            parsed
                .get(key)
                .and_then(|value| value.as_array())
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        });
    if !has_declared_targets {
        return Err(McpError::invalid_params(
            "repo-affecting hermes tasks must declare coord_targets, targets, repo_targets, or files",
            None,
        ));
    }
    validate_forge_lifecycle_contract(&parsed, load_registry().as_ref())
        .map_err(|message| McpError::invalid_params(message, None))?;
    Ok(())
}

fn validate_forge_lifecycle_contract(
    parsed: &serde_json::Value,
    registry: Option<&serde_json::Value>,
) -> Result<(), String> {
    let lifecycle_name = required_non_empty_string(parsed, "lifecycle")?;
    let contract_fields = [
        "orchestrator_organ",
        "build_runner_role",
        "deploy_target_role",
        "build_target",
        "deploy_target",
    ];
    for field in contract_fields {
        required_non_empty_string(parsed, field)?;
    }

    let Some(registry) = registry else {
        return Ok(());
    };
    let Some(lifecycle) = registry
        .get("lifecycle")
        .and_then(|value| value.get(lifecycle_name))
    else {
        return Ok(());
    };

    require_registry_match(parsed, lifecycle, "orchestrator_organ")?;
    require_registry_match(parsed, lifecycle, "build_runner_role")?;
    require_registry_match(parsed, lifecycle, "deploy_target_role")?;
    require_registry_json_match(parsed, lifecycle, "runtime_paths")?;
    require_registry_json_match(parsed, lifecycle, "runtime_path_proofs")?;
    require_registry_json_match(parsed, lifecycle, "activation_evidence_contract")?;

    let nodes = registry
        .get("nodes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let build_role = lifecycle
        .get("build_runner_role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let deploy_role = lifecycle
        .get("deploy_target_role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let deploy_flag = lifecycle
        .get("deploy_target_flag")
        .and_then(|value| value.as_str())
        .unwrap_or("deploy_target");
    if let Some(expected_build_target) = node_name_for_role(&nodes, build_role, None) {
        require_content_value(parsed, "build_target", &expected_build_target)?;
    }
    if let Some(expected_deploy_target) = node_name_for_role(&nodes, deploy_role, Some(deploy_flag))
    {
        require_content_value(parsed, "deploy_target", &expected_deploy_target)?;
    }
    Ok(())
}

fn required_non_empty_string<'a>(
    parsed: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, String> {
    let value = parsed
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if value.is_empty() {
        return Err(format!("repo-affecting forge tasks must declare {field}"));
    }
    Ok(value)
}

fn require_registry_match(
    parsed: &serde_json::Value,
    lifecycle: &serde_json::Value,
    field: &str,
) -> Result<(), String> {
    if let Some(expected) = lifecycle.get(field).and_then(|value| value.as_str()) {
        require_content_value(parsed, field, expected)?;
    }
    Ok(())
}

fn require_registry_json_match(
    parsed: &serde_json::Value,
    lifecycle: &serde_json::Value,
    field: &str,
) -> Result<(), String> {
    if let Some(expected) = lifecycle.get(field)
        && parsed.get(field) != Some(expected)
    {
        return Err(format!(
            "repo-affecting forge tasks must match registry {field}"
        ));
    }
    Ok(())
}

fn require_content_value(
    parsed: &serde_json::Value,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    if parsed.get(field).and_then(|value| value.as_str()) != Some(expected) {
        return Err(format!(
            "repo-affecting forge tasks must match registry {field}={expected}"
        ));
    }
    Ok(())
}

fn node_name_for_role(
    nodes: &[serde_json::Value],
    role: &str,
    deploy_flag: Option<&str>,
) -> Option<String> {
    nodes
        .iter()
        .find(|node| {
            node.get("role").and_then(|value| value.as_str()) == Some(role)
                && deploy_flag
                    .map(|flag| node.get(flag).and_then(|value| value.as_bool()) == Some(true))
                    .unwrap_or(true)
        })
        .and_then(|node| node.get("name").and_then(|value| value.as_str()))
        .map(String::from)
}

fn load_registry() -> Option<serde_json::Value> {
    let project_root = std::env::var("CYNIC_PROJECT_ROOT")
        .map(std::path::PathBuf::from)
        .ok()
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|path| std::path::PathBuf::from(path.trim()))
        })
        .or_else(|| std::env::current_dir().ok());
    let registry_path = project_root?.join("infra/registry.json");
    std::fs::read_to_string(registry_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

// ── MCP Tool Parameters ─────────────────────────────────────

// --- Coordination / transport-agnostic payloads -------------------------------

fn default_coordination_contract_version() -> String {
    "coordination-event-v1".to_string()
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
            "build_target": "cynic-vm-anvil",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-v2",
            "deploy_target_role": "kernel_primary",
            "runtime_paths": {"repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let r = validate_agent_task_content("hermes", "organ-forge", &content);

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

        let result = validate_forge_lifecycle_contract(&content, Some(&registry));

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

include!("types_split/coord.rs");
include!("types_split/task.rs");
include!("types_split/network.rs");
include!("types_split/infra.rs");
include!("types_split/fleet.rs");
include!("types_split/storage.rs");
include!("types_split/identity.rs");
