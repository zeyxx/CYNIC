#![allow(clippy::expect_used)]
use cynic_protocol::{
    build_recipe, canonical_build_action_from_argv, derive_build_workload_grant,
    validate_build_recipe, validate_signed_build_workload_grant, AgentBoundedAction,
    AgentCompletionStatus, AgentDelegationEnvelope, AgentWorkloadActionInput, AgentWorkloadClaim,
    AgentWorkloadEnvelope, AgentWorkloadRecord, BuildWorkloadGrant, HarnessAction, HarnessEvidence,
    HarnessEvidenceKind, HarnessEvidenceStatus, HarnessObservation, HarnessScenario,
    HarnessScenarioId, HarnessStatus, LeaseGuard,
};
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;

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
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let authority_public_key = hex::encode(verifying_key.as_bytes());

    let node_id = "cynic-builder";
    let workload_id = "workload-check-20260816";
    let source_commit = "f03536eac0efdf2c1e8d353760345205d05909c8";

    let envelope = AgentWorkloadEnvelope {
        schema: cynic_protocol::AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        workload_id: workload_id.to_string(),
        delegation: AgentDelegationEnvelope {
            schema: cynic_protocol::AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: "delegation-root".to_string(),
            node_id: node_id.to_string(),
            parent_workload_id: None,
            depth: 0,
            actor: "test-actor".to_string(),
            capabilities: vec!["build_runner".to_string()],
            budgets: Default::default(),
            context_refs: Vec::new(),
            intent: "canonical build validation".to_string(),
        },
        action: AgentBoundedAction {
            schema: cynic_protocol::AGENT_WORKLOAD_INPUT_SCHEMA.to_string(),
            argv: vec!["make".to_string(), "check".to_string()],
            semantic_action: Some(AgentWorkloadActionInput {
                action_kind: "make:check".to_string(),
                target: Some("build".to_string()),
                parameters: None,
            }),
            working_directory: Some("/source/workspaces/CYNIC".to_string()),
            source_preconditions: Vec::new(),
            action_budget_cost: 1,
            timeout_ms: 300_000,
        },
        idempotency_key: "idem-check-001".to_string(),
        reclaim_lease: None,
    };

    let record = AgentWorkloadRecord {
        schema: cynic_protocol::AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope,
        state: cynic_protocol::AgentWorkloadState::Claimed,
        claim: Some(AgentWorkloadClaim {
            schema: cynic_protocol::AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA.to_string(),
            lease: LeaseGuard {
                lease_id: "lease-001".into(),
                fencing_token: 1,
            },
            claimant_id: "cynic-node".to_string(),
            claimed_at_ms: 1_786_800_000_000,
            expires_at_ms: 1_786_800_300_000,
        }),
        action_budget_remaining: 10,
        cancellation: None,
        completion: None,
    };

    let grant = derive_build_workload_grant(&record, node_id, source_commit, 1_786_800_000)
        .expect("grant derivation");

    assert_eq!(grant.node_id, node_id);
    assert_eq!(grant.workload_id, workload_id);
    assert_eq!(grant.canonical_action, "make:check");
    assert_eq!(grant.source_commit, source_commit);

    // Sign the grant
    let signing_bytes = cynic_protocol::build_workload_grant_signing_bytes(&grant)
        .expect("signing bytes");
    let signature = signing_key.sign(&signing_bytes);
    let signature_hex = hex::encode(signature.to_bytes());

    let signed_grant = cynic_protocol::SignedBuildWorkloadGrant {
        schema: cynic_protocol::SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        grant,
        authority_public_key: authority_public_key.clone(),
        signature: signature_hex,
    };

    let validation = validate_signed_build_workload_grant(
        &signed_grant,
        &record,
        &authority_public_key,
        node_id,
        source_commit,
        "make:check",
    );

    assert!(
        validation.is_ok(),
        "signed build grant must validate successfully: {:?}",
        validation
    );
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
