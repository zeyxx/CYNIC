use serde::{Deserialize, Serialize};

use crate::{
    AgentWorkloadClaim, AgentWorkloadState, AgentWorkspaceClaimObservation,
    AgentWorkspaceObservation,
};
use cynic_protocol::{AgentEvidenceRef, LeaseGuard, validate_evidence_refs};

use super::workspace_checks::agent_workspace_target;

pub const AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA: &str =
    "cynic.agent_workspace_lifecycle_observation.v1";
pub const AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA: &str = "cynic.agent_workspace_lifecycle_plan.v1";
pub const AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA: &str =
    "cynic.agent_workspace_lifecycle_record.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceRetentionRoot {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceLifecycleRecord {
    pub schema: String,
    pub workload_id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub node_id: String,
    pub completion_evidence: AgentEvidenceRef,
    pub completed_at_ms: u64,
    pub retention_until_ms: u64,
    #[serde(default)]
    pub protected_roots: Vec<AgentWorkspaceRetentionRoot>,
}

pub fn validate_agent_workspace_lifecycle_record(
    record: &AgentWorkspaceLifecycleRecord,
) -> Vec<String> {
    let mut violations = Vec::new();
    if record.schema != AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA {
        violations.push("invalid_workspace_lifecycle_record_schema".to_string());
    }
    for (field, value) in [
        ("workload_id", record.workload_id.as_str()),
        ("repository_id", record.repository_id.as_str()),
        ("workspace_id", record.workspace_id.as_str()),
        ("node_id", record.node_id.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(format!("workspace_lifecycle_record_{field}_missing"));
        }
    }
    if record.completion_evidence.kind != "agent_completion"
        || record.completion_evidence.subject != record.workload_id
    {
        violations.push("workspace_lifecycle_completion_identity_mismatch".to_string());
    }
    violations.extend(
        validate_evidence_refs(std::slice::from_ref(&record.completion_evidence))
            .into_iter()
            .map(|item| format!("workspace_lifecycle_completion_{}", item.code)),
    );
    if record.completed_at_ms == 0 {
        violations.push("workspace_lifecycle_completed_at_missing".to_string());
    }
    if record.retention_until_ms < record.completed_at_ms {
        violations.push("workspace_lifecycle_retention_precedes_completion".to_string());
    }
    for root in &record.protected_roots {
        if root.kind.trim().is_empty() || root.id.trim().is_empty() {
            violations.push("workspace_lifecycle_protected_root_invalid".to_string());
        }
    }
    violations.sort();
    violations.dedup();
    violations
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceLifecycleObservation {
    pub schema: String,
    pub workload_id: String,
    pub workload_state: AgentWorkloadState,
    #[serde(default)]
    pub workload_claim: Option<AgentWorkloadClaim>,
    pub workspace: AgentWorkspaceObservation,
    #[serde(default)]
    pub completion_evidence: Option<AgentEvidenceRef>,
    /// Read-only Git/worktree cleanliness observation. `None` is unknown and
    /// `false` is retained; collection requires an explicit `Some(true)`.
    #[serde(default)]
    pub workspace_clean: Option<bool>,
    #[serde(default)]
    pub retention_until_ms: Option<u64>,
    #[serde(default)]
    pub protected_roots: Vec<AgentWorkspaceRetentionRoot>,
    #[serde(default)]
    pub collection_claim: Option<AgentWorkspaceClaimObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentWorkspaceLifecycleDecision {
    Keep {
        reasons: Vec<String>,
    },
    Renew {
        guard: LeaseGuard,
        requested_ttl_ms: u64,
    },
    DeliverCancellation {
        guard: LeaseGuard,
        cancellation_generation: u64,
    },
    Recover {
        reason: String,
    },
    AcquireCollectionClaim {
        target: String,
    },
    Collect {
        target: String,
        guard: LeaseGuard,
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
pub struct AgentWorkspaceLifecyclePlan {
    pub schema: String,
    pub workload_id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub mutates: bool,
    pub decision: AgentWorkspaceLifecycleDecision,
}

pub fn reconcile_agent_workspace_lifecycle(
    observation: &AgentWorkspaceLifecycleObservation,
    now_ms: u64,
    renewal_window_ms: u64,
    renewal_ttl_ms: u64,
) -> AgentWorkspaceLifecyclePlan {
    let workspace = &observation.workspace;
    let target = agent_workspace_target(&workspace.repository_id, &workspace.workspace_id);
    let mut violations = Vec::new();
    if observation.schema != AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA {
        violations.push("invalid_lifecycle_observation_schema".to_string());
    }
    if observation.workload_id.trim().is_empty()
        || workspace.repository_id.trim().is_empty()
        || workspace.workspace_id.trim().is_empty()
        || workspace.node_id.trim().is_empty()
    {
        violations.push("workspace_lifecycle_identity_missing".to_string());
    }
    if workspace.exists && !workspace.isolated {
        violations.push("shared_workspace_collection_forbidden".to_string());
    }
    if !violations.is_empty() {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Blocked { violations },
        );
    }
    if !workspace.exists {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Keep {
                reasons: vec!["workspace_absent".to_string()],
            },
        );
    }

    if !observation.workload_state.terminal() {
        return active_workload_plan(observation, now_ms, renewal_window_ms, renewal_ttl_ms);
    }

    let Some(completion) = observation.completion_evidence.as_ref() else {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Unknown {
                missing_evidence: vec!["completion_evidence".to_string()],
            },
        );
    };
    let evidence_violations = validate_evidence_refs(std::slice::from_ref(completion));
    if !evidence_violations.is_empty() {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Blocked {
                violations: evidence_violations
                    .into_iter()
                    .map(|item| format!("completion_evidence_{}", item.code))
                    .collect(),
            },
        );
    }
    if completion.kind != "agent_completion" || completion.subject != observation.workload_id {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Blocked {
                violations: vec!["completion_evidence_identity_mismatch".to_string()],
            },
        );
    }
    match observation.workspace_clean {
        None => {
            return plan(
                observation,
                AgentWorkspaceLifecycleDecision::Unknown {
                    missing_evidence: vec!["workspace_clean".to_string()],
                },
            );
        }
        Some(false) => {
            return plan(
                observation,
                AgentWorkspaceLifecycleDecision::Keep {
                    reasons: vec!["unpersisted_workspace_changes".to_string()],
                },
            );
        }
        Some(true) => {}
    }
    let Some(retention_until_ms) = observation.retention_until_ms else {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Unknown {
                missing_evidence: vec!["retention_until_ms".to_string()],
            },
        );
    };
    if !observation.protected_roots.is_empty() {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Keep {
                reasons: observation
                    .protected_roots
                    .iter()
                    .map(|root| format!("protected_root:{}:{}", root.kind, root.id))
                    .collect(),
            },
        );
    }
    if observation
        .workspace
        .repository_claims
        .iter()
        .any(|claim| claim.expires_at_ms > now_ms)
    {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Keep {
                reasons: vec!["live_repository_claim".to_string()],
            },
        );
    }
    if now_ms < retention_until_ms {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Keep {
                reasons: vec!["retention_not_expired".to_string()],
            },
        );
    }

    match observation.collection_claim.as_ref() {
        Some(claim)
            if claim.expires_at_ms > now_ms
                && claim.hard_expires_at_ms > now_ms
                && claim.workspace_id == workspace.workspace_id
                && claim.node_id == workspace.node_id
                && claim.guard.target == target
                && claim.guard.fencing_token > 0 =>
        {
            plan(
                observation,
                AgentWorkspaceLifecycleDecision::Collect {
                    target,
                    guard: claim.guard.clone(),
                    completion_evidence_digest: completion.digest.clone(),
                },
            )
        }
        _ => plan(
            observation,
            AgentWorkspaceLifecycleDecision::AcquireCollectionClaim { target },
        ),
    }
}

