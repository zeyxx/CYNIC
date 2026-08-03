use serde::{Deserialize, Serialize};

use crate::digest_json;

pub const DEPLOYMENT_EPOCH_SCHEMA: &str = "cynic.deployment_epoch.v1";
pub const DEPLOYMENT_TRANSITION_SCHEMA: &str = "cynic.deployment_transition.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentEpoch {
    pub schema: String,
    pub epoch_id: String,
    pub source_commit: String,
    pub desired_state_digest: String,
    pub target_scope: String,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentPhase {
    Proposed,
    Admitted,
    Building,
    Built,
    Staging,
    Staged,
    Canary,
    RollingOut,
    Verifying,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentExecutionClass {
    Synchronous,
    Asynchronous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentPhaseObservation {
    pub schema: String,
    pub epoch_id: String,
    pub phase: DeploymentPhase,
    pub ready: bool,
    pub pending: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentTransition {
    pub schema: String,
    pub epoch_id: String,
    pub from: DeploymentPhase,
    pub to: DeploymentPhase,
    pub execution_class: DeploymentExecutionClass,
    pub action: String,
    pub status: String,
    pub blockers: Vec<String>,
}

pub fn derive_deployment_epoch(
    source_commit: &str,
    desired_state_digest: &str,
    target_scope: &str,
    created_at_unix_ms: u64,
) -> Result<DeploymentEpoch, String> {
    if source_commit.trim().is_empty()
        || desired_state_digest.trim().is_empty()
        || target_scope.trim().is_empty()
        || created_at_unix_ms == 0
    {
        return Err("deployment_epoch_identity_incomplete".to_string());
    }
    let epoch_id = digest_json(&serde_json::json!({
        "schema": DEPLOYMENT_EPOCH_SCHEMA,
        "source_commit": source_commit,
        "desired_state_digest": desired_state_digest,
        "target_scope": target_scope,
    }))
    .map_err(|e| e.to_string())?;
    Ok(DeploymentEpoch {
        schema: DEPLOYMENT_EPOCH_SCHEMA.to_string(),
        epoch_id,
        source_commit: source_commit.to_string(),
        desired_state_digest: desired_state_digest.to_string(),
        target_scope: target_scope.to_string(),
        created_at_unix_ms,
    })
}

pub fn phase_execution_class(phase: DeploymentPhase) -> DeploymentExecutionClass {
    match phase {
        DeploymentPhase::Proposed
        | DeploymentPhase::Admitted
        | DeploymentPhase::Completed
        | DeploymentPhase::Failed => DeploymentExecutionClass::Synchronous,
        _ => DeploymentExecutionClass::Asynchronous,
    }
}

pub fn plan_deployment_transition(
    epoch: &DeploymentEpoch,
    current: DeploymentPhase,
    observation: &DeploymentPhaseObservation,
) -> DeploymentTransition {
    let mut blockers = Vec::new();
    if epoch.schema != DEPLOYMENT_EPOCH_SCHEMA {
        blockers.push("deployment_epoch_schema_mismatch".to_string());
    }
    if observation.schema != DEPLOYMENT_TRANSITION_SCHEMA || observation.epoch_id != epoch.epoch_id
    {
        blockers.push("deployment_epoch_observation_mismatch".to_string());
    }
    if observation.phase != current {
        blockers.push("deployment_phase_stale_observation".to_string());
    }
    if observation.pending {
        blockers.push("deployment_phase_pending".to_string());
    }
    blockers.extend(observation.blockers.clone());
    if !blockers.is_empty() {
        return transition(epoch, current, current, "wait_for_observation", blockers);
    }
    if !observation.ready {
        return transition(
            epoch,
            current,
            current,
            "observe_again",
            vec!["deployment_phase_not_ready".to_string()],
        );
    }
    transition(
        epoch,
        current,
        next_phase(current),
        "advance_exactly_one_phase",
        Vec::new(),
    )
}

fn next_phase(current: DeploymentPhase) -> DeploymentPhase {
    match current {
        DeploymentPhase::Proposed => DeploymentPhase::Admitted,
        DeploymentPhase::Admitted => DeploymentPhase::Building,
        DeploymentPhase::Building => DeploymentPhase::Built,
        DeploymentPhase::Built => DeploymentPhase::Staging,
        DeploymentPhase::Staging => DeploymentPhase::Staged,
        DeploymentPhase::Staged => DeploymentPhase::Canary,
        DeploymentPhase::Canary => DeploymentPhase::RollingOut,
        DeploymentPhase::RollingOut => DeploymentPhase::Verifying,
        DeploymentPhase::Verifying => DeploymentPhase::Completed,
        DeploymentPhase::Completed | DeploymentPhase::Failed => current,
    }
}

fn transition(
    epoch: &DeploymentEpoch,
    from: DeploymentPhase,
    to: DeploymentPhase,
    action: &str,
    blockers: Vec<String>,
) -> DeploymentTransition {
    DeploymentTransition {
        schema: DEPLOYMENT_TRANSITION_SCHEMA.to_string(),
        epoch_id: epoch.epoch_id.clone(),
        from,
        to,
        execution_class: phase_execution_class(to),
        action: action.to_string(),
        status: if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .to_string(),
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch() -> DeploymentEpoch {
        derive_deployment_epoch("commit-1", "sha256:policy", "fleet", 1).unwrap()
    }

    fn observed(epoch: &DeploymentEpoch, phase: DeploymentPhase) -> DeploymentPhaseObservation {
        DeploymentPhaseObservation {
            schema: DEPLOYMENT_TRANSITION_SCHEMA.to_string(),
            epoch_id: epoch.epoch_id.clone(),
            phase,
            ready: true,
            pending: false,
            blockers: Vec::new(),
            evidence_refs: Vec::new(),
        }
    }

    #[test]
    fn epoch_is_stable_for_same_intent() {
        assert_eq!(epoch().epoch_id, epoch().epoch_id);
    }

    #[test]
    fn stale_or_pending_observation_never_advances() {
        let e = epoch();
        let mut o = observed(&e, DeploymentPhase::Proposed);
        o.pending = true;
        let t = plan_deployment_transition(&e, DeploymentPhase::Proposed, &o);
        assert_eq!(t.status, "blocked");
        assert_eq!(t.to, DeploymentPhase::Proposed);
    }

    #[test]
    fn rollout_advances_one_phase_at_a_time() {
        let e = epoch();
        let t = plan_deployment_transition(
            &e,
            DeploymentPhase::Staged,
            &observed(&e, DeploymentPhase::Staged),
        );
        assert_eq!(t.to, DeploymentPhase::Canary);
        assert_eq!(t.execution_class, DeploymentExecutionClass::Asynchronous);
    }
}
