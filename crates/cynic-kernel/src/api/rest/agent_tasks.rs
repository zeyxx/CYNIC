//! REST handlers for agent task queue — dispatch, poll, complete.
//! Mirrors MCP agent_tools for HTTP clients (Python daemons, curl, future brain agent).

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::storage::AgentTask;

#[derive(Debug, Deserialize)]
pub struct DispatchTaskRequest {
    pub kind: String,
    pub domain: String,
    pub content: String,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskResultRequest {
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimTaskRequest {
    pub agent_id: String,
}

fn validate_agent_task_content(kind: &str, domain: &str, content: &str) -> Result<(), String> {
    if kind != "hermes" || !matches!(domain, "organ-forge" | "repo-lifecycle") {
        return Ok(());
    }
    let parsed: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("agent task content must be valid JSON for {domain}: {e}"))?;
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
        return Err(
            "repo-affecting hermes tasks must declare coord_targets, targets, repo_targets, or files"
                .to_string(),
        );
    }
    validate_forge_lifecycle_contract(&parsed, load_registry().as_ref())?;
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
    let project_root = match std::env::var("CYNIC_PROJECT_ROOT") {
        Ok(path) => Some(std::path::PathBuf::from(path)),
        Err(env_error) => {
            tracing::warn!(error = %env_error, "CYNIC_PROJECT_ROOT not set; resolving registry path from git");
            match std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
            {
                Ok(output) => match String::from_utf8(output.stdout) {
                    Ok(path) => Some(std::path::PathBuf::from(path.trim())),
                    Err(utf8_error) => {
                        tracing::warn!(error = %utf8_error, "git top-level path was not valid UTF-8");
                        None
                    }
                },
                Err(git_error) => {
                    tracing::warn!(error = %git_error, "failed to resolve git top-level path");
                    match std::env::current_dir() {
                        Ok(path) => Some(path),
                        Err(cwd_error) => {
                            tracing::warn!(error = %cwd_error, "failed to resolve current directory for registry path");
                            None
                        }
                    }
                }
            }
        }
    };
    let registry_path = project_root?.join("infra/registry.json");
    let content = match std::fs::read_to_string(&registry_path) {
        Ok(content) => content,
        Err(read_error) => {
            tracing::warn!(error = %read_error, path = %registry_path.display(), "failed to read registry.json");
            return None;
        }
    };
    match serde_json::from_str(&content) {
        Ok(registry) => Some(registry),
        Err(parse_error) => {
            tracing::warn!(error = %parse_error, path = %registry_path.display(), "failed to parse registry.json");
            None
        }
    }
}

/// POST /agent-tasks — dispatch a new task to the queue.
pub async fn dispatch_task_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DispatchTaskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.kind.is_empty() || req.kind.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "kind must be 1-64 characters".into(),
            }),
        ));
    }
    if req.content.is_empty() || req.content.chars().count() > 10_000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "content must be 1-10000 characters".into(),
            }),
        ));
    }
    if let Err(message) = validate_agent_task_content(&req.kind, &req.domain, &req.content) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: message }),
        ));
    }

    let task_id = format!("agent-task:{}", crate::infra::crypto::generate_secure_id());
    let now = chrono::Utc::now().to_rfc3339();
    let task = AgentTask {
        id: task_id.clone(),
        kind: req.kind,
        domain: req.domain,
        content: req.content,
        status: "pending".to_string(),
        result: None,
        created_at: now,
        completed_at: None,
        agent_id: req.agent_id.clone(),
        claimed_by: None,
        error: None,
    };

    let task_id = state.storage.store_agent_task(&task).await.map_err(|e| {
        tracing::warn!(error = %e, "dispatch_task failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to store task".into(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "pending",
    })))
}

/// POST /agent-tasks/{id}/claim — atomically claim a pending task.
pub async fn claim_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(req): Json<ClaimTaskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.agent_id.is_empty() || req.agent_id.chars().count() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "agent_id must be 1-64 characters".into(),
            }),
        ));
    }

    let claimed = state
        .storage
        .claim_agent_task(&task_id, &req.agent_id)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "claim_task failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to claim task".into(),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "claimed": claimed,
    })))
}

/// GET /agent-tasks?kind=hermes&domain=organ-anvil&limit=10 — poll pending tasks.
pub async fn list_tasks_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = params.get("kind").map(|s| s.as_str()).unwrap_or("hermes");
    let domain = params
        .get("domain")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tasks = state
        .storage
        .list_pending_agent_tasks(kind, domain, limit)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "list_tasks failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list tasks".into(),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
    })))
}

/// GET /agent-tasks/completed?kind=hermes&limit=10 — poll completed tasks.
pub async fn list_completed_tasks_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = params.get("kind").map(|s| s.as_str()).unwrap_or("hermes");
    let domain = params
        .get("domain")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tasks = state
        .storage
        .list_completed_agent_tasks(kind, domain, limit)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "list_completed_tasks failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list completed tasks".into(),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
    })))
}

/// POST /agent-tasks/{id}/result — mark task complete with result or error.
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(req): Json<UpdateTaskResultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Mark processing first
    let _ = state.storage.claim_agent_task(&task_id, "rest_api").await;

    state
        .storage
        .update_agent_task_result(&task_id, req.result, req.error)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "complete_task failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update task".into(),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "completed",
    })))
}

#[cfg(test)]
mod tests {
    use super::{validate_agent_task_content, validate_forge_lifecycle_contract};

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
    fn validates_forge_task_with_coord_targets() {
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "objective": "build kernel release",
            "orchestrator_organ": "organ-forge",
            "build_target": "cynic-vm-anvil",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-core",
            "deploy_target_role": "kernel_primary",
            "runtime_paths": {"repo_root": "%h/.cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let result = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_forge_task_without_declared_targets() {
        let content = serde_json::json!({
            "objective": "build kernel release"
        })
        .to_string();

        let result = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(
            result.is_err(),
            "repo-affecting forge tasks should declare coordination targets"
        );
    }

    #[test]
    fn rejects_forge_task_without_lifecycle_contract() {
        let content = serde_json::json!({
            "objective": "build kernel release",
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let result = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("declare lifecycle"));
    }

    #[test]
    fn validates_forge_task_against_dynamic_registry_contract() {
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
    fn rejects_forge_task_that_does_not_match_dynamic_registry_contract() {
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
            "orchestrator_organ": "organ-forge",
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

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("match registry orchestrator_organ")
        );
    }

    #[test]
    fn allows_non_forge_task_without_declared_targets() {
        let result =
            validate_agent_task_content("hermes", "telegram", r#"{"objective":"observe"}"#);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_invalid_json_for_forge_task() {
        let result = validate_agent_task_content("hermes", "organ-forge", "not-json");

        assert!(result.is_err());
    }
}
