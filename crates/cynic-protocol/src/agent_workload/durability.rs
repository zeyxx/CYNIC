use serde::{Deserialize, Serialize};

use crate::{AgentBudget, AgentCompletionEnvelope, LeaseGuard, digest_json};

use super::{
    AGENT_WORKLOAD_RECORD_SCHEMA, AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
    admit_agent_workload,
};

pub const DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA: &str = "cynic.durable_agent_workload_record.v1";
pub const AGENT_WORKLOAD_TRANSITION_REQUEST_SCHEMA: &str =
    "cynic.agent_workload_transition_request.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableAgentWorkloadRecord {
    pub schema: String,
    pub revision: u64,
    #[serde(default)]
    pub previous_record_digest: Option<String>,
    pub record_digest: String,
    pub record: AgentWorkloadRecord,
    pub committed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadTransitionRequest {
    pub schema: String,
    pub expected_revision: Option<u64>,
    pub transition: AgentWorkloadTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentWorkloadTransition {
    Dispatch {
        envelope: AgentWorkloadEnvelope,
    },
    Claim {
        workload_id: String,
        guard: LeaseGuard,
        expires_at_ms: u64,
    },
    RenewClaim {
        workload_id: String,
        guard: LeaseGuard,
        expires_at_ms: u64,
    },
    ConsumeAction {
        workload_id: String,
        guard: LeaseGuard,
    },
    RequestCancellation {
        workload_id: String,
    },
    Complete {
        workload_id: String,
        guard: LeaseGuard,
        completion: AgentCompletionEnvelope,
    },
}

impl AgentWorkloadTransition {
    pub fn workload_id(&self) -> &str {
        match self {
            Self::Dispatch { envelope } => &envelope.workload_id,
            Self::Claim { workload_id, .. }
            | Self::RenewClaim { workload_id, .. }
            | Self::ConsumeAction { workload_id, .. }
            | Self::RequestCancellation { workload_id }
            | Self::Complete { workload_id, .. } => workload_id,
        }
    }
}

pub fn validate_durable_agent_workload_record(durable: &DurableAgentWorkloadRecord) -> Vec<String> {
    let mut violations = Vec::new();
    if durable.schema != DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA {
        violations.push("invalid_durable_agent_workload_record_schema".to_string());
    }
    if durable.revision == 0 || durable.committed_at_ms == 0 {
        violations.push("durable_agent_workload_identity_missing".to_string());
    }
    match digest_json(&durable.record) {
        Ok(digest) if digest == durable.record_digest => {}
        _ => violations.push("durable_agent_workload_record_digest_mismatch".to_string()),
    }
    violations.extend(validate_agent_workload_record(&durable.record));
    if durable.revision == 1 && durable.previous_record_digest.is_some() {
        violations.push("initial_workload_record_has_predecessor".to_string());
    }
    if durable.revision > 1 && durable.previous_record_digest.is_none() {
        violations.push("workload_record_predecessor_missing".to_string());
    }
    violations
}

pub fn validate_agent_workload_record(record: &AgentWorkloadRecord) -> Vec<String> {
    let mut violations = Vec::new();
    if record.schema != AGENT_WORKLOAD_RECORD_SCHEMA
        || !admit_agent_workload(&record.envelope).admitted()
    {
        violations.push("invalid_agent_workload_record".to_string());
    }
    if record.consumed_actions > record.envelope.delegation.budget.actions {
        violations.push("agent_workload_consumed_budget_exceeded".to_string());
    }
    match (record.state, record.claim.as_ref()) {
        (AgentWorkloadState::Dispatched, Some(_))
        | (AgentWorkloadState::Completed, Some(_))
        | (AgentWorkloadState::Blocked, Some(_))
        | (AgentWorkloadState::Failed, Some(_)) => {
            violations.push("agent_workload_claim_state_mismatch".to_string());
        }
        (AgentWorkloadState::Claimed, None) => {
            violations.push("claimed_agent_workload_claim_missing".to_string());
        }
        _ => {}
    }
    if let Some(claim) = &record.claim {
        if claim.guard.target != record.envelope.workload_id
            || claim.guard.agent_id != record.envelope.delegation.agent_id
            || claim.guard.fencing_token != record.last_fencing_token
            || claim.cancellation_generation > record.cancellation_generation
        {
            violations.push("agent_workload_claim_guard_mismatch".to_string());
        }
    }
    violations
}

pub fn admit_aggregate_sibling_budget(
    envelope: &AgentWorkloadEnvelope,
    active_siblings: &[AgentWorkloadRecord],
) -> Result<(), String> {
    let Some(parent) = envelope.parent_delegation.as_ref() else {
        return Ok(());
    };
    let mut reserved = AgentBudget {
        context_tokens: 0,
        actions: 0,
        children: 0,
        wall_time_ms: 0,
    };
    let siblings = active_siblings.iter().filter(|record| {
        !record.state.terminal()
            && record
                .envelope
                .parent_delegation
                .as_ref()
                .is_some_and(|candidate| candidate.delegation_id == parent.delegation_id)
    });
    let mut sibling_count = 0_u16;
    for sibling in siblings {
        sibling_count = sibling_count
            .checked_add(1)
            .ok_or_else(|| "aggregate_sibling_count_overflow".to_string())?;
        add_budget(&mut reserved, &sibling.envelope.delegation.budget)?;
    }
    let direct_children = sibling_count
        .checked_add(1)
        .ok_or_else(|| "aggregate_sibling_count_overflow".to_string())?;
    add_budget(&mut reserved, &envelope.delegation.budget)?;
    if direct_children > parent.budget.children
        || reserved.context_tokens > parent.budget.context_tokens
        || reserved.actions > parent.budget.actions
        || reserved.children > parent.budget.children
        || reserved.wall_time_ms > parent.budget.wall_time_ms
    {
        return Err("aggregate_sibling_budget_exceeded".to_string());
    }
    Ok(())
}

fn add_budget(total: &mut AgentBudget, budget: &AgentBudget) -> Result<(), String> {
    total.context_tokens = total
        .context_tokens
        .checked_add(budget.context_tokens)
        .ok_or_else(|| "aggregate_sibling_budget_overflow".to_string())?;
    total.actions = total
        .actions
        .checked_add(budget.actions)
        .ok_or_else(|| "aggregate_sibling_budget_overflow".to_string())?;
    total.children = total
        .children
        .checked_add(budget.children)
        .ok_or_else(|| "aggregate_sibling_budget_overflow".to_string())?;
    total.wall_time_ms = total
        .wall_time_ms
        .checked_add(budget.wall_time_ms)
        .ok_or_else(|| "aggregate_sibling_budget_overflow".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_workload::tests::record;

    #[test]
    fn durable_record_binds_payload_and_predecessor() {
        let record = record();
        let mut durable = DurableAgentWorkloadRecord {
            schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.into(),
            revision: 1,
            previous_record_digest: None,
            record_digest: digest_json(&record).expect("digest"),
            record,
            committed_at_ms: 1,
        };
        assert!(validate_durable_agent_workload_record(&durable).is_empty());
        durable.revision = 2;
        assert_eq!(
            validate_durable_agent_workload_record(&durable),
            vec!["workload_record_predecessor_missing"]
        );
    }

    #[test]
    fn aggregate_sibling_budget_counts_active_children() {
        let mut candidate = crate::agent_workload::tests::test_envelope();
        let mut parent = candidate.delegation.clone();
        parent.delegation_id = "parent-delegation".into();
        parent.agent_id = "parent-agent".into();
        parent.budget.children = 1;
        candidate.parent_delegation = Some(parent.clone());
        candidate.delegation.parent_agent_id = Some(parent.agent_id.clone());
        candidate.delegation.depth = 1;
        candidate.delegation.budget.context_tokens = 500;
        candidate.delegation.budget.actions = 1;
        candidate.delegation.budget.children = 0;
        candidate.delegation.budget.wall_time_ms = 400_000;

        let mut sibling = record();
        sibling.envelope.parent_delegation = Some(parent);
        sibling.envelope.delegation.parent_agent_id = Some("parent-agent".into());
        sibling.envelope.delegation.depth = 1;
        sibling.envelope.delegation.budget.context_tokens = 500;
        sibling.envelope.delegation.budget.actions = 1;
        sibling.envelope.delegation.budget.children = 0;
        sibling.envelope.delegation.budget.wall_time_ms = 400_000;

        assert_eq!(
            admit_aggregate_sibling_budget(&candidate, &[sibling]),
            Err("aggregate_sibling_budget_exceeded".into())
        );
    }
}
