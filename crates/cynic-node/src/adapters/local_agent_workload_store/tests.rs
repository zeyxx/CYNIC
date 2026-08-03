use cynic_protocol::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA,
    AgentBoundedAction, AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope,
    AgentEvidenceRef, AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};

use super::*;

fn record(workload_id: &str, idempotency_key: &str) -> AgentWorkloadRecord {
    let action = AgentBoundedAction {
        argv: vec!["make".into(), "check".into()],
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
        input: None,
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
