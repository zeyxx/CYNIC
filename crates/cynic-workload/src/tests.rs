use crate::{
    AgentBoundedAction, AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
    WorkloadMandateBinding,
};
use cynic_consensus::{MandateAccount, MandateStatus};
use cynic_protocol::{
    AGENT_COMPLETION_ENVELOPE_SCHEMA, AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentBudget,
    AgentCapabilityGrant, AgentCompletionEnvelope, AgentCompletionStatus, AgentDelegationEnvelope,
    AgentEvidenceRef, EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, LeaseGuard,
    MutationContract, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};

use super::{
    AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, SETTLED_PARCEL_RECEIPT_SCHEMA,
    acknowledge_agent_workload_cancellation, admit_agent_workload, admit_agent_workload_dispatch,
    admit_workload_action, admit_workload_mandate_binding, claim_agent_workload,
    complete_agent_workload, consume_agent_workload_action,
    finalize_unclaimed_agent_workload_cancellation, renew_agent_workload_claim,
    request_agent_workload_cancellation, settle_parcel, settle_parcel_with_receipt,
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
        mandate_binding: None,
        input: None,
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
                cpu_ms: None,
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
        completion: None,
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
fn mandate_binding_admits_exact_active_mandate_snapshot() {
    let mut workload = test_envelope();
    let mandate = MandateAccount {
        schema: "cynic.mandate_account.v1".to_string(),
        mandate_id: "mandate-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        office_id: "builder".to_string(),
        scope: "build:commit-a".to_string(),
        fencing_token: 7,
        started_at_unix_s: 100,
        expires_at_unix_s: 200,
        max_actions: 2,
        consumed_actions: 0,
        max_wall_seconds: 900,
        consumed_wall_seconds: 0,
        status: MandateStatus::Active,
    };
    workload.mandate_binding = Some(WorkloadMandateBinding {
        mandate_id: mandate.mandate_id.clone(),
        fencing_token: 7,
        snapshot_digest: digest_json(&mandate).expect("mandate snapshot digest"),
        requested_actions: 1,
        requested_wall_seconds: 60,
    });

    assert!(admit_workload_mandate_binding(&workload, &mandate, 150).admitted());
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
    let first = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let recovered = claim_agent_workload(&first, guard(2), 40, 21, 100_000).unwrap();
    assert!(!admit_workload_action(&recovered, &guard(1), 22).admitted());
    assert!(admit_workload_action(&recovered, &guard(2), 22).admitted());
}

#[test]
fn claim_renewal_preserves_fence_and_moves_expiry_forward() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    assert!(renew_agent_workload_claim(&claimed, &guard(1), 19, 11).is_err());
    let renewed = renew_agent_workload_claim(&claimed, &guard(1), 40, 11).unwrap();
    assert_eq!(renewed.claim.unwrap().expires_at_ms, 40);
    assert_eq!(renewed.last_fencing_token, 1);
}

#[test]
fn cancellation_invalidates_the_active_claim() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let cancelled = request_agent_workload_cancellation(&claimed).unwrap();
    assert!(!admit_workload_action(&cancelled, &guard(1), 11).admitted());
}

#[test]
fn executor_can_durably_acknowledge_the_exact_cancelled_claim() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let cancelled = request_agent_workload_cancellation(&claimed).unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Blocked,
        evidence_refs: Vec::new(),
        unknowns: vec!["workload_cancelled_generation_1".to_string()],
        consumed_budget: AgentBudget {
            context_tokens: 0,
            actions: 0,
            children: 0,
            wall_time_ms: 1,
            cpu_ms: None,
        },
    };

    let acknowledged =
        acknowledge_agent_workload_cancellation(&cancelled, &guard(1), &completion).unwrap();
    assert_eq!(acknowledged.state, AgentWorkloadState::Blocked);
    assert!(acknowledged.claim.is_none());
    assert_eq!(acknowledged.completion, Some(completion.clone()));
    assert!(acknowledge_agent_workload_cancellation(&cancelled, &guard(2), &completion).is_err());
}

#[test]
fn unclaimed_cancellation_is_finalized_without_inventing_a_lease() {
    let cancelled = request_agent_workload_cancellation(&record()).unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Blocked,
        evidence_refs: Vec::new(),
        unknowns: vec!["cancelled_before_claim".to_string()],
        consumed_budget: AgentBudget {
            context_tokens: 0,
            actions: 0,
            children: 0,
            wall_time_ms: 0,
            cpu_ms: None,
        },
    };

    let finalized =
        finalize_unclaimed_agent_workload_cancellation(&cancelled, &completion).unwrap();
    assert_eq!(finalized.state, AgentWorkloadState::Blocked);
    assert_eq!(finalized.completion, Some(completion));
}

