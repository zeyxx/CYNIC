//! Admit a check-phase build into the durable workload store.
//!
//! The pull loop only selects what is admitted here, and delivery leases
//! only what the store holds. This constructor binds an exact source SHA to
//! the canonical cargo argv, validates the workload, and appends it with
//! compare-and-swap. A duplicate SHA plus package replays the existing
//! record instead of writing a second one. Only `cargo check` is eligible:
//! later phases mutate and need their own admission slice. Nothing spawns.

use cynic_build::build_slice::{
    BUILD_SLICE_ACTION_SCHEMA, BuildSliceAction, BuildSliceNetworkMode, BuildSlicePhase,
    BuildSliceScope, BuildSliceTarget,
};
use cynic_protocol::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope,
    AgentEvidenceRef, EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};
use cynic_workload::{
    AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction,
    AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState, DurableAgentWorkloadRecord,
    admit_agent_workload,
};

use crate::ports::build_slice_runner::build_slice_cargo_argv;

use crate::ports::agent_workload_store_port::{AgentWorkloadStoreError, AgentWorkloadStorePort};

pub const BUILD_ADMISSION_SCHEMA: &str = "cynic.node.build_admission.v1";

fn store_error(error: &AgentWorkloadStoreError) -> Vec<String> {
    vec![format!(
        "build_admission_store_failed:{}:{}",
        error.code, error.detail
    )]
}

/// Compose and append an admitted check workload, idempotent by SHA+package.
///
/// Returns the existing record when the same build was already admitted.
#[allow(clippy::too_many_arguments)]
pub fn admit_build_check(
    source_commit: &str,
    package: &str,
    runner_ref: &str,
    node_id: &str,
    agent_id: &str,
    committed_at_ms: u64,
    store: &dyn AgentWorkloadStorePort,
) -> Result<DurableAgentWorkloadRecord, Vec<String>> {
    let short = source_commit.chars().take(12).collect::<String>();
    let workload_id = format!("build-{short}-{package}");
    let idempotency_key = format!("build:{source_commit}:{package}:check");
    let action = BuildSliceAction {
        schema: BUILD_SLICE_ACTION_SCHEMA.to_string(),
        source_commit: source_commit.to_string(),
        phase: BuildSlicePhase::Check,
        scope: BuildSliceScope::Package,
        package: package.to_string(),
        target: BuildSliceTarget::Lib,
        test_filter: None,
        network_mode: BuildSliceNetworkMode::Offline,
        runner_ref: runner_ref.to_string(),
        idempotency_key: idempotency_key.clone(),
        timeout_secs: 600,
        max_jobs: 1,
    };
    let violations = action.validate();
    if !violations.is_empty() {
        return Err(violations);
    }
    let argv = build_slice_cargo_argv(&action).map_err(|error| vec![error.to_string()])?;
    let bounded = AgentBoundedAction {
        argv,
        working_directory: None,
        input_digest: None,
        mutates: false,
        requires_privilege: false,
        stop_condition: "stop on first non-zero exit".to_string(),
        rollback_intent: "discard isolated checkout".to_string(),
    };
    let plan_digest =
        digest_json(&bounded).map_err(|_| vec!["build_admission_digest_failed".to_string()])?;
    let envelope = AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        workload_id: workload_id.clone(),
        idempotency_key: idempotency_key.clone(),
        plan_digest: plan_digest.clone(),
        mandate_binding: None,
        input: None,
        action: bounded,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: format!("delegation-{workload_id}"),
            node_id: node_id.to_string(),
            agent_id: agent_id.to_string(),
            parent_agent_id: None,
            depth: 0,
            objective: "run canonical check".to_string(),
            authority_snapshot: AgentEvidenceRef {
                kind: "authority_snapshot".to_string(),
                subject: workload_id.clone(),
                digest: format!("sha256:{}", "e".repeat(64)),
                lineage: None,
            },
            context_refs: Vec::new(),
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".to_string(),
                mutates: false,
            }],
            budget: AgentBudget {
                context_tokens: 1_000,
                actions: 10,
                children: 0,
                wall_time_ms: 1_800_000,
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
                id: idempotency_key.clone(),
                kind: "repository_validation".to_string(),
                actor: agent_id.to_string(),
                subject: source_commit.to_string(),
                digest: format!("sha256:{}", "d".repeat(64)),
                plan_digest,
            },
            mutation: MutationContract {
                kind: "repository_validation".to_string(),
                scope: PolicyScope {
                    domain: "build".to_string(),
                    target: source_commit.to_string(),
                },
                mutates: false,
                requires_privilege: false,
                requires_explicit_apply: false,
                required_authority: PolicyAuthority::RuntimePrincipal,
                preconditions: vec!["source_commit_observed".to_string()],
                rollback_intent: "discard isolated checkout".to_string(),
                expected_proofs: vec![RequiredProof {
                    gate: "canonical_gate_report".to_string(),
                    subject: source_commit.to_string(),
                }],
            },
            decision: PolicyDecision::Allow {
                required_authority: PolicyAuthority::RuntimePrincipal,
                required_proofs: vec![RequiredProof {
                    gate: "canonical_gate_report".to_string(),
                    subject: source_commit.to_string(),
                }],
            },
        },
        execution_authorization: ExecutionAuthorization {
            schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
            actor: agent_id.to_string(),
            authority: PolicyAuthority::RuntimePrincipal,
            explicit_apply: false,
            satisfied_preconditions: vec!["source_commit_observed".to_string()],
        },
    };
    if !admit_agent_workload(&envelope).admitted() {
        return Err(vec![format!("build_admission_denied:{workload_id}")]);
    }
    let record = AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope,
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: None,
    };
    let keys = store
        .active_idempotency_keys()
        .map_err(|error| store_error(&error))?;
    if keys.iter().any(|key| key == &idempotency_key) {
        return store
            .read(&workload_id)
            .map_err(|error| store_error(&error))?
            .ok_or_else(|| vec![format!("build_admission_replay_missing:{workload_id}")]);
    }
    store
        .compare_and_swap(&workload_id, None, &record, committed_at_ms)
        .map_err(|error| store_error(&error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local_agent_workload_store::LocalAgentWorkloadStore;
    use std::sync::atomic::{AtomicU64, Ordering};

    static ADMISSION_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> (LocalAgentWorkloadStore, std::path::PathBuf) {
        let id = ADMISSION_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("cynic-build-admission-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create admission store dir");
        (LocalAgentWorkloadStore::new(&dir), dir)
    }

    const COMMIT: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4";

    #[test]
    fn admits_check_build_with_canonical_argv() {
        let (store, _dir) = temp_store();
        let durable = admit_build_check(
            COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit check build");

        assert_eq!(durable.record.state, AgentWorkloadState::Dispatched);
        assert_eq!(durable.record.envelope.action.argv[0], "cargo");
        assert_eq!(durable.record.envelope.action.argv[1], "check");
        assert!(!durable.record.envelope.action.mutates);
    }

    #[test]
    fn duplicate_sha_replays_without_second_record() {
        let (store, _dir) = temp_store();
        let first = admit_build_check(
            COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("first admit");
        let second = admit_build_check(
            COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            200,
            &store,
        )
        .expect("replay admit");

        assert_eq!(first, second);
        let records = store.durable_records().expect("list records");
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn bad_sha_fails_closed_before_store() {
        let (store, _dir) = temp_store();
        let errors = admit_build_check(
            "short",
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect_err("bad sha rejected");

        assert!(
            errors
                .iter()
                .any(|error| error == "build_slice_source_commit_invalid")
        );
        let records = store.durable_records().expect("list records");
        assert!(records.is_empty());
    }
}
