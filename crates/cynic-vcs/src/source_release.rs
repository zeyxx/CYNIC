use serde::{Deserialize, Serialize};

use cynic_protocol::SecurityEvidenceState;
use cynic_protocol::argv_allow::canonical_build_actions;

pub const SOURCE_RELEASE_REQUEST_SCHEMA: &str = "cynic.source.release_request.v1";
pub const SOURCE_RELEASE_OBSERVATION_SCHEMA: &str = "cynic.source.release_observation.v1";
pub const SOURCE_RELEASE_PLAN_SCHEMA: &str = "cynic.source.release_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReleaseRequest {
    pub schema: String,
    pub release_id: String,
    pub proposal_id: String,
    pub repository_id: String,
    pub sovereign_remote: String,
    pub sovereign_ref: String,
    pub source_commit: String,
    pub build_runner_node_id: String,
    pub retention_until_unix_ms: u64,
    pub requested_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReleaseObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub proposal_admission: SecurityEvidenceState,
    pub worktree_clean: SecurityEvidenceState,
    pub sovereign_transport_identity: SecurityEvidenceState,
    pub sovereign_authentication: SecurityEvidenceState,
    pub remote_ref_commit: Option<String>,
    pub snapshot_digest: Option<String>,
    pub snapshot_digest_algorithm: Option<String>,
    pub snapshot_store_read_back: SecurityEvidenceState,
    pub build_runner_role: Option<String>,
    pub build_runner_identity: SecurityEvidenceState,
    pub build_runner_materialization_port: SecurityEvidenceState,
    pub grant_signing_authority: SecurityEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReleasePlan {
    pub schema: String,
    pub status: String,
    pub release_id: String,
    pub source_commit: String,
    pub snapshot_digest: Option<String>,
    pub build_runner_node_id: String,
    pub action_budget: u32,
    pub actions: Vec<SourceReleaseAction>,
    pub stop_conditions: Vec<String>,
    pub retention_until_unix_ms: u64,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SourceReleaseAction {
    MaterializeSnapshot {
        repository_id: String,
        source_commit: String,
        snapshot_digest: String,
        target_node_id: String,
    },
    VerifyCheckout {
        source_commit: String,
        snapshot_digest: String,
        target_node_id: String,
    },
    DispatchBuildWorkload {
        source_commit: String,
        target_node_id: String,
        requested_action: String,
        retention_until_unix_ms: u64,
    },
}

pub fn plan_source_release(
    request: &SourceReleaseRequest,
    observation: &SourceReleaseObservation,
    now_unix_ms: u64,
) -> SourceReleasePlan {
    let mut blockers = Vec::new();
    if request.schema != SOURCE_RELEASE_REQUEST_SCHEMA {
        blockers.push("source_release_request_schema_mismatch".to_string());
    }
    if observation.schema != SOURCE_RELEASE_OBSERVATION_SCHEMA {
        blockers.push("source_release_observation_schema_mismatch".to_string());
    }
    for (value, blocker) in [
        (&request.release_id, "source_release_id_missing"),
        (&request.proposal_id, "source_release_proposal_id_missing"),
        (
            &request.repository_id,
            "source_release_repository_id_missing",
        ),
        (
            &request.sovereign_remote,
            "source_release_sovereign_remote_missing",
        ),
        (
            &request.sovereign_ref,
            "source_release_sovereign_ref_missing",
        ),
        (&request.source_commit, "source_release_commit_missing"),
        (
            &request.build_runner_node_id,
            "source_release_build_runner_missing",
        ),
    ] {
        if value.trim().is_empty() {
            blockers.push(blocker.to_string());
        }
    }
    if request.retention_until_unix_ms <= now_unix_ms {
        blockers.push("source_release_retention_expired".to_string());
    }
    if canonical_build_actions(&request.requested_action).is_err() {
        blockers.push("source_release_action_outside_build_authority".to_string());
    }
    for (state, blocker) in [
        (
            observation.proposal_admission,
            "source_change_proposal_unadmitted",
        ),
        (observation.worktree_clean, "source_worktree_clean_unproven"),
        (
            observation.sovereign_transport_identity,
            "sovereign_transport_identity_unproven",
        ),
        (
            observation.sovereign_authentication,
            "sovereign_authentication_unproven",
        ),
        (
            observation.snapshot_store_read_back,
            "source_snapshot_store_read_back_unproven",
        ),
        (
            observation.build_runner_identity,
            "build_runner_identity_unproven",
        ),
        (
            observation.build_runner_materialization_port,
            "build_runner_materialization_port_unproven",
        ),
        (
            observation.grant_signing_authority,
            "build_grant_signing_authority_unproven",
        ),
    ] {
        if state != SecurityEvidenceState::Proven {
            blockers.push(blocker.to_string());
        }
    }
    if observation.remote_ref_commit.as_deref() != Some(request.source_commit.as_str()) {
        blockers.push("sovereign_remote_commit_read_back_mismatch".to_string());
    }
    let digest = observation
        .snapshot_digest
        .as_deref()
        .filter(|digest| digest.starts_with("sha256:") && digest.len() == 71);
    if digest.is_none() || observation.snapshot_digest_algorithm.as_deref() != Some("sha256") {
        blockers.push("source_snapshot_digest_unproven".to_string());
    }
    if observation.build_runner_role.as_deref() != Some("build_runner") {
        blockers.push("source_release_target_not_build_runner".to_string());
    }
    if observation.evidence_refs.is_empty() {
        blockers.push("source_release_evidence_missing".to_string());
    }

    let actions = if blockers.is_empty() {
        let digest = digest.unwrap_or_default().to_string();
        vec![
            SourceReleaseAction::MaterializeSnapshot {
                repository_id: request.repository_id.clone(),
                source_commit: request.source_commit.clone(),
                snapshot_digest: digest.clone(),
                target_node_id: request.build_runner_node_id.clone(),
            },
            SourceReleaseAction::VerifyCheckout {
                source_commit: request.source_commit.clone(),
                snapshot_digest: digest,
                target_node_id: request.build_runner_node_id.clone(),
            },
            SourceReleaseAction::DispatchBuildWorkload {
                source_commit: request.source_commit.clone(),
                target_node_id: request.build_runner_node_id.clone(),
                requested_action: request.requested_action.clone(),
                retention_until_unix_ms: request.retention_until_unix_ms,
            },
        ]
    } else {
        Vec::new()
    };

    SourceReleasePlan {
        schema: SOURCE_RELEASE_PLAN_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted".to_string()
        } else {
            "blocked".to_string()
        },
        release_id: request.release_id.clone(),
        source_commit: request.source_commit.clone(),
        snapshot_digest: digest.map(str::to_string),
        build_runner_node_id: request.build_runner_node_id.clone(),
        action_budget: actions.len() as u32,
        actions,
        stop_conditions: vec![
            "stop_on_first_action_failure".to_string(),
            "stop_on_commit_or_digest_read_back_mismatch".to_string(),
            "stop_when_retention_expires".to_string(),
        ],
        retention_until_unix_ms: request.retention_until_unix_ms,
        blockers,
        evidence_refs: observation.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_request() -> SourceReleaseRequest {
        SourceReleaseRequest {
            schema: SOURCE_RELEASE_REQUEST_SCHEMA.to_string(),
            release_id: "release-1".to_string(),
            proposal_id: "proposal-1".to_string(),
            repository_id: "cynic".to_string(),
            sovereign_remote: "forgejo".to_string(),
            sovereign_ref: "refs/heads/release/storage".to_string(),
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
            build_runner_node_id: "cynic-builder".to_string(),
            retention_until_unix_ms: 20_000,
            requested_action: "make:check".to_string(),
        }
    }

    fn release_observation() -> SourceReleaseObservation {
        SourceReleaseObservation {
            schema: SOURCE_RELEASE_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 10_000,
            proposal_admission: SecurityEvidenceState::Proven,
            worktree_clean: SecurityEvidenceState::Proven,
            sovereign_transport_identity: SecurityEvidenceState::Proven,
            sovereign_authentication: SecurityEvidenceState::Proven,
            remote_ref_commit: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            snapshot_digest: Some(format!("sha256:{}", "a".repeat(64))),
            snapshot_digest_algorithm: Some("sha256".to_string()),
            snapshot_store_read_back: SecurityEvidenceState::Proven,
            build_runner_role: Some("build_runner".to_string()),
            build_runner_identity: SecurityEvidenceState::Proven,
            build_runner_materialization_port: SecurityEvidenceState::Proven,
            grant_signing_authority: SecurityEvidenceState::Proven,
            evidence_refs: vec![
                "forgejo:read-back:release/storage".to_string(),
                "snapshot-store:sha256".to_string(),
                "runner:cynic-builder".to_string(),
            ],
        }
    }

    #[test]
    fn release_plan_requires_sovereign_remote_read_back() {
        let request = release_request();
        let mut observation = release_observation();
        observation.remote_ref_commit = Some("different".to_string());

        let plan = plan_source_release(&request, &observation, 10_000);

        assert_eq!(plan.status, "blocked");
        assert!(plan.actions.is_empty());
        assert!(
            plan.blockers
                .contains(&"sovereign_remote_commit_read_back_mismatch".to_string())
        );
    }

    #[test]
    fn admitted_release_is_typed_and_bounded() {
        let plan = plan_source_release(&release_request(), &release_observation(), 10_000);

        assert_eq!(plan.status, "admitted");
        assert_eq!(plan.action_budget, 3);
        assert_eq!(plan.actions.len(), 3);
        assert!(matches!(
            plan.actions.last(),
            Some(SourceReleaseAction::DispatchBuildWorkload { .. })
        ));
    }

    #[test]
    fn caller_cannot_supply_nested_or_arbitrary_grant_actions() {
        let mut request = release_request();
        request.requested_action = "cargo:build".to_string();

        let plan = plan_source_release(&request, &release_observation(), 10_000);

        assert_eq!(plan.status, "blocked");
        assert!(plan.actions.is_empty());
        assert!(
            plan.blockers
                .contains(&"source_release_action_outside_build_authority".to_string())
        );
    }
}
