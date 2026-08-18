//! Unit tests for agent tasks.

use super::types::ReviewAgentProposalRequest;
use super::validation::{
    agent_proposal_review_value, event_matches_pending_task_watch, extract_agent_proposal,
    filter_proposal_projection, task_proposal_projection, validate_agent_proposal_review_request,
    validate_agent_task_content, wait_for_matching_task_event,
};
use crate::domain::events::KernelEvent;
use crate::domain::storage::AgentTask;

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
    let registry = serde_json::json!({
        "lifecycle": {
            "kernel_release": {
                "orchestrator_organ": "organ-forge",
                "build_runner_role": "build_runner",
                "deploy_target_role": "kernel_primary",
                "deploy_target_flag": "deploy_target",
                "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
                "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
                "activation_evidence_contract": activation_evidence_contract()
            }
        },
        "nodes": [
            {"name": "cynic-vm-anvil", "role": "build_runner"},
            {"name": "cynic-vm-core", "role": "kernel_primary", "deploy_target": true}
        ]
    });
    let content = serde_json::json!({
        "lifecycle": "kernel_release",
        "objective": "build kernel release",
        "orchestrator_organ": "organ-forge",
        "build_target": "cynic-vm-anvil",
        "build_runner_role": "build_runner",
        "deploy_target": "cynic-vm-core",
        "deploy_target_role": "kernel_primary",
        "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
        "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
        "activation_evidence_contract": activation_evidence_contract(),
        "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
    });

    let result = cynic_host::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(
        &content,
        Some(&registry),
    );

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

fn task_with_content(content: &serde_json::Value) -> AgentTask {
    AgentTask {
        id: "agent_tasks:test".to_string(),
        kind: "hermes".to_string(),
        domain: "organ-lifecycle".to_string(),
        content: content.to_string(),
        status: "pending".to_string(),
        result: None,
        created_at: 1_772_480_000_000,
        completed_at: None,
        agent_id: Some("organ-lifecycle-convergence".to_string()),
        claimed_by: None,
        error: None,
        due_at: None,
        priority: Some(95),
        consumer: Some("organ-lifecycle-hermes-agent".to_string()),
        escalation_policy: None,
        overdue: false,
        age_seconds: Some(12),
    }
}

#[test]
fn extracts_operator_interface_proposal_from_hermes_task_content() {
    let task = task_with_content(&serde_json::json!({
        "schema": "cynic.dependency_convergence_hermes_task.v1",
        "operator_interface_proposal": {
            "schema": "cynic.agent_proposal.v1",
            "proposal_id": "dependency-convergence-test",
            "visibility": "operator_private",
            "operator_review": {"state": "pending"},
            "public_projection": {"eligible": false, "redaction_status": "needs_review"}
        }
    }));

    let proposal = extract_agent_proposal(&task).expect("proposal should be extracted");

    assert_eq!(
        proposal.get("schema").and_then(|value| value.as_str()),
        Some("cynic.agent_proposal.v1")
    );
    assert_eq!(
        proposal.get("visibility").and_then(|value| value.as_str()),
        Some("operator_private")
    );
}

#[test]
fn projects_agent_proposal_for_operator_listing() {
    let task = task_with_content(&serde_json::json!({
        "operator_interface_proposal": {
            "schema": "cynic.agent_proposal.v1",
            "proposal_id": "dependency-convergence-test",
            "visibility": "operator_private",
            "operator_review": {"state": "pending"},
            "public_projection": {"eligible": false, "redaction_status": "needs_review"}
        }
    }));

    let projection = task_proposal_projection(&task).expect("projection should exist");

    assert_eq!(
        projection.get("schema").and_then(|value| value.as_str()),
        Some("cynic.agent_proposal_task_projection.v1")
    );
    assert_eq!(
        projection
            .get("task")
            .and_then(|value| value.get("consumer"))
            .and_then(|value| value.as_str()),
        Some("organ-lifecycle-hermes-agent")
    );
    assert!(filter_proposal_projection(
        &projection,
        Some("operator_private"),
        Some("pending")
    ));
    assert!(!filter_proposal_projection(
        &projection,
        Some("public"),
        Some("pending")
    ));
}

