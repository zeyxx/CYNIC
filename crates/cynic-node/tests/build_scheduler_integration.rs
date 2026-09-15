#![allow(clippy::expect_used)]
use cynic_build::{
    build_recipe,
    canonical_build_action_from_argv,
    derive_build_workload_grant,
    validate_build_recipe,
};
use cynic_workload::{
    AGENT_WORKLOAD_ENVELOPE_SCHEMA,
    AGENT_WORKLOAD_RECORD_SCHEMA,
    AgentBoundedAction,
    AgentWorkloadEnvelope,
    AgentWorkloadRecord,
    AgentWorkloadState,
    DurableAgentWorkloadRecord,
};
use cynic_protocol::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA,
    AgentBudget,
    AgentCapabilityGrant,
    AgentDelegationEnvelope,
    AgentEvidenceRef,
    EXECUTION_AUTHORIZATION_SCHEMA,
    ExecutionAuthorization,
    HarnessEvidence,
    HarnessEvidenceKind,
    HarnessEvidenceStatus,
    HarnessObservation,
    HarnessScenario,
    HarnessScenarioId,
    HarnessStatus,
    LeaseGuard,
    MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA,
    PolicyAdmissionReport,
    PolicyAuthority,
    PolicyDecision,
    PolicyIntentRef,
    PolicyScope,
    PolicySetRef,
    RequiredProof,
    digest_json,
};

#[test]
fn workload_action_maps_to_canonical_build_recipe() {
    let argv = vec!["make".to_string(), "check".to_string()];
    let action_str = canonical_build_action_from_argv(&argv).expect("canonical action");
    assert_eq!(action_str, "make:check");

    let recipe = build_recipe(action_str).expect("recipe");
    assert_eq!(recipe.action, "make:check");
    assert_eq!(recipe.target_profile, "debug");
    assert_eq!(recipe.rust_min_stack, 536_870_912);
    assert!(!recipe.cargo_incremental);
    assert!(validate_build_recipe(&recipe).is_ok());
}

#[test]
fn signed_build_grant_derivation_and_verification() {
    let node_id = "cynic-builder";
    let workload_id = "workload-check-20260816";
    let source_commit = "f03536eac0efdf2c1e8d353760345205d05909c8";

    let action = AgentBoundedAction {
        argv: vec!["make".into(), "check".into()],
        working_directory: Some("/source/workspaces/CYNIC".into()),
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
            target: source_commit.to_string(),
        },
        mutates: true,
        requires_privilege: false,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimePrincipal,
        preconditions: vec!["source_commit_observed".into()],
        rollback_intent: "discard isolated checkout".into(),
        expected_proofs: vec![RequiredProof {
            gate: "canonical_gate_report".into(),
            subject: source_commit.to_string(),
        }],
    };
    let envelope = AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.into(),
        workload_id: workload_id.into(),
        idempotency_key: "idem-check-001".into(),
        plan_digest: plan_digest.clone(),
        mandate_binding: None,
        input: None,
        action,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.into(),
            delegation_id: format!("delegation-{workload_id}"),
            node_id: node_id.into(),
            agent_id: "build-agent-1".into(),
            parent_agent_id: None,
            depth: 0,
            objective: "run canonical validation".into(),
            authority_snapshot: AgentEvidenceRef {
                kind: "policy".into(),
                subject: workload_id.into(),
                digest: format!("sha256:{}", "c".repeat(64)),
                lineage: None,
            },
            context_refs: vec![],
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".into(),
                mutates: true,
            }],
            budget: AgentBudget {
                context_tokens: 1_000,
                actions: 2,
                children: 0,
                wall_time_ms: 300_000,
                cpu_ms: None,
            },
            expected_proofs: vec!["canonical_gate_report".into()],
        },
        parent_delegation: None,
        policy_admission: PolicyAdmissionReport {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.into(),
            policy_set: PolicySetRef {
                id: "cynic.build.policy".into(),
                version: "v1".into(),
                digest: format!("sha256:{}", "e".repeat(64)),
            },
            intent: PolicyIntentRef {
                id: format!("intent-{workload_id}"),
                kind: "repository_validation".into(),
                actor: "build-agent-1".into(),
                subject: source_commit.to_string(),
                digest: format!("sha256:{}", "f".repeat(64)),
                plan_digest: plan_digest.clone(),
            },
            mutation,
            decision: PolicyDecision::Allow {
                required_authority: PolicyAuthority::RuntimePrincipal,
                required_proofs: vec![RequiredProof {
                    gate: "canonical_gate_report".into(),
                    subject: source_commit.to_string(),
                }],
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

    let record = AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.into(),
        envelope,
        state: AgentWorkloadState::Claimed,
        claim: Some(cynic_workload::AgentWorkloadClaim {
            guard: LeaseGuard {
                lease_id: "lease-001".into(),
                fencing_token: 1,
                agent_id: "build-agent-1".into(),
                target: workload_id.to_string(),
            },
            expires_at_ms: 1_786_800_300_000,
            cancellation_generation: 0,
            max_lease_window_ms: 3_600_000,
            granted_at_ms: 1_786_800_000_000,
        }),
        consumed_actions: 1,
        last_fencing_token: 1,
        cancellation_generation: 0,
        completion: None,
    };

    let record_digest = digest_json(&record).expect("record digest");
    let durable = DurableAgentWorkloadRecord {
        schema: cynic_workload::DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.into(),
        committed_at_ms: 1_786_800_000_000,
        previous_record_digest: None,
        record_digest,
        revision: 1,
        record,
    };

    let grant =
        derive_build_workload_grant(&durable, node_id, 1_786_800_100).expect("grant derivation");

    assert_eq!(grant.issuer_node_id, node_id);
    assert_eq!(grant.workload_id, workload_id);
}

