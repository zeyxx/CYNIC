//! Dynamic mesh admission rules and decision predicates (FCIS Pure Core).

use cynic_types::fleet_mesh::PreAuthKeySpec;
use serde::{Deserialize, Serialize};

/// Maximum allowed validity window for an ephemeral pre-auth key (24 hours).
pub const MAX_PREAUTH_EXPIRATION_SECS: u64 = 86_400;

/// Policy governing node pre-authentication admission into the WireGuard mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshAdmissionPolicy {
    /// Mandated user namespace (usually "cynic").
    pub mandated_user: String,
    /// Whether ephemeral keys are required for worker nodes.
    pub require_ephemeral: bool,
    /// Mandated ACL tag prefix (e.g. "tag:agent" or "tag:node").
    pub mandated_tag_prefix: String,
}

impl Default for MeshAdmissionPolicy {
    fn default() -> Self {
        Self {
            mandated_user: "cynic".to_string(),
            require_ephemeral: true,
            mandated_tag_prefix: "tag:".to_string(),
        }
    }
}

/// Admission decision verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionDecision {
    pub admitted: bool,
    pub reason: String,
}

/// Pure predicate evaluating whether a PreAuthKeySpec satisfies the security admission policy.
#[must_use]
pub fn evaluate_preauth_spec(
    policy: &MeshAdmissionPolicy,
    spec: &PreAuthKeySpec,
) -> AdmissionDecision {
    if spec.user != policy.mandated_user {
        return AdmissionDecision {
            admitted: false,
            reason: format!(
                "invalid user '{}': policy mandates '{}'",
                spec.user, policy.mandated_user
            ),
        };
    }

    if policy.require_ephemeral && !spec.ephemeral {
        return AdmissionDecision {
            admitted: false,
            reason: "policy mandates ephemeral keys for mesh nodes".to_string(),
        };
    }

    if spec.acl_tags.is_empty() {
        return AdmissionDecision {
            admitted: false,
            reason: "preauth key spec must contain at least one ACL tag".to_string(),
        };
    }

    for tag in &spec.acl_tags {
        if !tag.starts_with(&policy.mandated_tag_prefix) {
            return AdmissionDecision {
                admitted: false,
                reason: format!(
                    "invalid ACL tag '{}': must start with '{}'",
                    tag, policy.mandated_tag_prefix
                ),
            };
        }
    }

    AdmissionDecision {
        admitted: true,
        reason: "preauth spec complies with CYNIC mesh admission policy".to_string(),
    }
}
