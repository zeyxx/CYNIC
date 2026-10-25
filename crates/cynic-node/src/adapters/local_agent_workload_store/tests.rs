use cynic_protocol::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope,
    AgentEvidenceRef, EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};
use cynic_workload::{
    AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction,
    AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
};

use super::*;

fn record(workload_id: &str, idempotency_key: &str) -> AgentWorkloadRecord {
    let action = AgentBoundedAction {
        argv: vec!["cargo".into(), "check".into()],
        working_directory: Some("checkout".into()),
        input_digest: Some(format!("sha256:{}", "b".repeat(64))),
        mutates: true,
        requires_privilege: false,
        stop_condition: "stop on first non-zero exit".into(),
        rollback_intent: "discard isolated checkout".into(),
    };
    let plan_digest = digest_json(&action).expect("action digest");
    let mutation = MutationContract {
        kind: "repository_validation".into(),
        scope: PolicyScope {
            domain: "build".into(),
            target: "commit-a".into(),
        },
        mutates: true,
        requires_privilege: false,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimePrincipal,
        preconditions: vec!["source_commit_observed".into()],
        rollback_intent: "discard isolated checkout".into(),
        expected_proofs: vec![RequiredProof {
            gate: "canonical_gate_report".into(),
            subject: "commit-a".into(),
        }],
    };
    let envelope = AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.into(),
        workload_id: workload_id.into(),
        idempotency_key: idempotency_key.into(),
        plan_digest: plan_digest.clone(),
        mandate_binding: None,
        input: None,
        resource_budget: None,
        source_commit: None,
        branch: None,
        action,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.into(),
            delegation_id: format!("delegation-{workload_id}"),
            node_id: "builder-1".into(),
            agent_id: "build-agent-1".into(),
            parent_agent_id: None,
            depth: 0,
            objective: "run canonical validation".into(),
            authority_snapshot: evidence(workload_id),
            context_refs: vec![],
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".into(),
                mutates: true,
            }],
            budget: AgentBudget {
                context_tokens: 1_000,
                actions: 2,
                children: 0,
                wall_time_ms: 900_000,
                cpu_ms: None,
            },
            expected_proofs: vec!["canonical_gate_report".into()],
        },
        parent_delegation: None,
        policy_admission: PolicyAdmissionReport {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.into(),
            policy_set: PolicySetRef {
                id: "build.validation".into(),
                version: "v1".into(),
                digest: format!("sha256:{}", "c".repeat(64)),
            },
            intent: PolicyIntentRef {
                id: idempotency_key.into(),
                kind: "repository_validation".into(),
                actor: "build-agent-1".into(),
                subject: "commit-a".into(),
                digest: format!("sha256:{}", "d".repeat(64)),
                plan_digest,
            },
            mutation: mutation.clone(),
            decision: PolicyDecision::Allow {
                required_authority: PolicyAuthority::RuntimePrincipal,
                required_proofs: mutation.expected_proofs.clone(),
            },
        },
        execution_authorization: ExecutionAuthorization {
            schema: EXECUTION_AUTHORIZATION_SCHEMA.into(),
            actor: "build-agent-1".into(),
            authority: PolicyAuthority::RuntimePrincipal,
            explicit_apply: true,
            satisfied_preconditions: vec!["source_commit_observed".into()],
        },
    };
    AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.into(),
        envelope,
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: None,
    }
}

fn evidence(subject: &str) -> AgentEvidenceRef {
    AgentEvidenceRef {
        kind: "authority_snapshot".into(),
        subject: subject.into(),
        digest: format!("sha256:{}", "a".repeat(64)),
        lineage: None,
    }
}

#[test]
fn archive_terminal_moves_eligible_terminal_record() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());

    let mut terminal_record = record("workload-old", "key-old");
    terminal_record.state = AgentWorkloadState::Completed;
    store
        .compare_and_swap("workload-old", None, &terminal_record, 1_000)
        .expect("seed terminal record");

    let archived = store
        .archive_terminal("workload-old", 500, 2_000)
        .expect("archive terminal");
    let path = archived.expect("archive path returned");
    assert!(path.contains("archive"));
    assert!(path.contains("workload-old"));
    assert!(std::path::Path::new(&path).exists());
    assert!(!root.path().join("workload-old").exists());
}

#[test]
fn archive_terminal_returns_none_for_live_record() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());

    store
        .compare_and_swap("workload-live", None, &record("workload-live", "key-live"), 1_000)
        .expect("seed live record");

    let result = store
        .archive_terminal("workload-live", 500, 2_000)
        .expect("archive terminal live");
    assert!(result.is_none());
    assert!(root.path().join("workload-live").exists());
}

#[test]
fn archive_terminal_returns_none_for_missing_workload() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());

    let result = store
        .archive_terminal("workload-missing", 500, 2_000)
        .expect("archive terminal missing");
    assert!(result.is_none());
}

#[test]
fn append_evidence_stores_sidecar_next_to_record() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());

    store
        .compare_and_swap("workload-evi", None, &record("workload-evi", "key-evi"), 1_000)
        .expect("seed record");

    let body = serde_json::json!({"workload_id": "workload-evi", "status": "failed"});
    store
        .append_evidence("workload-evi", "settlement-1.json", &body)
        .expect("append evidence");

    let bytes = std::fs::read(root.path().join("workload-evi").join("settlement-1.json"))
        .expect("sidecar readable");
    let read_back: serde_json::Value = serde_json::from_slice(&bytes).expect("sidecar json");
    assert_eq!(read_back, body);
}

#[test]
fn append_evidence_fails_closed_on_unknown_workload_and_unsafe_names() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());

    store
        .compare_and_swap("workload-evi", None, &record("workload-evi", "key-evi"), 1_000)
        .expect("seed record");

    let body = serde_json::json!({});
    assert_eq!(
        store
            .append_evidence("workload-missing", "settlement-1.json", &body)
            .expect_err("unknown workload")
            .code,
        "workload_evidence_unknown_workload"
    );
    for unsafe_name in ["..", ".", "sub/evil.json", "no-extension"] {
        assert!(
            store
                .append_evidence("workload-evi", unsafe_name, &body)
                .is_err(),
            "rejects {unsafe_name}"
        );
    }
}

#[test]
fn archive_terminal_returns_none_for_fresh_terminal_record() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());

    let mut terminal_record = record("workload-fresh", "key-fresh");
    terminal_record.state = AgentWorkloadState::Blocked;
    store
        .compare_and_swap("workload-fresh", None, &terminal_record, 1_900)
        .expect("seed fresh terminal record");

    let result = store
        .archive_terminal("workload-fresh", 500, 2_000)
        .expect("archive terminal fresh");
    assert!(result.is_none());
    assert!(root.path().join("workload-fresh").exists());
}

#[test]
fn append_only_cas_rejects_stale_revision_and_duplicate_idempotency() {
    let root = tempfile::tempdir().expect("tempdir");
    let store = LocalAgentWorkloadStore::new(root.path());
    let first = store
        .compare_and_swap("workload-1", None, &record("workload-1", "key-1"), 1)
        .expect("initial");
    assert_eq!(first.revision, 1);
    assert_eq!(store.read("workload-1").expect("read"), Some(first));

    let stale = store
        .compare_and_swap("workload-1", None, &record("workload-1", "key-1"), 2)
        .expect_err("stale create");
    assert_eq!(stale.code, "workload_store_revision_conflict");

    let duplicate = store
        .compare_and_swap("workload-2", None, &record("workload-2", "key-1"), 2)
        .expect_err("duplicate key");
    assert_eq!(duplicate.code, "workload_store_duplicate_idempotency_key");
}
