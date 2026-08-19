//! Pure-typed `MachineEnrollment` contract.
//!
//! This lives in the *protocol* crate so that any surface (host, node,
//! CLI, MCP) can validate an enrollment record against an observed
//! node identity.  The cryptographic operations themselves (sign/verify)
//! live in `cynic-host` (`LocalCapabilitySigningKey`), keeping cryptography
//! out of the portable protocol layer.
//!
//! Authority: `contracts/foundation.json` (ContractMachineEnrollment).

use sha2::{Digest, Sha256};

use crate::digest_json;

pub const MACHINE_ENROLLMENT_SCHEMA: &str = "cynic.machine_enrollment.v1";
pub const MACHINE_ENROLLMENT_AUTHORITY_CLASS: &str = "MachineEnrollment";

/// Machine-level enrollment record.
///
/// Emitted by the kernel once (offline bootstrap) and referenced by every
/// `OneShotCapability` whose `authority_class == "MachineEnrollment"`.
/// The host verifies `signature_hex` over the canonical signing payload
/// using `public_key.ed25519`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineEnrollment {
    pub schema: String,
    pub node_id: String,
    pub authority_class: String,
    /// Subject public key of the enrollment keypair (ed25519), hex-encoded.
    pub subject_public_key: String,
    pub principal: String,
    pub role: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    /// Opaque machine fingerprints (e.g. bios_uuid, chassis_asset_tag)
    /// used as evidence that the enrollment was performed on the same host.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    /// Detached ed25519 signature over `signing_payload()` by the parent
    /// enrollment authority (or self-signed at the root).
    pub signature_hex: String,
}

impl MachineEnrollment {
    /// Canonical payload signed by the enrollment authority.
    /// Mirrors the on-disk field set exactly.
    pub fn input_digest(&self) -> String {
        digest_json(&EnrollmentSigningPayload {
            schema: &self.schema,
            node_id: &self.node_id,
            authority_class: &self.authority_class,
            subject_public_key: &self.subject_public_key,
            principal: &self.principal,
            role: &self.role,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
            evidence_refs: &self.evidence_refs,
        })
        .expect("digest_json")
    }
    /// SHA-256 digest of the enrollment public key, suitable for use as
    /// `OneShotCapability.authority_key_id`.
    pub fn authority_key_id(&self) -> String {
        let digest = Sha256::digest(self.subject_public_key.as_bytes());
        format!("sha256:{}", hex::encode(digest))
    }
    pub fn node_subject(&self) -> String {
        format!("node:{}", self.node_id)
    }

