use serde::{Deserialize, Serialize};

use crate::{
    AgentBudget, AgentWorkspaceLifecycleDecision, AgentWorkspaceLifecyclePlan, LeaseGuard,
};

pub const AGENT_SCHEDULER_STEP_OBSERVATION_SCHEMA: &str =
    "cynic.agent_scheduler_step_observation.v1";
pub const AGENT_SCHEDULER_STEP_PLAN_SCHEMA: &str = "cynic.agent_scheduler_step_plan.v1";
pub const AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA: &str = "cynic.agent_scheduler_journal_record.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerStepObservation {
    pub schema: String,
    pub node_id: String,
    pub scheduler_epoch: u64,
    pub step_id: String,
    pub aggregate_budget_limit: AgentBudget,
    pub aggregate_budget_reserved: AgentBudget,
    pub lifecycle: AgentWorkspaceLifecyclePlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerAction {
    pub idempotency_key: String,
    pub target: String,
    #[serde(default)]
    pub guard: Option<LeaseGuard>,
    pub mutates: bool,
    pub stop_condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentSchedulerStepDecision {
    Idle {
        reasons: Vec<String>,
    },
    RenewClaim {
        action: AgentSchedulerAction,
        requested_ttl_ms: u64,
    },
    DeliverCancellation {
        action: AgentSchedulerAction,
        cancellation_generation: u64,
    },
    AcquireRecoveryClaim {
        action: AgentSchedulerAction,
        reason: String,
    },
    AcquireCollectionClaim {
        action: AgentSchedulerAction,
    },
    ExecuteCollection {
        action: AgentSchedulerAction,
        completion_evidence_digest: String,
    },
    Unknown {
        missing_evidence: Vec<String>,
    },
    Blocked {
        violations: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerStepPlan {
    pub schema: String,
    pub node_id: String,
    pub scheduler_epoch: u64,
    pub step_id: String,
    pub lifecycle: AgentWorkspaceLifecyclePlan,
    pub decision: AgentSchedulerStepDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerJournalStatus {
    Planned,
    Applied,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerJournalRecord {
    pub schema: String,
    pub node_id: String,
    pub scheduler_epoch: u64,
    pub step_id: String,
    pub plan_digest: String,
    #[serde(default)]
    pub plan: Option<AgentSchedulerStepPlan>,
    pub status: AgentSchedulerJournalStatus,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    pub recorded_at_ms: u64,
}

pub fn validate_agent_scheduler_journal_record(
    record: &AgentSchedulerJournalRecord,
) -> Vec<String> {
    let mut violations = Vec::new();
    if record.schema != AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA {
        violations.push("invalid_scheduler_journal_record_schema".to_string());
    }
    if record.node_id.trim().is_empty()
        || record.step_id.trim().is_empty()
        || record.scheduler_epoch == 0
        || record.recorded_at_ms == 0
    {
        violations.push("scheduler_journal_identity_missing".to_string());
    }
    if !valid_sha256(&record.plan_digest) {
        violations.push("scheduler_journal_plan_digest_invalid".to_string());
    }
    match (&record.status, &record.plan) {
        (AgentSchedulerJournalStatus::Planned, Some(plan)) => {
            if plan.node_id != record.node_id
                || plan.scheduler_epoch != record.scheduler_epoch
                || plan.step_id != record.step_id
            {
                violations.push("scheduler_journal_plan_identity_mismatch".to_string());
            }
            match crate::digest_json(plan) {
                Ok(digest) if digest == record.plan_digest => {}
                _ => violations.push("scheduler_journal_plan_digest_mismatch".to_string()),
            }
        }
        (AgentSchedulerJournalStatus::Planned, None) => {
            violations.push("scheduler_journal_planned_without_plan".to_string());
        }
        (_, Some(_)) => violations.push("scheduler_journal_result_embeds_plan".to_string()),
        (_, None) => {}
    }
    if record.status == AgentSchedulerJournalStatus::Applied && record.evidence_refs.is_empty() {
        violations.push("scheduler_journal_applied_without_evidence".to_string());
    }
    if record.status == AgentSchedulerJournalStatus::Unknown && record.unknowns.is_empty() {
        violations.push("scheduler_journal_unknown_without_unknown".to_string());
    }
    violations
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

pub fn reconcile_agent_scheduler_step(
    observation: &AgentSchedulerStepObservation,
) -> AgentSchedulerStepPlan {
    let mut violations = Vec::new();
    if observation.schema != AGENT_SCHEDULER_STEP_OBSERVATION_SCHEMA {
        violations.push("invalid_scheduler_step_observation_schema".to_string());
    }
    if observation.node_id.trim().is_empty()
        || observation.step_id.trim().is_empty()
        || observation.scheduler_epoch == 0
    {
        violations.push("scheduler_step_identity_missing".to_string());
    }
    if !budget_fits(
        &observation.aggregate_budget_reserved,
        &observation.aggregate_budget_limit,
    ) {
        violations.push("aggregate_sibling_budget_exceeded".to_string());
    }
    if !violations.is_empty() {
        return plan(
            observation,
            AgentSchedulerStepDecision::Blocked { violations },
        );
    }

    let idempotency_key = format!(
        "{}:{}:{}",
        observation.node_id, observation.scheduler_epoch, observation.step_id
    );
    let decision = match &observation.lifecycle.decision {
        AgentWorkspaceLifecycleDecision::Keep { reasons } => AgentSchedulerStepDecision::Idle {
            reasons: reasons.clone(),
        },
        AgentWorkspaceLifecycleDecision::Renew {
            guard,
            requested_ttl_ms,
        } if *requested_ttl_ms > 0 => AgentSchedulerStepDecision::RenewClaim {
            action: action(
                &idempotency_key,
                &guard.target,
                Some(guard.clone()),
                "claim expiry read back beyond the requested renewal window",
            ),
            requested_ttl_ms: *requested_ttl_ms,
        },
        AgentWorkspaceLifecycleDecision::Renew { .. } => AgentSchedulerStepDecision::Blocked {
            violations: vec!["scheduler_renewal_ttl_zero".to_string()],
        },
        AgentWorkspaceLifecycleDecision::DeliverCancellation {
            guard,
            cancellation_generation,
        } => AgentSchedulerStepDecision::DeliverCancellation {
            action: action(
                &idempotency_key,
                &guard.target,
                Some(guard.clone()),
                "executor acknowledges the exact cancellation generation and stops",
            ),
            cancellation_generation: *cancellation_generation,
        },
        AgentWorkspaceLifecycleDecision::Recover { reason } => {
            AgentSchedulerStepDecision::AcquireRecoveryClaim {
                action: action(
                    &idempotency_key,
                    &format!(
                        "agent-workspace:{}:{}",
                        observation.lifecycle.repository_id, observation.lifecycle.workspace_id
                    ),
                    None,
                    "a new fenced recovery claim is durably observed",
                ),
                reason: reason.clone(),
            }
        }
        AgentWorkspaceLifecycleDecision::AcquireCollectionClaim { target } => {
            AgentSchedulerStepDecision::AcquireCollectionClaim {
                action: action(
                    &idempotency_key,
                    target,
                    None,
                    "a fresh collection claim is durably observed",
                ),
            }
        }
        AgentWorkspaceLifecycleDecision::Collect {
            target,
            guard,
            completion_evidence_digest,
        } => AgentSchedulerStepDecision::ExecuteCollection {
            action: action(
                &idempotency_key,
                target,
                Some(guard.clone()),
                "workspace absence and post-collection capacity are read back",
            ),
            completion_evidence_digest: completion_evidence_digest.clone(),
        },
        AgentWorkspaceLifecycleDecision::Unknown { missing_evidence } => {
            AgentSchedulerStepDecision::Unknown {
                missing_evidence: missing_evidence.clone(),
            }
        }
        AgentWorkspaceLifecycleDecision::Blocked { violations } => {
            AgentSchedulerStepDecision::Blocked {
                violations: violations.clone(),
            }
        }
    };
    plan(observation, decision)
}

fn action(
    idempotency_key: &str,
    target: &str,
    guard: Option<LeaseGuard>,
    stop_condition: &str,
) -> AgentSchedulerAction {
    AgentSchedulerAction {
        idempotency_key: idempotency_key.to_string(),
        target: target.to_string(),
        guard,
        mutates: true,
        stop_condition: stop_condition.to_string(),
    }
}

fn budget_fits(reserved: &AgentBudget, limit: &AgentBudget) -> bool {
    reserved.context_tokens <= limit.context_tokens
        && reserved.actions <= limit.actions
        && reserved.children <= limit.children
        && reserved.wall_time_ms <= limit.wall_time_ms
}

fn plan(
    observation: &AgentSchedulerStepObservation,
    decision: AgentSchedulerStepDecision,
) -> AgentSchedulerStepPlan {
    AgentSchedulerStepPlan {
        schema: AGENT_SCHEDULER_STEP_PLAN_SCHEMA.to_string(),
        node_id: observation.node_id.clone(),
        scheduler_epoch: observation.scheduler_epoch,
        step_id: observation.step_id.clone(),
        lifecycle: observation.lifecycle.clone(),
        decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(actions: u32) -> AgentBudget {
        AgentBudget {
            context_tokens: 100,
            actions,
            children: 1,
            wall_time_ms: 1000,
            cpu_ms: None,
        }
    }

    fn lifecycle(decision: AgentWorkspaceLifecycleDecision) -> AgentWorkspaceLifecyclePlan {
        AgentWorkspaceLifecyclePlan {
            schema: "cynic.agent_workspace_lifecycle_plan.v1".into(),
            workload_id: "workload-1".into(),
            repository_id: "cynic".into(),
            workspace_id: "workspace-1".into(),
            mutates: true,
            decision,
        }
    }

    fn observation(decision: AgentWorkspaceLifecycleDecision) -> AgentSchedulerStepObservation {
        AgentSchedulerStepObservation {
            schema: AGENT_SCHEDULER_STEP_OBSERVATION_SCHEMA.into(),
            node_id: "node-1".into(),
            scheduler_epoch: 7,
            step_id: "step-1".into(),
            aggregate_budget_limit: budget(2),
            aggregate_budget_reserved: budget(1),
            lifecycle: lifecycle(decision),
        }
    }

    #[test]
    fn aggregate_sibling_budget_blocks_before_action() {
        let mut observed = observation(AgentWorkspaceLifecycleDecision::Recover {
            reason: "expired".into(),
        });
        observed.aggregate_budget_reserved.actions = 3;
        assert!(matches!(
            reconcile_agent_scheduler_step(&observed).decision,
            AgentSchedulerStepDecision::Blocked { .. }
        ));
    }

    #[test]
    fn recovery_requires_a_new_fenced_claim() {
        let observed = observation(AgentWorkspaceLifecycleDecision::Recover {
            reason: "expired".into(),
        });
        assert!(matches!(
            reconcile_agent_scheduler_step(&observed).decision,
            AgentSchedulerStepDecision::AcquireRecoveryClaim { .. }
        ));
    }

    #[test]
    fn scheduler_step_idempotency_is_epoch_bound() {
        let observed = observation(AgentWorkspaceLifecycleDecision::AcquireCollectionClaim {
            target: "agent-workspace:cynic:workspace-1".into(),
        });
        let first = reconcile_agent_scheduler_step(&observed);
        let second = reconcile_agent_scheduler_step(&observed);
        assert_eq!(first, second);
    }

    #[test]
    fn applied_journal_record_requires_evidence() {
        let record = AgentSchedulerJournalRecord {
            schema: AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA.into(),
            node_id: "node-1".into(),
            scheduler_epoch: 1,
            step_id: "step-1".into(),
            plan: None,
            plan_digest: format!("sha256:{}", "a".repeat(64)),
            status: AgentSchedulerJournalStatus::Applied,
            evidence_refs: vec![],
            unknowns: vec![],
            recorded_at_ms: 1,
        };
        assert!(
            validate_agent_scheduler_journal_record(&record)
                .contains(&"scheduler_journal_applied_without_evidence".to_string())
        );
    }
}
