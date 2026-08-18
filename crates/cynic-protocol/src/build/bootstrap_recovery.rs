use serde::{Deserialize, Serialize};

pub const BOOTSTRAP_RECOVERY_ADMISSION_SCHEMA: &str = "cynic.build.bootstrap_recovery_admission.v1";

/// One-shot MachineEnrollment authority used only to restore the coordinator artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapRecoveryAdmission {
    pub schema: String,
    pub status: String,
    pub authority_class: String,
    pub receipt_id: String,
    pub node_id: String,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub mcp_artifact_digest: String,
    pub expires_at_unix_s: u64,
    pub allowed_actions: Vec<String>,
    pub consumption_proof: Option<String>,
    pub evidence_refs: Vec<String>,
}

impl BootstrapRecoveryAdmission {
    pub fn validate_for(
        &self,
        node_id: &str,
        source_commit: &str,
        action: &str,
        now: u64,
    ) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != BOOTSTRAP_RECOVERY_ADMISSION_SCHEMA
            || self.status != "admitted"
            || self.authority_class != "MachineEnrollment"
        {
            blockers.push("bootstrap_recovery_authority_invalid".into());
        }
        if self.receipt_id.is_empty() || self.consumption_proof.is_some() {
            blockers.push("bootstrap_recovery_receipt_unavailable".into());
        }
        if self.node_id != node_id || self.source_commit != source_commit {
            blockers.push("bootstrap_recovery_binding_mismatch".into());
        }
        if self.expires_at_unix_s == 0 || now >= self.expires_at_unix_s {
            blockers.push("bootstrap_recovery_expired".into());
        }
        if self.source_bundle_digest.is_empty() || self.mcp_artifact_digest.is_empty() {
            blockers.push("bootstrap_recovery_digest_missing".into());
        }
        if !matches!(action, "make:check" | "make:build-mcp")
            || !self.allowed_actions.iter().any(|allowed| allowed == action)
        {
            blockers.push("bootstrap_recovery_action_forbidden".into());
        }
        if self.evidence_refs.is_empty() {
            blockers.push("bootstrap_recovery_evidence_missing".into());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn admission() -> BootstrapRecoveryAdmission {
        BootstrapRecoveryAdmission {
            schema: BOOTSTRAP_RECOVERY_ADMISSION_SCHEMA.into(),
            status: "admitted".into(),
            authority_class: "MachineEnrollment".into(),
            receipt_id: "recovery-1".into(),
            node_id: "cynic-builder".into(),
            source_commit: "a".repeat(40),
            source_bundle_digest: format!("sha256:{}", "b".repeat(64)),
            mcp_artifact_digest: format!("sha256:{}", "c".repeat(64)),
            expires_at_unix_s: 100,
            allowed_actions: vec!["make:check".into(), "make:build-mcp".into()],
            consumption_proof: None,
            evidence_refs: vec!["machine-enrollment-read-back".into()],
        }
    }
    #[test]
    fn recovery_cannot_activate_or_be_reused() {
        let mut receipt = admission();
        assert!(
            receipt
                .validate_for("cynic-builder", &"a".repeat(40), "activate", 1)
                .contains(&"bootstrap_recovery_action_forbidden".to_string())
        );
        receipt.consumption_proof = Some("sha256:consumed".into());
        assert!(
            receipt
                .validate_for("cynic-builder", &"a".repeat(40), "make:check", 1)
                .contains(&"bootstrap_recovery_receipt_unavailable".to_string())
        );
    }
}
