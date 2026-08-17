//! Signed, time-bounded capability intended for exactly one local mutation.
//!
//! Policy owners specialize this shape for bootstrap, incident recovery, or a
//! similarly exceptional boundary. The protocol deliberately owns no key,
//! filesystem, clock, or consumption store.

use serde::{Deserialize, Serialize};

pub const ONE_SHOT_CAPABILITY_SCHEMA: &str = "cynic.one_shot_capability.v1";
pub const ONE_SHOT_CAPABILITY_CONSUMPTION_SCHEMA: &str = "cynic.one_shot_capability_consumption.v1";
pub const ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA: &str = "cynic.one_shot_capability_outcome.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneShotCapability {
    pub schema: String,
    pub status: String,
    pub authority_class: String,
    pub capability_id: String,
    pub subject: String,
    pub scope: String,
    pub input_digest: String,
    pub executor_digest: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub allowed_action_digests: Vec<String>,
    pub authority_key_id: String,
    pub signature_hex: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneShotCapabilityConsumption {
    pub schema: String,
    pub capability_id: String,
    pub subject: String,
    pub scope: String,
    pub action_digest: String,
    pub consumed_at_unix_s: u64,
    pub authority_key_id: String,
    pub capability_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OneShotCapabilityOutcomeStatus {
    Succeeded,
    Failed,
    UnknownAfterCrash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneShotCapabilityOutcome {
    pub schema: String,
    pub capability_id: String,
    pub consumption_digest: String,
    pub status: OneShotCapabilityOutcomeStatus,
    pub recorded_at_unix_s: u64,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct OneShotCapabilitySigningPayload<'a> {
    schema: &'a str,
    status: &'a str,
    authority_class: &'a str,
    capability_id: &'a str,
    subject: &'a str,
    scope: &'a str,
    input_digest: &'a str,
    executor_digest: &'a str,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
    allowed_action_digests: &'a [String],
    authority_key_id: &'a str,
    evidence_refs: &'a [String],
}

impl OneShotCapability {
    /// Canonical signed content; a local consumption record is intentionally
    /// separate so that use cannot alter the issuer's original statement.
    pub fn signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        crate::canonical_json_bytes(&OneShotCapabilitySigningPayload {
            schema: &self.schema,
            status: &self.status,
            authority_class: &self.authority_class,
            capability_id: &self.capability_id,
            subject: &self.subject,
            scope: &self.scope,
            input_digest: &self.input_digest,
            executor_digest: &self.executor_digest,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
            allowed_action_digests: &self.allowed_action_digests,
            authority_key_id: &self.authority_key_id,
            evidence_refs: &self.evidence_refs,
        })
    }

    /// Validates pure claims before an injected verifier checks the detached
    /// signature and an injected store atomically claims the capability.
    pub fn validate_for(
        &self,
        subject: &str,
        scope: &str,
        action_digest: &str,
        now_unix_s: u64,
    ) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != ONE_SHOT_CAPABILITY_SCHEMA
            || self.status != "admitted"
            || self.authority_class.trim().is_empty()
            || !valid_identifier(&self.capability_id)
            || self.subject != subject
            || self.scope != scope
            || !valid_sha256(&self.input_digest)
            || !valid_sha256(&self.executor_digest)
            || !valid_sha256(&self.authority_key_id)
            || !valid_signature(&self.signature_hex)
            || self.evidence_refs.is_empty()
        {
            blockers.push("one_shot_capability_claim_invalid".to_string());
        }
        if self.issued_at_unix_s == 0
            || self.expires_at_unix_s <= self.issued_at_unix_s
            || now_unix_s < self.issued_at_unix_s
            || now_unix_s >= self.expires_at_unix_s
        {
            blockers.push("one_shot_capability_expired_or_invalid".to_string());
        }
        if !valid_sha256(action_digest)
            || !self
                .allowed_action_digests
                .iter()
                .any(|allowed| allowed == action_digest)
        {
            blockers.push("one_shot_capability_action_forbidden".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn valid_signature(value: &str) -> bool {
    value.len() == 128 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability() -> OneShotCapability {
        OneShotCapability {
            schema: ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "MachineEnrollment".to_string(),
            capability_id: "bootstrap_build_1".to_string(),
            subject: "node-key:sha256:subject".to_string(),
            scope: "build-runner-bootstrap".to_string(),
            input_digest: format!("sha256:{}", "a".repeat(64)),
            executor_digest: format!("sha256:{}", "b".repeat(64)),
            issued_at_unix_s: 10,
            expires_at_unix_s: 20,
            allowed_action_digests: vec![format!("sha256:{}", "c".repeat(64))],
            authority_key_id: format!("sha256:{}", "d".repeat(64)),
            signature_hex: "e".repeat(128),
            evidence_refs: vec!["machine-enrollment:proof".to_string()],
        }
    }

    #[test]
    fn capability_binds_subject_scope_action_and_time() {
        let capability = capability();
        assert!(
            capability
                .validate_for(
                    "node-key:sha256:subject",
                    "build-runner-bootstrap",
                    &format!("sha256:{}", "c".repeat(64)),
                    11,
                )
                .is_empty()
        );
        assert!(
            capability
                .validate_for(
                    "other-node",
                    "build-runner-bootstrap",
                    &format!("sha256:{}", "c".repeat(64)),
                    11,
                )
                .contains(&"one_shot_capability_claim_invalid".to_string())
        );
        assert!(
            capability
                .validate_for(
                    "node-key:sha256:subject",
                    "build-runner-bootstrap",
                    &format!("sha256:{}", "c".repeat(64)),
                    20,
                )
                .contains(&"one_shot_capability_expired_or_invalid".to_string())
        );
    }
}
