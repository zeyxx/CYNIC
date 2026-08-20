use serde::{Deserialize, Serialize};

use crate::SecurityEvidenceState;

pub const NODE_IDENTITY_DESIRED_STATE_SCHEMA: &str = "cynic.node_identity.desired_state.v1";
pub const NODE_IDENTITY_OBSERVATION_SCHEMA: &str = "cynic.node_identity.observation.v1";
pub const NODE_IDENTITY_PLAN_SCHEMA: &str = "cynic.node_identity.plan.v1";
pub const NODE_CSR_SCHEMA: &str = "cynic.node_csr.v1";
pub const NODE_ENROLLMENT_CERTIFICATE_SCHEMA: &str = "cynic.node_enrollment_certificate.v1";
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

/// Certificate Signing Request (CSR) created by a node to prove possession of its private key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCsr {
    pub schema: String,
    pub node_id: String,
    pub requested_role: String,
    pub public_key_hex: String,
    pub created_at_unix_s: u64,
    pub csr_signature_hex: String,
}

#[derive(Serialize)]
struct NodeCsrSigningPayload<'a> {
    schema: &'a str,
    node_id: &'a str,
    requested_role: &'a str,
    public_key_hex: &'a str,
    created_at_unix_s: u64,
}

impl NodeCsr {
    pub fn new(
        node_id: String,
        requested_role: String,
        public_key_hex: String,
        created_at_unix_s: u64,
        csr_signature_hex: String,
    ) -> Self {
        Self {
            schema: NODE_CSR_SCHEMA.to_string(),
            node_id,
            requested_role,
            public_key_hex,
            created_at_unix_s,
            csr_signature_hex,
        }
    }

    /// Canonical signing payload bytes for the CSR.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = NodeCsrSigningPayload {
            schema: &self.schema,
            node_id: &self.node_id,
            requested_role: &self.requested_role,
            public_key_hex: &self.public_key_hex,
            created_at_unix_s: self.created_at_unix_s,
        };
        serde_json::to_vec(&payload).expect("serialize csr payload")
    }

    /// Pure structural validation of the CSR.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        check(
            self.schema == NODE_CSR_SCHEMA,
            "node_csr_schema_mismatch",
            &mut blockers,
        );
        check(
            valid_node_id(&self.node_id),
            "node_csr_node_id_invalid",
            &mut blockers,
        );
        check(
            !self.requested_role.trim().is_empty(),
            "node_csr_requested_role_empty",
            &mut blockers,
        );
        check(
            valid_hex_32(&self.public_key_hex),
            "node_csr_public_key_invalid",
            &mut blockers,
        );
        check(
            self.created_at_unix_s > 0,
            "node_csr_created_at_invalid",
            &mut blockers,
        );
        check(
            self.csr_signature_hex.len() == 128 && self.csr_signature_hex.bytes().all(|b| b.is_ascii_hexdigit()),
            "node_csr_signature_invalid",
            &mut blockers,
        );
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

/// Enrollment certificate signed by the Kernel authority for an admitted node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentCertificate {
    pub schema: String,
    pub certificate_id: String,
    pub node_id: String,
    pub role: String,
    pub subject_public_key_hex: String,
    pub kernel_authority_id: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub kernel_signature_hex: String,
}

#[derive(Serialize)]
struct NodeEnrollmentCertificateSigningPayload<'a> {
    schema: &'a str,
    node_id: &'a str,
    role: &'a str,
    subject_public_key_hex: &'a str,
    kernel_authority_id: &'a str,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
}

