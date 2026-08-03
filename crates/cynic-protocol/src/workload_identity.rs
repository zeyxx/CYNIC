use serde::{Deserialize, Serialize};

pub const WORKLOAD_IDENTITY_DESIRED_STATE_SCHEMA: &str = "cynic.workload_identity.desired_state.v1";
pub const WORKLOAD_IDENTITY_OBSERVATION_SCHEMA: &str = "cynic.workload_identity.observation.v1";
pub const WORKLOAD_IDENTITY_PLAN_SCHEMA: &str = "cynic.workload_identity.plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadIdentityDesiredState {
    pub schema: String,
    pub subject: String,
    pub trust_domain: String,
    pub audience: String,
    pub role: String,
    pub renewal_window_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadIdentityState {
    Absent,
    Present,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadIdentityObservation {
    pub schema: String,
    pub state: WorkloadIdentityState,
    pub subject: Option<String>,
    pub issuer: Option<String>,
    pub audience: Option<String>,
    pub expires_at_unix_seconds: Option<u64>,
    pub generation: Option<u64>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadIdentityAction {
    Enroll,
    Renew,
    Observe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadIdentityPlan {
    pub schema: String,
    pub status: String,
    pub subject: String,
    pub action: Option<WorkloadIdentityAction>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rollback_intent: String,
}

pub fn reconcile_workload_identity(
    desired: &WorkloadIdentityDesiredState,
    observed: &WorkloadIdentityObservation,
    now_unix_seconds: u64,
) -> WorkloadIdentityPlan {
    let mut blockers = validate_desired(desired);
    let mut unknowns = observed.unknowns.clone();
    if observed.schema != WORKLOAD_IDENTITY_OBSERVATION_SCHEMA {
        unknowns.push("workload_identity_observation_schema_mismatch".to_string());
    }
    if observed.state == WorkloadIdentityState::Unknown {
        unknowns.push("workload_identity_state_unobserved".to_string());
    }
    if observed.state == WorkloadIdentityState::Present {
        if observed.subject.as_deref() != Some(desired.subject.as_str()) {
            blockers.push("workload_identity_subject_mismatch".to_string());
        }
        if observed.audience.as_deref() != Some(desired.audience.as_str()) {
            blockers.push("workload_identity_audience_mismatch".to_string());
        }
        if observed.issuer.as_deref().is_none_or(str::is_empty) {
            unknowns.push("workload_identity_issuer_unobserved".to_string());
        }
        if observed.generation.is_none() {
            unknowns.push("workload_identity_generation_unobserved".to_string());
        }
        match observed.expires_at_unix_seconds {
            Some(expires_at) if expires_at <= now_unix_seconds => {
                blockers.push("workload_identity_expired".to_string())
            }
            Some(expires_at)
                if expires_at.saturating_sub(now_unix_seconds)
                    <= desired.renewal_window_seconds
                    && blockers.is_empty()
                    && unknowns.is_empty() =>
            {
                return plan(
                    desired,
                    "planned",
                    Some(WorkloadIdentityAction::Renew),
                    blockers,
                    unknowns,
                    observed,
                );
            }
            None => unknowns.push("workload_identity_expiry_unobserved".to_string()),
            _ => {}
        }
    }
    let action = match observed.state {
        WorkloadIdentityState::Absent => Some(WorkloadIdentityAction::Enroll),
        WorkloadIdentityState::Present => Some(WorkloadIdentityAction::Observe),
        WorkloadIdentityState::Unknown => None,
    };
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if observed.state == WorkloadIdentityState::Present {
        "converged"
    } else {
        "planned"
    };
    plan(desired, status, action, blockers, unknowns, observed)
}

fn validate_desired(desired: &WorkloadIdentityDesiredState) -> Vec<String> {
    let mut blockers = Vec::new();
    if desired.schema != WORKLOAD_IDENTITY_DESIRED_STATE_SCHEMA {
        blockers.push("workload_identity_desired_state_schema_mismatch".to_string());
    }
    for (value, blocker) in [
        (&desired.subject, "workload_identity_subject_missing"),
        (
            &desired.trust_domain,
            "workload_identity_trust_domain_missing",
        ),
        (&desired.audience, "workload_identity_audience_missing"),
        (&desired.role, "workload_identity_role_missing"),
    ] {
        if value.trim().is_empty() {
            blockers.push(blocker.to_string());
        }
    }
    if desired.renewal_window_seconds == 0 {
        blockers.push("workload_identity_renewal_window_invalid".to_string());
    }
    blockers
}

fn plan(
    desired: &WorkloadIdentityDesiredState,
    status: &str,
    action: Option<WorkloadIdentityAction>,
    mut blockers: Vec<String>,
    mut unknowns: Vec<String>,
    observed: &WorkloadIdentityObservation,
) -> WorkloadIdentityPlan {
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    WorkloadIdentityPlan {
        schema: WORKLOAD_IDENTITY_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        subject: desired.subject.clone(),
        action,
        blockers,
        unknowns,
        evidence_refs: observed.evidence_refs.clone(),
        rollback_intent: "retain the last proven identity until replacement is observed; revoke only after the new generation is proven".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desired() -> WorkloadIdentityDesiredState {
        WorkloadIdentityDesiredState {
            schema: WORKLOAD_IDENTITY_DESIRED_STATE_SCHEMA.to_string(),
            subject: "spiffe://synapse.cynic/node/cynic-builder".to_string(),
            trust_domain: "synapse.cynic".to_string(),
            audience: "cynic-control-plane".to_string(),
            role: "build_runner".to_string(),
            renewal_window_seconds: 300,
        }
    }

    #[test]
    fn absent_identity_plans_enrollment() {
        let observed = WorkloadIdentityObservation {
            schema: WORKLOAD_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            state: WorkloadIdentityState::Absent,
            subject: None,
            issuer: None,
            audience: None,
            expires_at_unix_seconds: None,
            generation: None,
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };
        let plan = reconcile_workload_identity(&desired(), &observed, 100);
        assert_eq!(plan.status, "planned");
        assert_eq!(plan.action, Some(WorkloadIdentityAction::Enroll));
    }

    #[test]
    fn expired_identity_is_blocked() {
        let observed = WorkloadIdentityObservation {
            schema: WORKLOAD_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            state: WorkloadIdentityState::Present,
            subject: Some(desired().subject),
            issuer: Some("issuer".to_string()),
            audience: Some("cynic-control-plane".to_string()),
            expires_at_unix_seconds: Some(99),
            generation: Some(1),
            evidence_refs: vec!["proof:identity".to_string()],
            unknowns: Vec::new(),
        };
        let plan = reconcile_workload_identity(&desired(), &observed, 100);
        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"workload_identity_expired".to_string())
        );
    }
}
