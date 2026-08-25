use serde::{Deserialize, Serialize};

pub const SSH_TRUST_STORE_SCHEMA: &str = "cynic.ssh_trust_store.v1";
pub const SSH_TRUST_VERIFICATION_SCHEMA: &str = "cynic.ssh_trust_verification.v1";
pub const SSH_TRUST_ADMISSION_SCHEMA: &str = "cynic.ssh_trust_admission.v1";
pub const SSH_TRUST_VERIFICATION_MAX_AGE_MS: u64 = 300_000;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SshTrustAnchor {
    pub target: String,
    pub algorithm: String,
    pub fingerprint: String,
    pub attestation_source: String,
    /// Unix milliseconds. Aligned on the protocol-wide `u64` millisecond
    /// contract (same unit as `SshTrustVerification.observed_at_unix_ms`);
    /// the former `u128` broke `verify_ssh_target_trust_at`'s signature.
    pub admitted_at_unix_ms: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SshTrustStore {
    pub schema: String,
    pub anchors: Vec<SshTrustAnchor>,
}

impl SshTrustStore {
    pub fn empty() -> Self {
        Self {
            schema: SSH_TRUST_STORE_SCHEMA.to_string(),
            anchors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshTrustVerification {
    pub schema: &'static str,
    pub target: String,
    pub status: String,
    pub anchor: Option<SshTrustAnchor>,
    pub observed_fingerprints: Vec<String>,
    pub observed_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub expires_at_unix_ms: u64,
    pub residual_risks: Vec<String>,
}

/// Read-back projection pairing a fresh network observation with the persisted
/// anchor decision. Discovery alone never grants trust.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshTrustInspectionReport {
    pub inspection: crate::transport::SshTargetInspectionReport,
    pub verification: SshTrustVerification,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshTrustAdmissionReport {
    pub schema: &'static str,
    pub target: String,
    pub status: String,
    pub mutates: bool,
    pub requested_apply: bool,
    pub verification: SshTrustVerification,
    pub anchor: Option<SshTrustAnchor>,
    pub blockers: Vec<String>,
}