impl NodeEnrollmentCertificate {
    /// Canonical signing payload bytes for the Kernel signature.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = NodeEnrollmentCertificateSigningPayload {
            schema: &self.schema,
            node_id: &self.node_id,
            role: &self.role,
            subject_public_key_hex: &self.subject_public_key_hex,
            kernel_authority_id: &self.kernel_authority_id,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
        };
        serde_json::to_vec(&payload).expect("serialize cert payload")
    }

    /// Check whether the certificate is active at a given unix timestamp.
    pub fn is_valid_at(&self, now_unix_s: u64) -> bool {
        self.issued_at_unix_s <= now_unix_s && now_unix_s <= self.expires_at_unix_s
    }

    /// Pure structural validation of the certificate.
    pub fn validate(&self, now_unix_s: u64) -> Vec<String> {
        let mut blockers = Vec::new();
        check(
            self.schema == NODE_ENROLLMENT_CERTIFICATE_SCHEMA,
            "node_certificate_schema_mismatch",
            &mut blockers,
        );
        check(
            valid_node_id(&self.node_id),
            "node_certificate_node_id_invalid",
            &mut blockers,
        );
        check(
            !self.role.trim().is_empty(),
            "node_certificate_role_empty",
            &mut blockers,
        );
        check(
            valid_hex_32(&self.subject_public_key_hex),
            "node_certificate_subject_public_key_invalid",
            &mut blockers,
        );
        check(
            !self.kernel_authority_id.trim().is_empty(),
            "node_certificate_kernel_authority_id_empty",
            &mut blockers,
        );
        check(
            self.expires_at_unix_s > self.issued_at_unix_s,
            "node_certificate_expiry_before_issuance",
            &mut blockers,
        );
        check(
            self.is_valid_at(now_unix_s),
            "node_certificate_expired_or_not_yet_valid",
            &mut blockers,
        );
        check(
            self.kernel_signature_hex.len() == 128 && self.kernel_signature_hex.bytes().all(|b| b.is_ascii_hexdigit()),
            "node_certificate_signature_invalid",
            &mut blockers,
        );
        blockers.sort();
        blockers.dedup();
        blockers
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

    #[test]
    fn csr_structural_validation() {
        let valid_csr = NodeCsr::new(
            "node-alpha".to_string(),
            "builder".to_string(),
            "a".repeat(64),
            1700000000,
            "b".repeat(128),
        );
        assert!(valid_csr.validate().is_empty());

        let invalid_csr = NodeCsr::new(
            "".to_string(),
            "".to_string(),
            "not-hex".to_string(),
            0,
            "too-short".to_string(),
        );
        let blockers = invalid_csr.validate();
        assert!(blockers.contains(&"node_csr_node_id_invalid".to_string()));
        assert!(blockers.contains(&"node_csr_requested_role_empty".to_string()));
        assert!(blockers.contains(&"node_csr_public_key_invalid".to_string()));
        assert!(blockers.contains(&"node_csr_created_at_invalid".to_string()));
        assert!(blockers.contains(&"node_csr_signature_invalid".to_string()));
    }

    #[test]
    fn certificate_validity_and_temporal_validation() {
        let cert = NodeEnrollmentCertificate {
            schema: NODE_ENROLLMENT_CERTIFICATE_SCHEMA.to_string(),
            certificate_id: "sha256:1234".to_string(),
            node_id: "node-alpha".to_string(),
            role: "builder".to_string(),
            subject_public_key_hex: "c".repeat(64),
            kernel_authority_id: "kernel-root".to_string(),
            issued_at_unix_s: 1700000000,
            expires_at_unix_s: 1700086400,
            kernel_signature_hex: "d".repeat(128),
        };

        // Within valid window
        assert!(cert.is_valid_at(1700040000));
        assert!(cert.validate(1700040000).is_empty());

        // Expired
        assert!(!cert.is_valid_at(1700100000));
        let expired_blockers = cert.validate(1700100000);
        assert!(expired_blockers.contains(&"node_certificate_expired_or_not_yet_valid".to_string()));

        // Before issuance
        assert!(!cert.is_valid_at(1699999999));
        let early_blockers = cert.validate(1699999999);
        assert!(early_blockers.contains(&"node_certificate_expired_or_not_yet_valid".to_string()));
    }
}
