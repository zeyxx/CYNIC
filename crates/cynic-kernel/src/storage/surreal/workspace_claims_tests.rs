use serde_json::json;

use super::*;

fn claim() -> serde_json::Value {
    json!({
        "repository_id": "cynic",
        "workspace_id": "workspace-a",
        "workload_id": "workload-a",
        "node_id": "node-a",
        "agent_id": "agent-a",
        "lease_id": "workspace-1",
        "fencing_token": 7,
        "expires_at_ms": 200,
        "hard_expires_at_ms": 500,
        "zones": [{"kind": "tree", "path": "crates/cynic-protocol"}]
    })
}

#[test]
fn parses_fenced_workspace_claim_record() {
    let parsed = parse_claim(&claim()).unwrap();
    assert_eq!(parsed.guard.fencing_token, 7);
    assert_eq!(parsed.guard.target, "source-worktree:cynic:workspace-a");
    assert_eq!(parsed.hard_expires_at_ms, 500);
}

#[test]
fn parses_structured_conflict_outcome() {
    let outcome = parse_reservation_outcome(vec![vec![json!({
        "kind": "conflict",
        "conflicts": [claim()]
    })]])
    .unwrap();
    assert!(matches!(
        outcome,
        AgentWorkspaceClaimOutcome::Conflict { conflicts }
            if conflicts.len() == 1 && conflicts[0].guard.fencing_token == 7
    ));
}

#[test]
fn rejects_claim_record_without_hard_expiry() {
    let mut invalid = claim();
    invalid
        .as_object_mut()
        .expect("object")
        .remove("hard_expires_at_ms");
    assert!(parse_claim(&invalid).is_err());
}