fn active_workload_plan(
    observation: &AgentWorkspaceLifecycleObservation,
    now_ms: u64,
    renewal_window_ms: u64,
    renewal_ttl_ms: u64,
) -> AgentWorkspaceLifecyclePlan {
    let Some(claim) = observation.workload_claim.as_ref() else {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Recover {
                reason: "active_workspace_without_workload_claim".to_string(),
            },
        );
    };
    if claim.expires_at_ms <= now_ms {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Recover {
                reason: "workload_claim_expired".to_string(),
            },
        );
    }
    if observation.workload_state == AgentWorkloadState::CancelRequested {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::DeliverCancellation {
                guard: claim.guard.clone(),
                cancellation_generation: claim.cancellation_generation,
            },
        );
    }
    if claim.expires_at_ms.saturating_sub(now_ms) <= renewal_window_ms {
        return plan(
            observation,
            AgentWorkspaceLifecycleDecision::Renew {
                guard: claim.guard.clone(),
                requested_ttl_ms: renewal_ttl_ms,
            },
        );
    }
    plan(
        observation,
        AgentWorkspaceLifecycleDecision::Keep {
            reasons: vec!["active_workload_claim_fresh".to_string()],
        },
    )
}

fn plan(
    observation: &AgentWorkspaceLifecycleObservation,
    decision: AgentWorkspaceLifecycleDecision,
) -> AgentWorkspaceLifecyclePlan {
    let mutates = matches!(
        &decision,
        AgentWorkspaceLifecycleDecision::Renew { .. }
            | AgentWorkspaceLifecycleDecision::DeliverCancellation { .. }
            | AgentWorkspaceLifecycleDecision::Recover { .. }
            | AgentWorkspaceLifecycleDecision::AcquireCollectionClaim { .. }
            | AgentWorkspaceLifecycleDecision::Collect { .. }
    );
    AgentWorkspaceLifecyclePlan {
        schema: AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA.to_string(),
        workload_id: observation.workload_id.clone(),
        repository_id: observation.workspace.repository_id.clone(),
        workspace_id: observation.workspace.workspace_id.clone(),
        mutates,
        decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AGENT_WORKSPACE_OBSERVATION_SCHEMA, AgentWorkspaceClaimObservation, AgentWorkspaceZone,
        AgentWorkspaceZoneKind,
    };

    fn guard(target: &str, token: u64) -> LeaseGuard {
        LeaseGuard {
            agent_id: "agent-1".into(),
            target: target.into(),
            lease_id: "lease-1".into(),
            fencing_token: token,
        }
    }

    fn observation(state: AgentWorkloadState) -> AgentWorkspaceLifecycleObservation {
        AgentWorkspaceLifecycleObservation {
            schema: AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA.into(),
            workload_id: "workload-1".into(),
            workload_state: state,
            workload_claim: None,
            workspace: AgentWorkspaceObservation {
                schema: AGENT_WORKSPACE_OBSERVATION_SCHEMA.into(),
                repository_id: "cynic".into(),
                workspace_id: "workspace-1".into(),
                node_id: "node-1".into(),
                exists: true,
                isolated: true,
                head_commit: Some("a".repeat(40)),
                path_preimages: vec![],
                repository_claims: vec![],
                evidence_refs: vec![],
            },
            completion_evidence: None,
            workspace_clean: Some(true),
            retention_until_ms: None,
            protected_roots: vec![],
            collection_claim: None,
        }
    }

    fn completion() -> AgentEvidenceRef {
        AgentEvidenceRef {
            kind: "agent_completion".into(),
            subject: "workload-1".into(),
            digest: format!("sha256:{}", "a".repeat(64)),
            lineage: None,
        }
    }

    fn lifecycle_record() -> AgentWorkspaceLifecycleRecord {
        AgentWorkspaceLifecycleRecord {
            schema: AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA.into(),
            workload_id: "workload-1".into(),
            repository_id: "cynic".into(),
            workspace_id: "workspace-1".into(),
            node_id: "node-1".into(),
            completion_evidence: completion(),
            completed_at_ms: 100,
            retention_until_ms: 200,
            protected_roots: vec![],
        }
    }

    #[test]
    fn active_claim_near_expiry_is_renewed() {
        let mut observed = observation(AgentWorkloadState::Claimed);
        observed.workload_claim = Some(AgentWorkloadClaim {
            guard: guard("agent-workload:workload-1", 2),
            expires_at_ms: 120,
            cancellation_generation: 0,
            granted_at_ms: 0,
            max_lease_window_ms: 1000,
        });
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::Renew { .. }
        ));
    }

    #[test]
    fn cancellation_is_delivered_before_renewal() {
        let mut observed = observation(AgentWorkloadState::CancelRequested);
        observed.workload_claim = Some(AgentWorkloadClaim {
            guard: guard("agent-workload:workload-1", 2),
            expires_at_ms: 120,
            cancellation_generation: 1,
            granted_at_ms: 0,
            max_lease_window_ms: 1000,
        });
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::DeliverCancellation { .. }
        ));
    }

    #[test]
    fn terminal_workspace_without_completion_proof_is_unknown() {
        let observed = observation(AgentWorkloadState::Completed);
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::Unknown { .. }
        ));
    }

    #[test]
    fn protected_root_prevents_collection() {
        let mut observed = observation(AgentWorkloadState::Completed);
        observed.completion_evidence = Some(completion());
        observed.retention_until_ms = Some(50);
        observed.protected_roots.push(AgentWorkspaceRetentionRoot {
            kind: "release".into(),
            id: "release-1".into(),
        });
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::Keep { .. }
        ));
    }

    #[test]
    fn dirty_workspace_is_never_collected() {
        let mut observed = observation(AgentWorkloadState::Completed);
        observed.completion_evidence = Some(completion());
        observed.workspace_clean = Some(false);
        observed.retention_until_ms = Some(50);
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::Keep { .. }
        ));
    }

    #[test]
    fn completion_proof_must_be_bound_to_the_workload() {
        let mut observed = observation(AgentWorkloadState::Completed);
        let mut wrong = completion();
        wrong.subject = "other-workload".into();
        observed.completion_evidence = Some(wrong);
        observed.retention_until_ms = Some(50);
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::Blocked { .. }
        ));
    }

    #[test]
    fn collection_requires_a_fresh_fenced_claim() {
        let mut observed = observation(AgentWorkloadState::Completed);
        observed.completion_evidence = Some(completion());
        observed.retention_until_ms = Some(50);
        let first = reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000);
        assert!(matches!(
            first.decision,
            AgentWorkspaceLifecycleDecision::AcquireCollectionClaim { .. }
        ));
        let target = agent_workspace_target("cynic", "workspace-1");
        observed.collection_claim = Some(AgentWorkspaceClaimObservation {
            workload_id: "gc-workload".into(),
            workspace_id: "workspace-1".into(),
            node_id: "node-1".into(),
            agent_id: "gc-agent".into(),
            zones: vec![AgentWorkspaceZone {
                kind: AgentWorkspaceZoneKind::Tree,
                path: ".".into(),
            }],
            guard: guard(&target, 3),
            expires_at_ms: 200,
            hard_expires_at_ms: 300,
        });
        assert!(matches!(
            reconcile_agent_workspace_lifecycle(&observed, 100, 30, 1000).decision,
            AgentWorkspaceLifecycleDecision::Collect { .. }
        ));
    }

    #[test]
    fn durable_record_binds_completion_and_retention() {
        assert!(validate_agent_workspace_lifecycle_record(&lifecycle_record()).is_empty());
        let mut invalid = lifecycle_record();
        invalid.retention_until_ms = 99;
        assert!(
            validate_agent_workspace_lifecycle_record(&invalid)
                .contains(&"workspace_lifecycle_retention_precedes_completion".to_string())
        );
    }
}
