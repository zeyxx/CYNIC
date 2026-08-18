use serde::{Deserialize, Serialize};

use crate::CynicErrorCode;

pub const BREAKGLASS_ADMISSION_SCHEMA: &str = "cynic.breakglass.admission.v1";

/// A time-bounded incident admission. It is deliberately separate from
/// bootstrap recovery, which is only for reconstructing an MCP artifact.
///
/// A durable incident-authority adapter must write `consumption_proof` after
/// use; this pure contract rejects a receipt already marked consumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakglassAdmission {
    pub schema: String,
    pub status: String,
    pub authority_class: String,
    pub receipt_id: String,
    pub target_scope: String,
    pub expires_at_unix_s: u64,
    pub allowed_actions: Vec<String>,
    pub consumption_proof: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// Stable identity of the incident authority that signed this admission.
    pub authority_key_id: String,
    /// Detached Ed25519 signature, encoded as hexadecimal. The signature binds
    /// every admission field except itself and local consumption state.
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct BreakglassSigningPayload<'a> {
    schema: &'a str,
    status: &'a str,
    authority_class: &'a str,
    receipt_id: &'a str,
    target_scope: &'a str,
    expires_at_unix_s: u64,
    allowed_actions: &'a [String],
    evidence_refs: &'a [String],
    authority_key_id: &'a str,
}

impl BreakglassAdmission {
    pub fn validate_for(
        &self,
        target_scope: &str,
        action: &str,
        now_unix_s: u64,
    ) -> Vec<CynicErrorCode> {
        let mut errors = Vec::new();
        if self.schema != BREAKGLASS_ADMISSION_SCHEMA
            || self.status != "admitted"
            || self.authority_class != "RecoveryBreakglass"
            || self.receipt_id.trim().is_empty()
            || self.target_scope != target_scope
            || self.evidence_refs.is_empty()
            || self.authority_key_id.trim().is_empty()
            || self.signature_hex.trim().is_empty()
        {
            errors.push(CynicErrorCode::BreakglassAdmissionInvalid);
        }
        if self.consumption_proof.is_some() {
            errors.push(CynicErrorCode::BreakglassReceiptConsumed);
        }
        if self.expires_at_unix_s == 0 || now_unix_s >= self.expires_at_unix_s {
            errors.push(CynicErrorCode::BreakglassAdmissionExpired);
        }
        if !self.allowed_actions.iter().any(|allowed| allowed == action) {
            errors.push(CynicErrorCode::BreakglassActionForbidden);
        }
        errors.sort_by_key(|error| error.code());
        errors.dedup();
        errors
    }

    /// Deterministic serialized payload used by the injected signature adapter.
    /// Consumption is intentionally excluded: it is local authority state, not
    /// an attacker-controlled amendment to the original admission.
    pub fn signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&BreakglassSigningPayload {
            schema: &self.schema,
            status: &self.status,
            authority_class: &self.authority_class,
            receipt_id: &self.receipt_id,
            target_scope: &self.target_scope,
            expires_at_unix_s: self.expires_at_unix_s,
            allowed_actions: &self.allowed_actions,
            evidence_refs: &self.evidence_refs,
            authority_key_id: &self.authority_key_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admission() -> BreakglassAdmission {
        BreakglassAdmission {
            schema: BREAKGLASS_ADMISSION_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "RecoveryBreakglass".to_string(),
            receipt_id: "incident-1".to_string(),
            target_scope: "lxc:105".to_string(),
            expires_at_unix_s: 100,
            allowed_actions: vec!["systemctl:restart:cynic-node.service".to_string()],
            consumption_proof: None,
            evidence_refs: vec!["incident:INC-1".to_string()],
            authority_key_id: "sha256:test".to_string(),
            signature_hex: "signature".to_string(),
        }
    }

    #[test]
    fn admission_rejects_a_marked_consumed_receipt_and_binds_scope_and_action() {
        let mut receipt = admission();
        assert!(
            receipt
                .validate_for("lxc:105", "systemctl:restart:cynic-node.service", 1)
                .is_empty()
        );
        receipt.consumption_proof = Some("proof".to_string());
        assert!(
            receipt
                .validate_for("lxc:105", "systemctl:restart:cynic-node.service", 1)
                .contains(&CynicErrorCode::BreakglassReceiptConsumed)
        );
        assert!(
            receipt
                .validate_for("lxc:106", "systemctl:restart:cynic-node.service", 1)
                .contains(&CynicErrorCode::BreakglassAdmissionInvalid)
        );
    }
}