#[test]
fn scheduler_conformance_scenarios_reduction() {
    let scenarios = vec![
        HarnessScenario::closed_lab_vm(),
        HarnessScenario::lxc_clone_conformance(),
        HarnessScenario::builder_staging_observe(),
        HarnessScenario::builder_canary_apply(),
        HarnessScenario::failure_recovery(),
        HarnessScenario::proof_replay(),
    ];

    let observations = vec![
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::ClosedLabVm),
            target_observed: true,
            evidences: vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Observation,
                status: HarnessEvidenceStatus::Completed,
                reference: "proof:lab-vm".to_string(),
                digest: None,
                detail: None,
            }],
            ..Default::default()
        },
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::LxcCloneConformance),
            target_observed: true,
            evidences: vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Observation,
                status: HarnessEvidenceStatus::Completed,
                reference: "proof:lxc-clone".to_string(),
                digest: None,
                detail: None,
            }],
            ..Default::default()
        },
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::BuilderStagingObserve),
            target_observed: true,
            staging_proof_completed: true,
            evidences: vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Observation,
                status: HarnessEvidenceStatus::Completed,
                reference: "proof:staging".to_string(),
                digest: None,
                detail: None,
            }],
            ..Default::default()
        },
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::BuilderCanaryApply),
            target_observed: true,
            staging_proof_completed: true,
            evidences: vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Execution,
                status: HarnessEvidenceStatus::Completed,
                reference: "proof:canary-execution".to_string(),
                digest: None,
                detail: None,
            }],
            ..Default::default()
        },
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::FailureRecovery),
            target_observed: true,
            evidences: vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Fault,
                status: HarnessEvidenceStatus::Completed,
                reference: "proof:cancellation-acknowledged".to_string(),
                digest: None,
                detail: None,
            }],
            ..Default::default()
        },
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::ProofReplay),
            target_observed: true,
            evidences: vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Replay,
                status: HarnessEvidenceStatus::Completed,
                reference: "proof:replay-matched".to_string(),
                digest: Some("sha256:matched".to_string()),
                detail: None,
            }],
            ..Default::default()
        },
    ];

    let matrix = cynic_protocol::reconcile_harness_matrix(&scenarios, &observations);
    assert_eq!(matrix.status, HarnessStatus::Completed);
    assert_eq!(matrix.cells.len(), 6);
    assert!(matrix.blockers.is_empty());
}
