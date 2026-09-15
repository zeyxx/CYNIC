use cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort;
use cynic_workload::{
    AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA,
    AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA,
    AGENT_WORKSPACE_REQUEST_SCHEMA,
    AgentWorkspaceClaimIntent,
    AgentWorkspaceClaimOutcome,
    AgentWorkspaceDecision,
    AgentWorkspaceLifecycleRecord,
    AgentWorkspaceRequest,
    admit_agent_workspace_claim,
    reconcile_agent_workspace,
};
use cynic_protocol::{
    AgentEvidenceRef,
    build_evidence_envelope,
    digest_json,
};

use crate::ports::{
    agent_workspace_port::{
        AgentWorkspaceClaimStorePort, AgentWorkspaceLayoutPort, AgentWorkspaceLifecycleStorePort,
        AgentWorkspaceObservationPort, AgentWorkspaceProvisionPort,
    },
    source_change_transaction_port::AgentWorkspaceApplyPort,
};

use super::types::{
    AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA, AgentSourceChangeTransactionRequest,
    AgentSourceChangeTransactionResult, AgentSourceChangeTransactionStatus,
};

pub(crate) async fn execute_agent_source_change_transaction<C, L, O, P, A, E, S>(
    request: &AgentSourceChangeTransactionRequest,
    claim_store: &C,
    layout_port: &L,
    observation_port: &O,
    provision_port: &P,
    apply_port: &A,
    evidence_store: &E,
    lifecycle_store: &S,
    now_ms: u64,
    max_claim_ttl_ms: u64,
    max_retention_ms: u64,
) -> AgentSourceChangeTransactionResult
where
    C: AgentWorkspaceClaimStorePort,
    L: AgentWorkspaceLayoutPort,
    O: AgentWorkspaceObservationPort,
    P: AgentWorkspaceProvisionPort,
    A: AgentWorkspaceApplyPort,
    E: ContentAddressedEvidenceStorePort,
    S: AgentWorkspaceLifecycleStorePort,
{
    let workload_id = request.workload.workload_id.clone();
    if request.retention_ms > max_retention_ms {
        return AgentSourceChangeTransactionResult::blocked(
            workload_id,
            vec!["workspace_retention_exceeds_node_policy".to_string()],
        );
    }
    if request.protected_roots.len() > 64 {
        return AgentSourceChangeTransactionResult::blocked(
            workload_id,
            vec!["workspace_protected_root_budget_exceeded".to_string()],
        );
    }
    let change_set_digest = match digest_json(&request.changes) {
        Ok(digest) => digest,
        Err(failure) => {
            return AgentSourceChangeTransactionResult::blocked(
                workload_id,
                vec![format!("change_set_digest_failed:{failure}")],
            );
        }
    };
    let claim_intent = AgentWorkspaceClaimIntent {
        schema: AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA.to_string(),
        workload_id: workload_id.clone(),
        repository_id: request.repository_id.clone(),
        workspace_id: request.workspace_id.clone(),
        node_id: request.workload.delegation.node_id.clone(),
        agent_id: request.workload.delegation.agent_id.clone(),
        zones: request.zones.clone(),
        requested_ttl_ms: request.requested_claim_ttl_ms,
    };
    let claim_admission =
        admit_agent_workspace_claim(&request.workload, &claim_intent, max_claim_ttl_ms);
    if !claim_admission.admitted() {
        return AgentSourceChangeTransactionResult::blocked(
            workload_id,
            vec!["workspace_claim_not_admitted".to_string()],
        );
    }
    let claim = match claim_store
        .reserve(&claim_intent, &claim_admission, now_ms)
        .await
    {
        Ok(AgentWorkspaceClaimOutcome::Granted { claim }) => claim,
        Ok(AgentWorkspaceClaimOutcome::Conflict { conflicts }) => {
            return AgentSourceChangeTransactionResult::blocked(
                workload_id,
                conflicts
                    .into_iter()
                    .map(|claim| format!("workspace_zone_conflict:{}", claim.workload_id))
                    .collect(),
            );
        }
        Ok(AgentWorkspaceClaimOutcome::Rejected { violations }) => {
            return AgentSourceChangeTransactionResult::blocked(
                workload_id,
                violations.into_iter().map(|item| item.code).collect(),
            );
        }
        Err(failure) => {
            return AgentSourceChangeTransactionResult {
                schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
                workload_id,
                status: AgentSourceChangeTransactionStatus::Unknown,
                reconciliation: None,
                apply_report: None,
                proof_ref: None,
                blockers: Vec::new(),
                unknowns: vec![format!(
                    "workspace_claim_store:{}:{}",
                    failure.code, failure.message
                )],
            };
        }
    };

    let workspace_request = AgentWorkspaceRequest {
        schema: AGENT_WORKSPACE_REQUEST_SCHEMA.to_string(),
        workload_id: workload_id.clone(),
        repository_id: request.repository_id.clone(),
        workspace_id: request.workspace_id.clone(),
        node_id: request.workload.delegation.node_id.clone(),
        agent_id: request.workload.delegation.agent_id.clone(),
        base_commit: request.base_commit.clone(),
        change_set_digest,
        changes: request.changes.clone(),
        zones: request.zones.clone(),
        guard: claim.guard.clone(),
    };
    let layout = match layout_port.resolve(
        &request.repository_id,
        &request.workspace_id,
        &workspace_request.node_id,
    ) {
        Ok(layout) => layout,
        Err(failure) => {
            let _ = claim_store.release(&claim.guard, now_ms).await;
            return AgentSourceChangeTransactionResult::blocked(
                workload_id,
                vec![format!("workspace_layout:{}", failure.code)],
            );
        }
    };
    let repository_claims = match claim_store
        .repository_claims(&request.repository_id, now_ms)
        .await
    {
        Ok(claims) => claims,
        Err(failure) => {
            return AgentSourceChangeTransactionResult {
                schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
                workload_id,
                status: AgentSourceChangeTransactionStatus::Unknown,
                reconciliation: None,
                apply_report: None,
                proof_ref: None,
                blockers: Vec::new(),
                unknowns: vec![format!("repository_claim_observation:{}", failure.code)],
            };
        }
    };
    let mut observation =
        match observation_port.observe(&workspace_request, repository_claims.clone()) {
            Ok(observation) => observation,
            Err(failure) => {
                let _ = claim_store.release(&claim.guard, now_ms).await;
                return AgentSourceChangeTransactionResult::blocked(
                    workload_id,
                    vec![format!("workspace_observation:{}", failure.code)],
                );
            }
        };
    if !observation.exists {
        if let Err(failure) = provision_port.ensure(&layout, &workspace_request) {
            let _ = claim_store.release(&claim.guard, now_ms).await;
            return AgentSourceChangeTransactionResult::blocked(
                workload_id,
                vec![format!("workspace_provision:{}", failure.code)],
            );
        }
        observation = match observation_port.observe(&workspace_request, repository_claims) {
            Ok(observation) => observation,
            Err(failure) => {
                let _ = claim_store.release(&claim.guard, now_ms).await;
                return AgentSourceChangeTransactionResult::blocked(
                    workload_id,
                    vec![format!("workspace_provision_read_back:{}", failure.code)],
                );
            }
        };
    }
    let reconciliation =
        reconcile_agent_workspace(&request.workload, &workspace_request, &observation, now_ms);
    if !reconciliation.ready() {
        let _ = claim_store.release(&claim.guard, now_ms).await;
        let blockers = match &reconciliation.decision {
            AgentWorkspaceDecision::Deny { violations } => {
                violations.iter().map(|item| item.code.clone()).collect()
            }
            AgentWorkspaceDecision::Unknown { missing_evidence } => missing_evidence.clone(),
            AgentWorkspaceDecision::Ready { .. } => Vec::new(),
        };
        return AgentSourceChangeTransactionResult {
            schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
            workload_id,
            status: AgentSourceChangeTransactionStatus::Blocked,
            reconciliation: Some(reconciliation),
            apply_report: None,
            proof_ref: None,
            blockers,
            unknowns: Vec::new(),
        };
    }

    let receipt = match apply_port.apply(&layout, &workspace_request, &reconciliation) {
        Ok(receipt) => receipt,
        Err(failure) => {
            let _ = claim_store.release(&claim.guard, now_ms).await;
            return AgentSourceChangeTransactionResult {
                schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
                workload_id,
                status: AgentSourceChangeTransactionStatus::Blocked,
                reconciliation: Some(reconciliation),
                apply_report: None,
                proof_ref: None,
                blockers: vec![format!("workspace_apply:{}", failure.code)],
                unknowns: Vec::new(),
            };
        }
    };
    let apply_report = A::report(&receipt).clone();
    let provenance = observation
        .evidence_refs
        .iter()
        .map(|evidence| evidence.digest.clone())
        .collect::<Vec<_>>();
    let evidence = match build_evidence_envelope(
        &format!("source-change:{workload_id}"),
        &workspace_request.node_id,
        u128::from(now_ms),
        provenance,
        &apply_report,
    ) {
        Ok(evidence) => evidence,
        Err(failure) => {
            let rollback = apply_port.rollback(receipt);
            let _ = claim_store.release(&claim.guard, now_ms).await;
            return rolled_back(
                workload_id,
                reconciliation,
                apply_report,
                format!("apply_evidence_build:{failure}"),
                rollback.err().map(|error| error.code),
            );
        }
    };
    if let Err(failure) = evidence_store.persist(&evidence) {
        let rollback = apply_port.rollback(receipt);
        let _ = claim_store.release(&claim.guard, now_ms).await;
        return rolled_back(
            workload_id,
            reconciliation,
            apply_report,
            format!("apply_evidence_persist:{failure}"),
            rollback.err().map(|error| error.code),
        );
    }
    if let Err(failure) = claim_store
        .finalize(&claim.guard, &evidence.digest, now_ms)
        .await
    {
        let rollback = apply_port.rollback(receipt);
        return rolled_back(
            workload_id,
            reconciliation,
            apply_report,
            format!("claim_finalize:{}", failure.code),
            rollback.err().map(|error| error.code),
        );
    }
    if let Err(failure) = apply_port.commit(receipt) {
        return AgentSourceChangeTransactionResult {
            schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
            workload_id,
            status: AgentSourceChangeTransactionStatus::Unknown,
            reconciliation: Some(reconciliation),
            apply_report: Some(apply_report),
            proof_ref: Some(evidence.digest),
            blockers: Vec::new(),
            unknowns: vec![format!("transaction_cleanup:{}", failure.code)],
        };
    }
    let retention_until_ms = match now_ms.checked_add(request.retention_ms) {
        Some(value) => value,
        None => {
            return AgentSourceChangeTransactionResult {
                schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
                workload_id,
                status: AgentSourceChangeTransactionStatus::Unknown,
                reconciliation: Some(reconciliation),
                apply_report: Some(apply_report),
                proof_ref: Some(evidence.digest),
                blockers: Vec::new(),
                unknowns: vec!["workspace_lifecycle_retention_overflow".to_string()],
            };
        }
    };
    let lifecycle = AgentWorkspaceLifecycleRecord {
        schema: AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA.to_string(),
        workload_id: workload_id.clone(),
        repository_id: request.repository_id.clone(),
        workspace_id: request.workspace_id.clone(),
        node_id: request.workload.delegation.node_id.clone(),
        completion_evidence: AgentEvidenceRef {
            kind: "agent_completion".to_string(),
            subject: workload_id.clone(),
            digest: evidence.digest.clone(),
            lineage: None,
        },
        completed_at_ms: now_ms,
        retention_until_ms,
        protected_roots: request.protected_roots.clone(),
    };
    if let Err(failure) = lifecycle_store.persist(&lifecycle) {
        return AgentSourceChangeTransactionResult {
            schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
            workload_id,
            status: AgentSourceChangeTransactionStatus::Unknown,
            reconciliation: Some(reconciliation),
            apply_report: Some(apply_report),
            proof_ref: Some(evidence.digest),
            blockers: Vec::new(),
            unknowns: vec![format!("workspace_lifecycle_persist:{}", failure.code)],
        };
    }
    AgentSourceChangeTransactionResult {
        schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
        workload_id,
        status: AgentSourceChangeTransactionStatus::Completed,
        reconciliation: Some(reconciliation),
        apply_report: Some(apply_report),
        proof_ref: Some(evidence.digest),
        blockers: Vec::new(),
        unknowns: Vec::new(),
    }
}

fn rolled_back(
    workload_id: String,
    reconciliation: cynic_workload::AgentWorkspaceReconciliation,
    apply_report: cynic_workload::AgentWorkspaceApplyReport,
    blocker: String,
    rollback_unknown: Option<String>,
) -> AgentSourceChangeTransactionResult {
    AgentSourceChangeTransactionResult {
        schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
        workload_id,
        status: if rollback_unknown.is_some() {
            AgentSourceChangeTransactionStatus::Unknown
        } else {
            AgentSourceChangeTransactionStatus::RolledBack
        },
        reconciliation: Some(reconciliation),
        apply_report: Some(apply_report),
        proof_ref: None,
        blockers: vec![blocker],
        unknowns: rollback_unknown.into_iter().collect(),
    }
}
