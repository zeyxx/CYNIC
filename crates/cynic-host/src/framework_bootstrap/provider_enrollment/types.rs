use std::path::PathBuf;

use cynic_protocol::provider::ProviderControlDesiredState;
use serde::Serialize;

use crate::runtime_secret_install::{RuntimeSecretInstallReport, RuntimeSecretInstallSpec};

pub const PROVIDER_CONTROL_ENROLLMENT_PLAN_SCHEMA: &str =
    "cynic.provider_control.enrollment_plan.v1";
pub const PROVIDER_CONTROL_ENROLLMENT_REPORT_SCHEMA: &str =
    "cynic.provider_control.enrollment_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderControlEnrollmentSpec {
    pub desired: ProviderControlDesiredState,
    pub token_secret_source: PathBuf,
    pub ca_source: Option<PathBuf>,
    pub config_target: PathBuf,
    pub ca_target: PathBuf,
    pub secret: RuntimeSecretInstallSpec,
    pub owner_id: u32,
    pub group_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderControlEnrollmentObservation {
    pub schema: &'static str,
    pub provider: String,
    pub config_path: String,
    pub config_present: bool,
    pub config_regular_file: Option<bool>,
    pub config_mode: Option<u32>,
    pub config_owner_id: Option<u32>,
    pub config_group_id: Option<u32>,
    pub endpoint_matches: Option<bool>,
    pub token_identity_matches: Option<bool>,
    pub token_secret_path_matches: Option<bool>,
    pub ca_path_matches: Option<bool>,
    pub secret_source_present: bool,
    pub secret_source_admissible: bool,
    pub secret_target_present: bool,
    pub secret_target_admissible: bool,
    pub ca_source_present: Option<bool>,
    pub ca_source_admissible: Option<bool>,
    pub ca_target_present: bool,
    pub ca_target_regular_file: Option<bool>,
    pub ca_target_mode: Option<u32>,
    pub ca_target_owner_id: Option<u32>,
    pub ca_target_group_id: Option<u32>,
    pub ca_digest_matches: Option<bool>,
    pub secret_exposed: bool,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderControlEnrollmentAction {
    InstallRuntimeMachineSecret,
    InstallProviderCa,
    InstallProviderConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderControlEnrollmentPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub provider: String,
    pub endpoint: String,
    pub token_identity: String,
    pub actions: Vec<ProviderControlEnrollmentAction>,
    pub blockers: Vec<String>,
    pub required_proofs: Vec<&'static str>,
    pub rollback_intent: &'static str,
    pub secret_exposed: bool,
}

#[derive(Debug, Serialize)]
pub struct ProviderControlEnrollmentReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub provider: String,
    pub before: ProviderControlEnrollmentObservation,
    pub after: ProviderControlEnrollmentObservation,
    pub secret_install: Option<RuntimeSecretInstallReport>,
    pub proofs: Vec<&'static str>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub rollback_performed: bool,
    pub secret_exposed: bool,
}
