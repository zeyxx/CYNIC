//! Capability admission contract: cross-checks a `OneShotCapability` against
//! a `MachineEnrollment` record and a typed `Binding`.
//!
//! This is *pure protocol validation* — it never touches the crypto verifier
//! (which lives in `cynic-host`) or the filesystem.  It establishes the
//! structural invariants that any signed binding MUST satisfy before the host
//! attempts to consume the one-shot capability.

use cynic_node_protocol::machine_enrollment::{MACHINE_ENROLLMENT_AUTHORITY_CLASS, MachineEnrollment};
use cynic_protocol::one_shot_capability::{OneShotCapability, valid_sha256};

/// A single canonical contract unifying a capability + enrollment + binding.
///
/// `validate()` returns pure blockers; the host injects
/// `MachineEnrollmentSignatureVerifier` to check the actual ed25519 proof.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildRunnerBootstrapAdmissionContract {
    pub schema: String,
    pub node_id: String,
    pub enrollment: MachineEnrollment,
    pub capability: OneShotCapability,
    pub binding_digest: String,
}

pub const BUILD_RUNNER_BOOTSTRAP_ADMISSION_CONTRACT_SCHEMA: &str =
    "cynic.build_runner.bootstrap_admission_contract.v1";

impl BuildRunnerBootstrapAdmissionContract {
    /// Pure cross-validation: no crypto, no FS.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();

        if self.schema != BUILD_RUNNER_BOOTSTRAP_ADMISSION_CONTRACT_SCHEMA {
            blockers.push("contract_schema_invalid".to_string());
        }

        // 1. Enrollment identity must be non-blocked structurally.
        let enrollment_blockers = self.enrollment.validate();
        if !enrollment_blockers.is_empty() {
            blockers.push("contract_enrollment_invalid".to_string());
        }

        // 2. Capability subject must bind to the enrollment node_subject.
        if self.capability.subject != self.enrollment.node_subject() {
            blockers.push("contract_capability_subject_mismatch".to_string());
        }

        // 3. Capability authority_key_id must equal enrollment authority_key_id.
        if self.capability.authority_key_id != self.enrollment.authority_key_id() {
            blockers.push("contract_authority_key_id_mismatch".to_string());
        }

        // 4. Capability authority_class must be MachineEnrollment.
        if self.capability.authority_class != MACHINE_ENROLLMENT_AUTHORITY_CLASS {
            blockers.push("contract_authority_class_invalid".to_string());
        }

        // 5. Binding digest must be sha256.
        if !valid_sha256(&self.binding_digest) {
            blockers.push("contract_binding_digest_invalid".to_string());
        }

        // 6. Capability scope for build-runner bootstrap.
        if self.capability.scope != "build-runner-bootstrap" {
            blockers.push("contract_capability_scope_invalid".to_string());
        }

        // 7. Capability ID must match node_id (prefix `build_` + node_id).
        let expected_capability_id = format!("build_{}", self.node_id);
        if self.capability.capability_id != expected_capability_id {
            blockers.push("contract_capability_id_mismatch".to_string());
        }

        blockers.sort();
        blockers.dedup();
        blockers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_node_protocol::machine_enrollment::MachineEnrollment;
    use cynic_protocol::one_shot_capability::{ONE_SHOT_CAPABILITY_SCHEMA, OneShotCapability};

    fn enrollment(node_id: &str) -> MachineEnrollment {
        let pubkey = "0".repeat(64);
        MachineEnrollment {
            schema: "cynic.machine_enrollment.v1".to_string(),
            node_id: node_id.to_string(),
            authority_class: "MachineEnrollment".to_string(),
            subject_public_key: pubkey,
            principal: "cynic".to_string(),
            role: "build-runner".to_string(),
            issued_at_unix_s: 10,
            expires_at_unix_s: 2000,
            signature_hex: "f".repeat(128),
            evidence_refs: vec!["machine-enrollment:proof".to_string()],
        }
    }

    fn capability(enr: &MachineEnrollment) -> OneShotCapability {
        OneShotCapability {
            schema: ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "MachineEnrollment".to_string(),
            capability_id: format!("build_{}", enr.node_id),
            subject: enr.node_subject(),
            scope: "build-runner-bootstrap".to_string(),
            input_digest: format!("sha256:{}", "a".repeat(64)),
            executor_digest: format!("sha256:{}", "d".repeat(64)),
            issued_at_unix_s: 10,
            expires_at_unix_s: 2000,
            allowed_action_digests: vec![],
            authority_key_id: enr.authority_key_id(),
            signature_hex: "0".repeat(128),
            evidence_refs: vec!["enrollment:proof".to_string()],
            budget: None,
        }
    }

    #[test]
    fn contract_passes_when_all_fields_aligned() {
        let enr = enrollment("cynic-builder");
        let contract = BuildRunnerBootstrapAdmissionContract {
            schema: BUILD_RUNNER_BOOTSTRAP_ADMISSION_CONTRACT_SCHEMA.to_string(),
            node_id: enr.node_id.clone(),
            enrollment: enr.clone(),
            capability: capability(&enr),
            binding_digest: format!("sha256:{}", "b".repeat(64)),
        };
        assert!(contract.validate().is_empty());
    }

    #[test]
    fn contract_rejects_subject_mismatch() {
        let enr = enrollment("cynic-builder");
        let mut cap = capability(&enr);
        cap.subject = "node-key:sha256:WRONG".to_string();
        let contract = BuildRunnerBootstrapAdmissionContract {
            schema: BUILD_RUNNER_BOOTSTRAP_ADMISSION_CONTRACT_SCHEMA.to_string(),
            node_id: enr.node_id.clone(),
            enrollment: enr,
            capability: cap,
            binding_digest: format!("sha256:{}", "b".repeat(64)),
        };
        assert!(
            contract
                .validate()
                .iter()
                .any(|b| b == "contract_capability_subject_mismatch")
        );
    }

    #[test]
    fn contract_rejects_bad_binding_digest() {
        let enr = enrollment("cynic-builder");
        let contract = BuildRunnerBootstrapAdmissionContract {
            schema: BUILD_RUNNER_BOOTSTRAP_ADMISSION_CONTRACT_SCHEMA.to_string(),
            node_id: enr.node_id.clone(),
            enrollment: enr.clone(),
            capability: capability(&enr),
            binding_digest: "not-a-digest".to_string(),
        };
        assert!(
            contract
                .validate()
                .iter()
                .any(|b| b == "contract_binding_digest_invalid")
        );
    }
}