#[test]
fn action_budget_is_retained_in_the_record() {
    let mut claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    claimed = consume_agent_workload_action(&claimed, &guard(1), 11).unwrap();
    claimed = consume_agent_workload_action(&claimed, &guard(1), 12).unwrap();
    assert!(consume_agent_workload_action(&claimed, &guard(1), 13).is_err());
}

#[test]
fn completion_requires_expected_read_back_proof() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
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
            cpu_ms: None,
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

#[test]
fn a_degraded_build_is_terminal_but_cannot_claim_completion() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Degraded,
        evidence_refs: vec![evidence("build_execution_provenance")],
        unknowns: vec!["kernel_poh_read_back_unavailable".to_string()],
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 10,
            cpu_ms: None,
        },
    };
    let terminal = complete_agent_workload(&claimed, &guard(1), &completion, 11).unwrap();
    assert_eq!(terminal.state, AgentWorkloadState::Degraded);
    assert!(terminal.state.terminal());
}

#[test]
fn parcel_settlement_succeeds_and_generates_hermetic_receipt() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("canonical_gate_report")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 50,
            cpu_ms: None,
        },
    };

    let (settled, receipt) =
        settle_parcel_with_receipt(&claimed, &guard(1), &completion, 15).expect("valid settlement");

    assert_eq!(settled.state, AgentWorkloadState::Completed);
    assert!(settled.claim.is_none());
    assert_eq!(settled.completion, Some(completion.clone()));

    assert_eq!(receipt.schema, SETTLED_PARCEL_RECEIPT_SCHEMA);
    assert_eq!(receipt.workload_id, "workload-1");
    assert_eq!(receipt.settled_at_ms, 15);
    assert_eq!(receipt.status, AgentCompletionStatus::Completed);
    assert_eq!(receipt.evidence_count, 1);
    assert_eq!(receipt.final_record_digest, digest_json(&settled).unwrap());
}

#[test]
fn parcel_settlement_rejects_plan_tampering_in_flight() {
    let mut claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    // Tamper with argv in flight
    claimed
        .envelope
        .action
        .argv
        .push("--malicious-flag".to_string());

    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("canonical_gate_report")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 50,
            cpu_ms: None,
        },
    };

    let err = settle_parcel(&claimed, &guard(1), &completion, 15).unwrap_err();
    assert_eq!(err.code, "plan_digest_tampered");
}

#[test]
fn parcel_settlement_rejects_stale_or_mismatched_guard() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("canonical_gate_report")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 50,
            cpu_ms: None,
        },
    };

    // Mismatched fencing token
    let err = settle_parcel(&claimed, &guard(2), &completion, 15).unwrap_err();
    assert_eq!(err.code, "claim_guard_rejected");
}

#[test]
fn parcel_settlement_rejects_expired_lease() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("canonical_gate_report")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 50,
            cpu_ms: None,
        },
    };

    // now_ms = 25 > expires_at_ms = 20
    let err = settle_parcel(&claimed, &guard(1), &completion, 25).unwrap_err();
    assert_eq!(err.code, "claim_expired");
}

#[test]
fn parcel_settlement_rejects_missing_expected_proof() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();
    // Return evidence other than the expected "canonical_gate_report"
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("arbitrary_log_ref")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 50,
            cpu_ms: None,
        },
    };

    let err = settle_parcel(&claimed, &guard(1), &completion, 15).unwrap_err();
    assert_eq!(err.code, "expected_proof_missing");
}

#[test]
fn parcel_settlement_rejects_exceeded_budgets() {
    let claimed = claim_agent_workload(&record(), guard(1), 20, 10, 100_000).unwrap();

    // 1. Exceeded action budget (delegated: 2, reported: 3)
    let action_overflow = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("canonical_gate_report")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 3,
            children: 0,
            wall_time_ms: 50,
            cpu_ms: None,
        },
    };
    assert!(settle_parcel(&claimed, &guard(1), &action_overflow, 15).is_err());

    // 2. Exceeded wall time (delegated: 900_000, reported: 900_001)
    let time_overflow = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "delegation-1".to_string(),
        node_id: "builder-1".to_string(),
        agent_id: "build-agent-1".to_string(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("canonical_gate_report")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 100,
            actions: 1,
            children: 0,
            wall_time_ms: 900_001,
            cpu_ms: None,
        },
    };
    assert!(settle_parcel(&claimed, &guard(1), &time_overflow, 15).is_err());
}
