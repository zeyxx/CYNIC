use crate::{
    AGENT_COMPLETION_ENVELOPE_SCHEMA, AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentBoundedAction,
    AgentBudget, AgentCapabilityGrant, AgentCompletionEnvelope, AgentCompletionStatus,
    AgentDelegationEnvelope, AgentEvidenceRef, AgentWorkloadEnvelope, AgentWorkloadRecord,
    AgentWorkloadState, BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE,
    DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA, DurableAgentWorkloadRecord,
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, LeaseGuard, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA,
    SignedBuildWorkloadGrant, build_workload_grant_signing_bytes, derive_build_workload_grant,
    digest_json, validate_build_workload_grant, validate_signed_build_workload_grant,
};

use super::{
    AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, admit_agent_workload,
    admit_agent_workload_dispatch, admit_workload_action, claim_agent_workload,
    complete_agent_workload, consume_agent_workload_action, renew_agent_workload_claim,
    request_agent_workload_cancellation,
};

fn evidence(kind: &str) -> AgentEvidenceRef {
    AgentEvidenceRef {
        kind: kind.to_string(),
        subject: "workload-1".to_string(),
        digest: format!("sha256:{}", "a".repeat(64)),
        lineage: None,
    }
}

pub(super) fn test_envelope() -> AgentWorkloadEnvelope {
    let action = AgentBoundedAction {
        argv: vec!["make".to_string(), "check".to_string()],
        working_directory: Some("checkout".to_string()),
        input_digest: Some(format!("sha256:{}", "b".repeat(64))),
        mutates: true,
        requires_privilege: false,
        stop_condition: "stop on first non-zero exit".to_string(),
        rollback_intent: "discard isolated build checkout".to_string(),
    };
    let plan_digest = match digest_json(&action) {
        Ok(digest) => digest,
        Err(error) => panic!("bounded action must serialize in test: {error}"),
    };
    let mutation = MutationContract {
        kind: "repository_validation".to_string(),
        scope: PolicyScope {
            domain: "build".to_string(),
            target: "commit-a".to_string(),
        },
        mutates: true,
        requires_privilege: false,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimePrincipal,
        preconditions: vec!["source_commit_observed".to_string()],
        rollback_intent: "discard isolated build checkout".to_string(),
        expected_proofs: vec![RequiredProof {
            gate: "canonical_gate_report".to_string(),
            subject: "commit-a".to_string(),
        }],
    };
    AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        workload_id: "workload-1".to_string(),
        idempotency_key: "validate-commit-a".to_string(),
        plan_digest: plan_digest.clone(),
        action,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: "delegation-1".to_string(),
            node_id: "builder-1".to_string(),
            agent_id: "build-agent-1".to_string(),
            parent_agent_id: None,
            depth: 0,
            objective: "run canonical validation".to_string(),
            authority_snapshot: evidence("authority_snapshot"),
            context_refs: vec![evidence("source_commit")],
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".to_string(),
                mutates: true,
            }],
            budget: AgentBudget {
                context_tokens: 1_000,
                actions: 2,
                children: 0,
                wall_time_ms: 900_000,
            },
            expected_proofs: vec!["canonical_gate_report".to_string()],
        },
        parent_delegation: None,
        policy_admission: PolicyAdmissionReport {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
            policy_set: PolicySetRef {
                id: "build.validation".to_string(),
                version: "v1".to_string(),
                digest: format!("sha256:{}", "c".repeat(64)),
            },
            intent: PolicyIntentRef {
                id: "validate-commit-a".to_string(),
                kind: "repository_validation".to_string(),
                actor: "build-agent-1".to_string(),
                subject: "commit-a".to_string(),
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
            schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
            actor: "build-agent-1".to_string(),
            authority: PolicyAuthority::RuntimePrincipal,
            explicit_apply: true,
            satisfied_preconditions: vec!["source_commit_observed".to_string()],
        },
    }
}

pub(super) fn record() -> AgentWorkloadRecord {
    AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope: test_envelope(),
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
    }
}

fn guard(token: u64) -> LeaseGuard {
    LeaseGuard {
        agent_id: "build-agent-1".to_string(),
        target: "workload-1".to_string(),
        lease_id: format!("lease-{token}"),
        fencing_token: token,
    }
}

#[test]
fn admits_a_fully_bounded_workload() {
    assert!(admit_agent_workload(&test_envelope()).admitted());
}

#[test]
fn mutation_requires_a_mutating_delegated_capability() {
    let mut candidate = test_envelope();
    candidate.delegation.capabilities[0].mutates = false;
    assert!(!admit_agent_workload(&candidate).admitted());
}

#[test]
fn mutation_requires_a_content_addressed_semantic_input() {
    let mut candidate = test_envelope();
    candidate.action.input_digest = None;
    let digest = match digest_json(&candidate.action) {
        Ok(digest) => digest,
        Err(error) => panic!("bounded action must serialize in test: {error}"),
    };
    candidate.plan_digest = digest.clone();
    candidate.policy_admission.intent.plan_digest = digest;
    assert!(!admit_agent_workload(&candidate).admitted());
}

#[test]
fn shell_composition_is_not_an_admitted_action() {
    let mut candidate = test_envelope();
    candidate.action.argv = vec![
        "bash".to_string(),
        "-c".to_string(),
        "make check".to_string(),
    ];
    let digest = match digest_json(&candidate.action) {
        Ok(digest) => digest,
        Err(error) => panic!("bounded action must serialize in test: {error}"),
    };
    candidate.plan_digest = digest.clone();
    candidate.policy_admission.intent.plan_digest = digest;
    assert!(!admit_agent_workload(&candidate).admitted());
}

