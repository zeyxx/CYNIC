use serde::{Deserialize, Serialize};

use crate::SecurityEvidenceState;

pub const NODE_IDENTITY_DESIRED_STATE_SCHEMA: &str = "cynic.node_identity.desired_state.v1";
pub const NODE_IDENTITY_OBSERVATION_SCHEMA: &str = "cynic.node_identity.observation.v1";
pub const NODE_IDENTITY_PLAN_SCHEMA: &str = "cynic.node_identity.plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeIdentityDesiredState {
    pub schema: String,
    pub node_id: String,
    pub signing_key_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeIdentityObservation {
    pub schema: String,
    pub node_id: String,
    pub key_state: SecurityEvidenceState,
    pub private_key_mode: Option<u32>,
    pub machine_identity_digest: Option<String>,
    pub machine_public_key_hex: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeIdentityDecision {
    Generate,
    AlreadyPresent,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeIdentityPlan {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub decision: NodeIdentityDecision,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rollback_intent: String,
}

pub fn reconcile_node_identity(
    desired: &NodeIdentityDesiredState,
    observed: &NodeIdentityObservation,
) -> NodeIdentityPlan {
    let mut blockers = Vec::new();
    let mut unknowns = observed.unknowns.clone();
    check(
        desired.schema == NODE_IDENTITY_DESIRED_STATE_SCHEMA,
        "node_identity_desired_state_schema_mismatch",
        &mut blockers,
    );
    check(
        observed.schema == NODE_IDENTITY_OBSERVATION_SCHEMA,
        "node_identity_observation_schema_mismatch",
        &mut blockers,
    );
    check(
        valid_node_id(&desired.node_id),
        "node_identity_node_id_invalid",
        &mut blockers,
    );
    check(
        desired.node_id == observed.node_id,
        "node_identity_node_binding_mismatch",
        &mut blockers,
    );
    check(
        valid_absolute_path(&desired.signing_key_path),
        "node_identity_signing_key_path_invalid",
        &mut blockers,
    );

    let decision = match observed.key_state {
        SecurityEvidenceState::Absent if blockers.is_empty() => NodeIdentityDecision::Generate,
        SecurityEvidenceState::Proven => {
            check(
                observed
                    .private_key_mode
                    .is_some_and(|mode| mode & 0o077 == 0),
                "node_identity_private_key_mode_unsafe",
                &mut blockers,
            );
            check(
                observed
                    .machine_public_key_hex
                    .as_deref()
                    .is_some_and(valid_hex_32),
                "node_identity_public_key_invalid",
                &mut blockers,
            );
            check(
                observed
                    .machine_identity_digest
                    .as_deref()
                    .is_some_and(valid_sha256),
                "node_identity_digest_invalid",
                &mut blockers,
            );
            if blockers.is_empty() {
                NodeIdentityDecision::AlreadyPresent
            } else {
                NodeIdentityDecision::Blocked
            }
        }
        SecurityEvidenceState::Unknown | SecurityEvidenceState::Declared => {
            unknowns.push("node_identity_state_unproved".to_string());
            NodeIdentityDecision::Unknown
        }
        SecurityEvidenceState::Absent => NodeIdentityDecision::Blocked,
        SecurityEvidenceState::Refuted | SecurityEvidenceState::NotApplicable => {
            blockers.push("node_identity_material_invalid".to_string());
            NodeIdentityDecision::Blocked
        }
    };
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    let status = match decision {
        NodeIdentityDecision::Generate => "planned",
        NodeIdentityDecision::AlreadyPresent => "converged",
        NodeIdentityDecision::Unknown => "unknown",
        NodeIdentityDecision::Blocked => "blocked",
    };
    NodeIdentityPlan {
        schema: NODE_IDENTITY_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        node_id: desired.node_id.clone(),
        decision,
        blockers,
        unknowns,
        evidence_refs: observed.evidence_refs.clone(),
        rollback_intent: "retain an already-proven identity; remove only a newly generated key before it is admitted by the enrollment authority".to_string(),
    }
}

fn valid_node_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
}

fn valid_absolute_path(value: &str) -> bool {
    value.starts_with('/') && !value.contains('\0') && !value.split('/').any(|part| part == "..")
}

fn valid_hex_32(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(valid_hex_32)
}

fn check(condition: bool, blocker: &str, blockers: &mut Vec<String>) {
    if !condition {
        blockers.push(blocker.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desired() -> NodeIdentityDesiredState {
        NodeIdentityDesiredState {
            schema: NODE_IDENTITY_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            signing_key_path: "/var/lib/cynic/identity/node-signing.key".to_string(),
        }
    }

    #[test]
    fn absent_identity_plans_local_generation() {
        let observed = NodeIdentityObservation {
            schema: NODE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            key_state: SecurityEvidenceState::Absent,
            private_key_mode: None,
            machine_identity_digest: None,
            machine_public_key_hex: None,
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };

        let plan = reconcile_node_identity(&desired(), &observed);

        assert_eq!(plan.status, "planned");
        assert_eq!(plan.decision, NodeIdentityDecision::Generate);
    }

    #[test]
    fn unsafe_existing_private_key_is_blocked_not_replaced() {
        let observed = NodeIdentityObservation {
            schema: NODE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            key_state: SecurityEvidenceState::Proven,
            private_key_mode: Some(0o644),
            machine_identity_digest: Some(format!("sha256:{}", "a".repeat(64))),
            machine_public_key_hex: Some("b".repeat(64)),
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };

        let plan = reconcile_node_identity(&desired(), &observed);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"node_identity_private_key_mode_unsafe".to_string())
        );
    }
}