    /// Pure structural validation.  Cryptography (signature proof)
    /// is verified by the injected `MachineEnrollmentVerifier` in the host.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != MACHINE_ENROLLMENT_SCHEMA {
            blockers.push("machine_enrollment_schema_invalid".to_string());
        }
        if self.authority_class != MACHINE_ENROLLMENT_AUTHORITY_CLASS {
            blockers.push("machine_enrollment_authority_class_invalid".to_string());
        }
        if self.node_id.trim().is_empty()
            || self.subject_public_key.trim().is_empty()
            || !is_valid_ed25519_hex(&self.subject_public_key)
        {
            blockers.push("machine_enrollment_identity_invalid".to_string());
        }
        if self.issued_at_unix_s == 0 || self.expires_at_unix_s <= self.issued_at_unix_s {
            blockers.push("machine_enrollment_lifetime_invalid".to_string());
        }
        if self.signature_hex.trim().is_empty() || !is_valid_signature_hex(&self.signature_hex) {
            blockers.push("machine_enrollment_signature_missing".to_string());
        }
        if self.evidence_refs.is_empty() {
            blockers.push("machine_enrollment_evidence_missing".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

#[derive(serde::Serialize)]
struct EnrollmentSigningPayload<'a> {
    schema: &'a str,
    node_id: &'a str,
    authority_class: &'a str,
    subject_public_key: &'a str,
    principal: &'a str,
    role: &'a str,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
    evidence_refs: &'a [String],
}

fn is_valid_ed25519_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn is_valid_signature_hex(value: &str) -> bool {
    value.len() == 128 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Pure check used by `OneShotCapability::verify_against_enrollment`.
pub fn enrollment_matches_subject(
    enrollment: &MachineEnrollment,
    capability_subject: &str,
) -> bool {
    capability_subject == enrollment.node_subject()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> MachineEnrollment {
        MachineEnrollment {
            schema: MACHINE_ENROLLMENT_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            authority_class: MACHINE_ENROLLMENT_AUTHORITY_CLASS.to_string(),
            subject_public_key: "a".repeat(64),
            principal: "cynic".to_string(),
            role: "build_runner".to_string(),
            issued_at_unix_s: 1_700_000_000,
            expires_at_unix_s: 1_700_003_600,
            evidence_refs: vec!["bios_uuid:abc".to_string()],
            signature_hex: "b".repeat(128),
        }
    }

    #[test]
    fn a_well_formed_enrollment_has_no_blockers() {
        assert!(valid().validate().is_empty());
    }

    #[test]
    fn validate_reports_every_violated_contract_at_once() {
        // Surfacing only the first blocker makes the operator repair one and hit
        // the next blind, so the count is locked here.
        let mut broken = valid();
        broken.schema = "cynic.wrong.v1".to_string();
        broken.authority_class = "HumanAdmin".to_string();
        broken.subject_public_key = "not-hex".to_string();
        broken.expires_at_unix_s = broken.issued_at_unix_s;
        broken.signature_hex = String::new();
        broken.evidence_refs.clear();

        let blockers = broken.validate();
        assert_eq!(blockers.len(), 6, "blockers: {blockers:?}");
        assert!(blockers.contains(&"machine_enrollment_schema_invalid".to_string()));
        assert!(blockers.contains(&"machine_enrollment_authority_class_invalid".to_string()));
        assert!(blockers.contains(&"machine_enrollment_identity_invalid".to_string()));
        assert!(blockers.contains(&"machine_enrollment_lifetime_invalid".to_string()));
        assert!(blockers.contains(&"machine_enrollment_signature_missing".to_string()));
        assert!(blockers.contains(&"machine_enrollment_evidence_missing".to_string()));
    }

    #[test]
    fn a_truncated_key_or_signature_is_rejected_not_accepted_as_hex() {
        let mut short_key = valid();
        short_key.subject_public_key = "ab".to_string();
        assert!(
            short_key
                .validate()
                .contains(&"machine_enrollment_identity_invalid".to_string())
        );

        let mut short_sig = valid();
        short_sig.signature_hex = "ab".to_string();
        assert!(
            short_sig
                .validate()
                .contains(&"machine_enrollment_signature_missing".to_string())
        );
    }

    #[test]
    fn expiry_must_be_strictly_after_issuance() {
        let mut backwards = valid();
        backwards.expires_at_unix_s = backwards.issued_at_unix_s - 1;
        assert!(
            backwards
                .validate()
                .contains(&"machine_enrollment_lifetime_invalid".to_string())
        );
    }

    #[test]
    fn subject_matching_requires_the_exact_node_prefix() {
        let enrollment = valid();
        assert!(enrollment_matches_subject(
            &enrollment,
            "node:cynic-builder"
        ));
        // A bare id, a different node, or a prefix must not authorize.
        assert!(!enrollment_matches_subject(&enrollment, "cynic-builder"));
        assert!(!enrollment_matches_subject(&enrollment, "node:cortex"));
        assert!(!enrollment_matches_subject(&enrollment, "node:cynic-build"));
    }

    #[test]
    fn signing_payload_digest_excludes_the_signature() {
        // The digest must be stable when only the signature changes, otherwise a
        // record could never be verified against what was signed.
        let base = valid();
        let mut resigned = base.clone();
        resigned.signature_hex = "c".repeat(128);
        assert_eq!(base.input_digest(), resigned.input_digest());

        // Any signed field must change it.
        let mut rerole = base.clone();
        rerole.role = "kernel_primary".to_string();
        assert_ne!(base.input_digest(), rerole.input_digest());
    }

    #[test]
    fn authority_key_id_is_a_prefixed_sha256_of_the_subject_key() {
        let id = valid().authority_key_id();
        assert!(id.starts_with("sha256:"));
        assert_eq!(id.len(), "sha256:".len() + 64);
    }
}
