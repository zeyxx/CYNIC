use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use cynic_protocol::{
    AgentCompletionEnvelope, AgentCompletionStatus, LeaseGuard, admit_completion, digest_json,
};

use super::{
    admission::admit_agent_workload,
    shared::violation,
    types::{
        AGENT_WORKLOAD_RECORD_SCHEMA, AgentWorkloadRecord, AgentWorkloadState,
        AgentWorkloadViolation,
    },
};

pub const SETTLED_PARCEL_RECEIPT_SCHEMA: &str = "cynic.settled_parcel_receipt.v1";

/// A content-addressed receipt proving atomic settlement of an Agent Parcel (RFC-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettledParcelReceipt {
    pub schema: &'static str,
    pub workload_id: String,
    pub settled_at_ms: u64,
    pub status: AgentCompletionStatus,
    pub final_record_digest: String,
    pub evidence_count: usize,
}

impl SettledParcelReceipt {
    /// Construct a receipt from an already-settled record.
    pub fn for_record(
        record: &AgentWorkloadRecord,
        settled_at_ms: u64,
    ) -> Result<Self, AgentWorkloadViolation> {
        let Some(completion) = &record.completion else {
            return Err(violation(
                "completion_missing",
                "settled record must contain a completion envelope",
            ));
        };
        let final_record_digest = digest_json(record).map_err(|e| {
            violation(
                "record_digest_failed",
                format!("failed to compute record digest: {e}"),
            )
        })?;
        Ok(Self {
            schema: SETTLED_PARCEL_RECEIPT_SCHEMA,
            workload_id: record.envelope.workload_id.clone(),
            settled_at_ms,
            status: completion.status,
            final_record_digest,
            evidence_count: completion.evidence_refs.len(),
        })
    }
}

/// Atomically settles an admitted and claimed Agent Parcel (RFC-003).
///
/// Pure Zero-I/O settlement enforcing:
/// 1. Canonical action plan immutability (`plan_digest`).
/// 2. Fenced claim ownership and monotonic fencing token match.
/// 3. Valid lease window (`now_ms <= claim.expires_at_ms`).
/// 4. Delegation budget compliance (wall time, actions).
/// 5. Expected evidence completeness for successful completions.
pub fn settle_parcel(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    completion: &AgentCompletionEnvelope,
    now_ms: u64,
) -> Result<AgentWorkloadRecord, AgentWorkloadViolation> {
    let (settled, _receipt) = settle_parcel_with_receipt(record, guard, completion, now_ms)?;
    Ok(settled)
}

/// Atomically settles an Agent Parcel and returns both the updated record and its cryptographic receipt.
pub fn settle_parcel_with_receipt(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    completion: &AgentCompletionEnvelope,
    now_ms: u64,
) -> Result<(AgentWorkloadRecord, SettledParcelReceipt), AgentWorkloadViolation> {
    // 1. Verify action plan immutability against in-flight tampering
    let computed_plan_digest = digest_json(&record.envelope.action).map_err(|e| {
        violation(
            "action_plan_not_serializable",
            format!("bounded action could not be canonicalized: {e}"),
        )
    })?;
    if computed_plan_digest != record.envelope.plan_digest {
        return Err(violation(
            "plan_digest_tampered",
            "bounded action digest does not match the immutable plan_digest admitted in envelope",
        ));
    }

    require_valid_record(record)?;
    let claim = require_live_claim(record, guard, now_ms)?;

    // 3. Validate completion against delegation requirements
    validate_parcel_completion(record, claim, completion)?;

    // 4. Perform atomic state transition
    let mut next = record.clone();
    next.state = match completion.status {
        AgentCompletionStatus::Completed => AgentWorkloadState::Completed,
        AgentCompletionStatus::Blocked => AgentWorkloadState::Blocked,
        AgentCompletionStatus::Failed => AgentWorkloadState::Failed,
        AgentCompletionStatus::Degraded => AgentWorkloadState::Degraded,
        AgentCompletionStatus::RecoveredUnattributed => AgentWorkloadState::RecoveredUnattributed,
    };
    next.claim = None;
    next.completion = Some(completion.clone());

    // 5. Generate immutable settlement receipt
    let receipt = SettledParcelReceipt::for_record(&next, now_ms)?;

    Ok((next, receipt))
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

fn require_live_claim<'a>(
    record: &'a AgentWorkloadRecord,
    guard: &LeaseGuard,
    now_ms: u64,
) -> Result<&'a super::types::AgentWorkloadClaim, AgentWorkloadViolation> {
    if record.state != AgentWorkloadState::Claimed {
        return Err(violation(
            "workload_not_claimed",
            "workload must have a live claim to settle",
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
    if now_ms > claim.expires_at_ms {
        return Err(violation("claim_expired", "claim lease has expired"));
    }
    if claim.cancellation_generation != record.cancellation_generation {
        return Err(violation(
            "workload_cancelled",
            "claim predates the current cancellation generation",
        ));
    }
    Ok(claim)
}

fn validate_parcel_completion(
    record: &AgentWorkloadRecord,
    _claim: &super::types::AgentWorkloadClaim,
    completion: &AgentCompletionEnvelope,
) -> Result<(), AgentWorkloadViolation> {
    let completion_admission = admit_completion(&record.envelope.delegation, completion);
    if !completion_admission.admitted() {
        return Err(violation(
            "completion_not_admitted",
            "completion violates delegation, budget, lineage, or evidence requirements",
        ));
    }
    if completion.consumed_budget.actions < record.consumed_actions {
        return Err(violation(
            "completion_action_budget_underreported",
            "completion consumed budget cannot be lower than durable action accounting",
        ));
    }
    if completion.consumed_budget.actions > record.envelope.delegation.budget.actions {
        return Err(violation(
            "action_budget_exceeded",
            "completion consumed actions exceed delegated budget",
        ));
    }
    if completion.consumed_budget.wall_time_ms > record.envelope.delegation.budget.wall_time_ms {
        return Err(violation(
            "wall_time_budget_exceeded",
            "completion consumed wall time exceeds delegated budget",
        ));
    }
    if completion.status == AgentCompletionStatus::Completed {
        let returned_proofs = completion
            .evidence_refs
            .iter()
            .map(|evidence| evidence.kind.as_str())
            .collect::<BTreeSet<_>>();
        for expected in &record.envelope.delegation.expected_proofs {
            if !returned_proofs.contains(expected.as_str()) {
                return Err(violation(
                    "expected_proof_missing",
                    format!("settlement missing expected proof '{expected}'"),
                ));
            }
        }
    }

    Ok(())
}
