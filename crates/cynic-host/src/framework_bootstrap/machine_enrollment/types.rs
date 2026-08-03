use cynic_protocol::{MachineEnrollmentAuthority, MachineEnrollmentAuthorityPolicy};
use serde::Serialize;

pub const MACHINE_ENROLLMENT_AUTHORITY_PLAN_SCHEMA: &str =
    "cynic.machine_enrollment.authority_plan.v1";
pub const MACHINE_ENROLLMENT_AUTHORITY_REPORT_SCHEMA: &str =
    "cynic.machine_enrollment.authority_apply_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineEnrollmentAuthoritySpec {
    pub authority_role: String,
    pub endpoint_service: String,
    pub node_id: String,
    pub seed_id: String,
    pub role: String,
    pub requested_capabilities: Vec<String>,
    pub policy: MachineEnrollmentAuthorityPolicy,
    pub config_root: String,
    pub staging_root: String,
    pub owner_id: u32,
    pub group_id: u32,
    pub now_unix_s: u64,
}

impl MachineEnrollmentAuthoritySpec {
    pub fn authority_path(&self) -> String {
        format!("{}/machine-enrollment-authority.json", self.config_root)
    }

    pub fn signing_key_path(&self) -> String {
        format!("{}/machine-enrollment-signing-key", self.config_root)
    }

    pub fn staged_public_key_path(&self) -> String {
        format!("{}/enrollment-authority.pub", self.staging_root)
    }

    pub fn staged_token_path(&self) -> String {
        format!("{}/enrollment-token", self.staging_root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineEnrollmentAuthorityObservation {
    pub schema: &'static str,
    pub authority_present: bool,
    pub authority_valid: Option<bool>,
    pub authority_id: Option<String>,
    pub seed_id: Option<String>,
    pub token_digest: Option<String>,
    pub signing_key_present: bool,
    pub signing_key_valid: Option<bool>,
    pub signing_key_mode: Option<u32>,
    pub signing_key_owner_id: Option<u32>,
    pub signing_key_group_id: Option<u32>,
    pub authority_key_id: Option<String>,
    pub staged_public_key_matches: Option<bool>,
    pub staged_token_matches: Option<bool>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineEnrollmentAuthorityAction {
    CreateSigningKey,
    RepairSigningKeyProjection,
    StageAuthorityPublicKey,
    StageOneTimeToken,
    InstallEnrollmentAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineEnrollmentAuthorityPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub authority_role: String,
    pub endpoint_service: String,
    pub node_id: String,
    pub seed_id: String,
    pub role: String,
    pub requested_capabilities: Vec<String>,
    pub actions: Vec<MachineEnrollmentAuthorityAction>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub required_proofs: Vec<&'static str>,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineEnrollmentAuthorityApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub plan: MachineEnrollmentAuthorityPlan,
    pub before: MachineEnrollmentAuthorityObservation,
    pub after: MachineEnrollmentAuthorityObservation,
    pub authority: Option<MachineEnrollmentAuthority>,
    pub proofs_observed: Vec<&'static str>,
    pub blockers: Vec<String>,
    pub rollback_performed: bool,
    pub secret_material_returned: bool,
}