#[test]
fn validates_agent_proposal_review_request() {
    let request = ReviewAgentProposalRequest {
        state: "approved".to_string(),
        reviewer: Some("operator".to_string()),
        reason: Some("proof inspected".to_string()),
        public_eligible: Some(true),
    };

    let result = validate_agent_proposal_review_request(&request);

    assert!(result.is_ok());
}

#[test]
fn rejects_public_eligible_without_approval() {
    let request = ReviewAgentProposalRequest {
        state: "needs_redaction".to_string(),
        reviewer: Some("operator".to_string()),
        reason: None,
        public_eligible: Some(true),
    };

    let result = validate_agent_proposal_review_request(&request);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires state=approved"));
}

#[test]
fn builds_agent_proposal_review_observation_payload() {
    let request = ReviewAgentProposalRequest {
        state: "rejected".to_string(),
        reviewer: None,
        reason: Some("insufficient evidence".to_string()),
        public_eligible: None,
    };

    let review = agent_proposal_review_value("proposal-1", &request);

    assert_eq!(
        review.get("schema").and_then(|value| value.as_str()),
        Some("cynic.agent_proposal_review.v1")
    );
    assert_eq!(
        review.get("proposal_id").and_then(|value| value.as_str()),
        Some("proposal-1")
    );
    assert_eq!(
        review.get("reviewer").and_then(|value| value.as_str()),
        Some("operator")
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
                "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
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
        "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
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
fn rejects_forge_task_that_does_not_match_dynamic_registry_contract() {
    let registry = serde_json::json!({
        "lifecycle": {
            "kernel_release": {
                "orchestrator_organ": "organ-build",
                "build_runner_role": "builder",
                "deploy_target_role": "primary",
                "deploy_target_flag": "ships_kernel",
                "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
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
        "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
        "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
        "activation_evidence_contract": activation_evidence_contract(),
        "coord_targets": ["infra/registry.json"]
    });

    let result = cynic_host::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(
        &content,
        Some(&registry),
    );

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("match registry orchestrator_organ")
    );
}

#[test]
fn allows_non_forge_task_without_declared_targets() {
    let result = validate_agent_task_content("hermes", "telegram", r#"{"objective":"observe"}"#);

    assert!(result.is_ok());
}

#[test]
fn rejects_invalid_json_for_forge_task() {
    let result = validate_agent_task_content("hermes", "organ-forge", "not-json");

    assert!(result.is_err());
}

#[test]
fn agent_task_created_event_matches_watch_filters() {
    let event = KernelEvent::AgentTaskCreated {
        task_id: "agent_tasks:test".to_string(),
        kind: "hermes".to_string(),
        domain: "forge".to_string(),
    };

    assert!(event_matches_pending_task_watch(
        &event,
        "hermes",
        Some("forge")
    ));
    assert!(!event_matches_pending_task_watch(
        &event,
        "judge",
        Some("forge")
    ));
    assert!(!event_matches_pending_task_watch(
        &event,
        "hermes",
        Some("token")
    ));
}

#[tokio::test]
async fn task_watch_waiter_wakes_for_matching_event() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(8);
    let waiter = tokio::spawn(async move {
        wait_for_matching_task_event(&mut rx, "hermes", Some("forge"), 1).await
    });

    let _ = tx.send(KernelEvent::AgentTaskCreated {
        task_id: "agent_tasks:test".to_string(),
        kind: "hermes".to_string(),
        domain: "forge".to_string(),
    });

    assert!(waiter.await.unwrap());
}

#[tokio::test]
async fn task_watch_waiter_times_out_without_matching_event() {
    let (_tx, mut rx) = tokio::sync::broadcast::channel(8);
    assert!(!wait_for_matching_task_event(&mut rx, "hermes", Some("forge"), 1).await);
}