#[test]
fn dispatch_rejects_an_active_idempotency_key() {
    assert!(
        !admit_agent_workload_dispatch(&test_envelope(), &["validate-commit-a".to_string()])
            .admitted()
    );
}

#[test]
fn stale_claim_cannot_act_after_recovery() {
    let first = claim_agent_workload(&record(), guard(1), 20, 10).unwrap();
    let recovered = claim_agent_workload(&first, guard(2), 40, 21).unwrap();
    assert!(!admit_workload_action(&recovered, &guard(1), 22).admitted());
    assert!(admit_workload_action(&recovered, &guard(2), 22).admitted());
}

#[test]
fn claim_renewal_preserves_fence_and_moves_expiry_forward() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10).unwrap();
    assert!(renew_agent_workload_claim(&claimed, &guard(1), 19, 11).is_err());
    let renewed = renew_agent_workload_claim(&claimed, &guard(1), 40, 11).unwrap();
    assert_eq!(renewed.claim.unwrap().expires_at_ms, 40);
    assert_eq!(renewed.last_fencing_token, 1);
}

#[test]
fn cancellation_invalidates_the_active_claim() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10).unwrap();
    let cancelled = request_agent_workload_cancellation(&claimed).unwrap();
    assert!(!admit_workload_action(&cancelled, &guard(1), 11).admitted());
}

#[test]
fn action_budget_is_retained_in_the_record() {
    let mut claimed = claim_agent_workload(&record(), guard(1), 20, 10).unwrap();
    claimed = consume_agent_workload_action(&claimed, &guard(1), 11).unwrap();
    claimed = consume_agent_workload_action(&claimed, &guard(1), 12).unwrap();
    assert!(consume_agent_workload_action(&claimed, &guard(1), 13).is_err());
}

#[test]
fn build_grant_is_derived_from_the_live_durable_workload() {
    let now_unix_s = 10;
    let source_commit = "a".repeat(40);
    let mut dispatched = record();
    dispatched.envelope.policy_admission.intent.subject = source_commit.clone();
    dispatched.envelope.policy_admission.mutation.scope.target = source_commit.clone();
    let claimed = claim_agent_workload(&dispatched, guard(1), 40_000, now_unix_s * 1_000)
        .expect("claim workload");
    let durable = DurableAgentWorkloadRecord {
        schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 2,
        previous_record_digest: Some(format!("sha256:{}", "e".repeat(64))),
        record_digest: digest_json(&claimed).expect("record digest"),
        record: claimed,
        committed_at_ms: now_unix_s * 1_000,
    };

    let grant = derive_build_workload_grant(&durable, "scheduler-1", now_unix_s)
        .expect("derive workload grant");

    assert_eq!(grant.source_commit, source_commit);
    assert_eq!(
        grant.allowed_actions,
        vec!["make:check", "cargo:check", "cargo:run", "cargo:test"]
    );
    assert_eq!(grant.workload_record_digest, durable.record_digest);
    assert!(
        validate_build_workload_grant(
            &grant,
            &durable,
            "builder-1",
            &source_commit,
            "make:check",
            now_unix_s + 1,
        )
        .is_ok()
    );

    let mut signed = SignedBuildWorkloadGrant {
        schema: SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        authority_key_id: format!("sha256:{}", "c".repeat(64)),
        signature_namespace: BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: "d".repeat(128),
    };
    let signing_bytes = build_workload_grant_signing_bytes(&signed).expect("signing bytes");
    signed.signature_hex = "e".repeat(128);
    assert_eq!(
        signing_bytes,
        build_workload_grant_signing_bytes(&signed).expect("stable signing bytes")
    );
    assert!(validate_signed_build_workload_grant(&signed).is_empty());
}

#[test]
fn cancellation_or_fence_change_revokes_a_derived_build_grant() {
    let now_unix_s = 10;
    let source_commit = "b".repeat(40);
    let mut dispatched = record();
    dispatched.envelope.policy_admission.intent.subject = source_commit.clone();
    dispatched.envelope.policy_admission.mutation.scope.target = source_commit.clone();
    let claimed = claim_agent_workload(&dispatched, guard(1), 40_000, now_unix_s * 1_000)
        .expect("claim workload");
    let mut durable = DurableAgentWorkloadRecord {
        schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 2,
        previous_record_digest: Some(format!("sha256:{}", "e".repeat(64))),
        record_digest: digest_json(&claimed).expect("record digest"),
        record: claimed,
        committed_at_ms: now_unix_s * 1_000,
    };
    let grant = derive_build_workload_grant(&durable, "scheduler-1", now_unix_s)
        .expect("derive workload grant");

    durable.record = request_agent_workload_cancellation(&durable.record).expect("cancel");
    durable.revision += 1;
    durable.previous_record_digest = Some(durable.record_digest.clone());
    durable.record_digest = digest_json(&durable.record).expect("cancelled digest");

    assert!(
        validate_build_workload_grant(
            &grant,
            &durable,
            "builder-1",
            &source_commit,
            "make:check",
            now_unix_s + 1,
        )
        .is_err()
    );
}

#[test]
fn completion_requires_expected_read_back_proof() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10).unwrap();
    let mut completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("unrelated")],
        unknowns: vec![],
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 10,
        },
    };
    assert!(complete_agent_workload(&claimed, &guard(1), &completion, 11).is_err());
    completion.evidence_refs = vec![evidence("canonical_gate_report")];
    assert_eq!(
        complete_agent_workload(&claimed, &guard(1), &completion, 11)
            .unwrap()
            .state,
        AgentWorkloadState::Completed
    );
}
