use std::collections::BTreeSet;

use crate::{AgentCompletionEnvelope, AgentCompletionStatus, LeaseGuard, admit_completion};

use super::{
    admission::admit_agent_workload,
    shared::{admission, violation},
    types::{
        AGENT_WORKLOAD_RECORD_SCHEMA, AgentWorkloadAdmission, AgentWorkloadClaim,
        AgentWorkloadRecord, AgentWorkloadState, AgentWorkloadViolation,
    },
};

pub fn claim_agent_workload(
    record: &AgentWorkloadRecord,
    guard: LeaseGuard,
    expires_at_ms: u64,
    now_ms: u64,
) -> Result<AgentWorkloadRecord, AgentWorkloadViolation> {
    require_valid_record(record)?;
    if record.state.terminal() || record.state == AgentWorkloadState::CancelRequested {
        return Err(violation(
            "workload_not_claimable",
            "terminal or cancelled work cannot be claimed",
        ));
    }
    if record.state == AgentWorkloadState::Claimed
        && record
            .claim
            .as_ref()
            .is_some_and(|claim| claim.expires_at_ms > now_ms)
    {
        return Err(violation(
            "active_claim_exists",
            "the current claim has not expired",
        ));
    }
    if guard.agent_id != record.envelope.delegation.agent_id
        || guard.target != record.envelope.workload_id
        || guard.lease_id.trim().is_empty()
    {
        return Err(violation(
            "claim_identity_mismatch",
            "claim identity, target, and lease must match the admitted workload",
        ));
    }
    if expires_at_ms <= now_ms {
        return Err(violation(
            "claim_already_expired",
            "claim expiry must be in the future",
        ));
    }
    if guard.fencing_token <= record.last_fencing_token {
        return Err(violation(
            "stale_fencing_token",
            "a new claim must advance the fencing token",
        ));
    }

    let mut next = record.clone();
    next.state = AgentWorkloadState::Claimed;
    next.last_fencing_token = guard.fencing_token;
    next.claim = Some(AgentWorkloadClaim {
        guard,
        expires_at_ms,
        cancellation_generation: record.cancellation_generation,
    });
    Ok(next)
}

pub fn admit_workload_action(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    now_ms: u64,
) -> AgentWorkloadAdmission {
    let mut violations = Vec::new();
    if let Err(error) = require_live_claim(record, guard, now_ms) {
        violations.push(error);
    }
    if record.consumed_actions >= record.envelope.delegation.budget.actions {
        violations.push(violation(
            "action_budget_exhausted",
            "the delegated action budget is exhausted",
        ));
    }
    admission(record.envelope.workload_id.clone(), violations, Vec::new())
}

pub fn consume_agent_workload_action(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    now_ms: u64,
) -> Result<AgentWorkloadRecord, AgentWorkloadViolation> {
    let admission = admit_workload_action(record, guard, now_ms);
    if !admission.admitted() {
        return Err(violation(
            "action_not_admitted",
            "the workload action violates its claim, cancellation, or budget",
        ));
    }
    let mut next = record.clone();
    next.consumed_actions = next
        .consumed_actions
        .checked_add(1)
        .ok_or_else(|| violation("action_budget_overflow", "action budget counter overflow"))?;
    Ok(next)
}

pub fn request_agent_workload_cancellation(
    record: &AgentWorkloadRecord,
) -> Result<AgentWorkloadRecord, AgentWorkloadViolation> {
    require_valid_record(record)?;
    if record.state.terminal() {
        return Err(violation(
            "terminal_workload",
            "terminal work cannot be cancelled",
        ));
    }
    let mut next = record.clone();
    next.cancellation_generation = next
        .cancellation_generation
        .checked_add(1)
        .ok_or_else(|| violation("cancellation_overflow", "cancellation generation overflow"))?;
    next.state = AgentWorkloadState::CancelRequested;
    Ok(next)
}

pub fn complete_agent_workload(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    completion: &AgentCompletionEnvelope,
    now_ms: u64,
) -> Result<AgentWorkloadRecord, AgentWorkloadViolation> {
    require_live_claim(record, guard, now_ms)?;
    let completion_admission = admit_completion(&record.envelope.delegation, completion);
    if !completion_admission.admitted() {
        return Err(violation(
            "completion_not_admitted",
            "completion violates delegation, budget, lineage, or evidence requirements",
        ));
    }
    if completion.status == AgentCompletionStatus::Completed {
        let returned_proofs = completion
            .evidence_refs
            .iter()
            .map(|evidence| evidence.kind.as_str())
            .collect::<BTreeSet<_>>();
        if let Some(missing) = record
            .envelope
            .delegation
            .expected_proofs
            .iter()
            .find(|proof| !returned_proofs.contains(proof.as_str()))
        {
            return Err(violation(
                "expected_proof_missing",
                format!("completion is missing expected proof kind '{missing}'"),
            ));
        }
    }

    let mut next = record.clone();
    next.state = match completion.status {
        AgentCompletionStatus::Completed => AgentWorkloadState::Completed,
        AgentCompletionStatus::Blocked => AgentWorkloadState::Blocked,
        AgentCompletionStatus::Failed => AgentWorkloadState::Failed,
    };
    next.claim = None;
    Ok(next)
}

fn require_valid_record(record: &AgentWorkloadRecord) -> Result<(), AgentWorkloadViolation> {
    if record.schema != AGENT_WORKLOAD_RECORD_SCHEMA
        || !admit_agent_workload(&record.envelope).admitted()
    {
        return Err(violation(
            "workload_not_admitted",
            "workload record does not contain an admitted constitutional envelope",
        ));
    }
    Ok(())
}

fn require_live_claim(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    now_ms: u64,
) -> Result<(), AgentWorkloadViolation> {
    require_valid_record(record)?;
    if record.state != AgentWorkloadState::Claimed {
        return Err(violation(
            "workload_not_claimed",
            "workload must have a live claim",
        ));
    }
    let claim = record
        .claim
        .as_ref()
        .ok_or_else(|| violation("claim_missing", "claimed workload has no durable claim"))?;
    if &claim.guard != guard || guard.fencing_token != record.last_fencing_token {
        return Err(violation(
            "claim_guard_rejected",
            "claim guard or fencing token is stale",
        ));
    }
    if claim.expires_at_ms <= now_ms {
        return Err(violation("claim_expired", "claim lease has expired"));
    }
    if claim.cancellation_generation != record.cancellation_generation {
        return Err(violation(
            "workload_cancelled",
            "claim predates the current cancellation generation",
        ));
    }
    Ok(())
}
