use serde::{Deserialize, Serialize};

use crate::SecurityEvidenceState;

pub const SOURCE_MATERIALIZATION_REQUEST_SCHEMA: &str = "cynic.source.materialization_request.v1";
pub const SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA: &str =
    "cynic.source.materialization_observation.v1";
pub const SOURCE_MATERIALIZATION_PLAN_SCHEMA: &str = "cynic.source.materialization_plan.v1";
pub const SOURCE_MATERIALIZATION_FORMAT: &str = "git_bundle_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMaterializationRequest {
    pub schema: String,
    pub release_id: String,
    pub repository_id: String,
    pub source_commit: String,
    pub snapshot_digest: String,
    pub snapshot_format: String,
    pub target_node_id: String,
    pub target_relative_path: String,
    pub required_principal: String,
    pub retention_until_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMaterializationObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub target_node_id: String,
    pub target_role: Option<String>,
    pub node_identity: SecurityEvidenceState,
    pub running_as_required_principal: SecurityEvidenceState,
    pub snapshot_present: bool,
    pub snapshot_digest_read_back: Option<String>,
    pub snapshot_format_read_back: Option<String>,
    pub target_exists: bool,
    pub target_is_isolated: SecurityEvidenceState,
    pub observed_commit: Option<String>,
    pub observed_snapshot_digest: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SourceMaterializationAction {
    AcquireSnapshot {
        snapshot_digest: String,
        snapshot_format: String,
    },
    MaterializeCheckout {
        snapshot_digest: String,
        source_commit: String,
        target_relative_path: String,
    },
    ReadBack {
        snapshot_digest: String,
        source_commit: String,
        target_relative_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMaterializationPlan {
    pub schema: String,
    pub status: String,
    pub release_id: String,
    pub action_budget: u32,
    pub actions: Vec<SourceMaterializationAction>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn plan_source_materialization(
    request: &SourceMaterializationRequest,
    observation: &SourceMaterializationObservation,
    now_unix_ms: u64,
) -> SourceMaterializationPlan {
    let mut blockers = Vec::new();
    let mut unknowns = observation.unknowns.clone();
    if request.schema != SOURCE_MATERIALIZATION_REQUEST_SCHEMA {
        blockers.push("source_materialization_request_schema_mismatch".to_string());
    }
    if observation.schema != SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA {
        blockers.push("source_materialization_observation_schema_mismatch".to_string());
    }
    if request.target_node_id != observation.target_node_id {
        blockers.push("source_materialization_node_mismatch".to_string());
    }
    if observation.target_role.as_deref() != Some("build_runner") {
        blockers.push("source_materialization_target_not_build_runner".to_string());
    }
    if observation.node_identity != SecurityEvidenceState::Proven {
        unknowns.push("source_materialization_node_identity_unproven".to_string());
    }
    if observation.running_as_required_principal != SecurityEvidenceState::Proven {
        blockers.push("source_materialization_principal_mismatch".to_string());
    }
    if request.retention_until_unix_ms <= now_unix_ms {
        blockers.push("source_materialization_retention_expired".to_string());
    }
    if request.snapshot_format != SOURCE_MATERIALIZATION_FORMAT {
        blockers.push("source_materialization_format_unsupported".to_string());
    }
    if !valid_sha256_digest(&request.snapshot_digest) {
        blockers.push("source_materialization_digest_invalid".to_string());
    }
    if request.source_commit.len() != 40
        || !request
            .source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("source_materialization_commit_invalid".to_string());
    }
    if !safe_relative_path(&request.target_relative_path) {
        blockers.push("source_materialization_target_path_unsafe".to_string());
    }
    if !safe_identifier(&request.release_id)
        || !safe_identifier(&request.repository_id)
        || !safe_identifier(&request.required_principal)
    {
        blockers.push("source_materialization_identity_incomplete".to_string());
    }
    if observation.target_exists
        && (observation.target_is_isolated != SecurityEvidenceState::Proven
            || observation.observed_commit.as_deref() != Some(request.source_commit.as_str())
            || observation.observed_snapshot_digest.as_deref()
                != Some(request.snapshot_digest.as_str()))
    {
        blockers.push("source_materialization_existing_target_drift".to_string());
    }
    if observation.snapshot_present
        && (observation.snapshot_digest_read_back.as_deref()
            != Some(request.snapshot_digest.as_str())
            || observation.snapshot_format_read_back.as_deref()
                != Some(request.snapshot_format.as_str()))
    {
        blockers.push("source_materialization_snapshot_read_back_mismatch".to_string());
    }
    if observation.evidence_refs.is_empty() {
        unknowns.push("source_materialization_evidence_missing".to_string());
    }

    unknowns.sort();
    unknowns.dedup();
    let actions = if blockers.is_empty() && unknowns.is_empty() {
        if observation.target_exists {
            Vec::new()
        } else if observation.snapshot_present {
            vec![
                SourceMaterializationAction::MaterializeCheckout {
                    snapshot_digest: request.snapshot_digest.clone(),
                    source_commit: request.source_commit.clone(),
                    target_relative_path: request.target_relative_path.clone(),
                },
                SourceMaterializationAction::ReadBack {
                    snapshot_digest: request.snapshot_digest.clone(),
                    source_commit: request.source_commit.clone(),
                    target_relative_path: request.target_relative_path.clone(),
                },
            ]
        } else {
            vec![
                SourceMaterializationAction::AcquireSnapshot {
                    snapshot_digest: request.snapshot_digest.clone(),
                    snapshot_format: request.snapshot_format.clone(),
                },
                SourceMaterializationAction::MaterializeCheckout {
                    snapshot_digest: request.snapshot_digest.clone(),
                    source_commit: request.source_commit.clone(),
                    target_relative_path: request.target_relative_path.clone(),
                },
                SourceMaterializationAction::ReadBack {
                    snapshot_digest: request.snapshot_digest.clone(),
                    source_commit: request.source_commit.clone(),
                    target_relative_path: request.target_relative_path.clone(),
                },
            ]
        }
    } else {
        Vec::new()
    };
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if observation.target_exists {
        "converged"
    } else {
        "planned"
    };

    SourceMaterializationPlan {
        schema: SOURCE_MATERIALIZATION_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        release_id: request.release_id.clone(),
        action_budget: actions.len() as u32,
        actions,
        blockers,
        unknowns,
        stop_conditions: vec![
            "stop_on_first_action_failure".to_string(),
            "stop_on_snapshot_digest_mismatch".to_string(),
            "stop_on_checkout_commit_mismatch".to_string(),
            "stop_when_retention_expires".to_string(),
        ],
        evidence_refs: observation.evidence_refs.clone(),
    }
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SourceMaterializationRequest {
        SourceMaterializationRequest {
            schema: SOURCE_MATERIALIZATION_REQUEST_SCHEMA.to_string(),
            release_id: "release-1".to_string(),
            repository_id: "cynic".to_string(),
            source_commit: "a".repeat(40),
            snapshot_digest: format!("sha256:{}", "b".repeat(64)),
            snapshot_format: SOURCE_MATERIALIZATION_FORMAT.to_string(),
            target_node_id: "cynic-builder".to_string(),
            target_relative_path: "source/releases/release-1".to_string(),
            required_principal: "cynic".to_string(),
            retention_until_unix_ms: 20_000,
        }
    }

    fn observation() -> SourceMaterializationObservation {
        SourceMaterializationObservation {
            schema: SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 10_000,
            target_node_id: "cynic-builder".to_string(),
            target_role: Some("build_runner".to_string()),
            node_identity: SecurityEvidenceState::Proven,
            running_as_required_principal: SecurityEvidenceState::Proven,
            snapshot_present: false,
            snapshot_digest_read_back: None,
            snapshot_format_read_back: None,
            target_exists: false,
            target_is_isolated: SecurityEvidenceState::Proven,
            observed_commit: None,
            observed_snapshot_digest: None,
            evidence_refs: vec!["runner:cynic-builder".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn missing_snapshot_plans_acquire_materialize_and_read_back() {
        let plan = plan_source_materialization(&request(), &observation(), 10_000);
        assert_eq!(plan.status, "planned");
        assert_eq!(plan.action_budget, 3);
    }

    #[test]
    fn existing_target_with_digest_drift_is_blocked() {
        let mut observation = observation();
        observation.target_exists = true;
        observation.observed_commit = Some("a".repeat(40));
        observation.observed_snapshot_digest = Some(format!("sha256:{}", "c".repeat(64)));
        let plan = plan_source_materialization(&request(), &observation, 10_000);
        assert_eq!(plan.status, "blocked");
        assert!(plan.actions.is_empty());
    }
}
