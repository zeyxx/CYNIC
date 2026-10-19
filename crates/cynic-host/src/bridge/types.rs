use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::execution::HostExecutionPlan;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrivilegeBridgeInstallSpec {
    pub target: String,
    pub bundle_complete: bool,
    pub subject: String,
    pub authority_user: String,
    pub authority_group: String,
    pub provider: String,
    pub executor: String,
    pub membrane_source: String,
    pub membrane_target: String,
    pub membrane_mode: u32,
    pub expected_membrane: String,
    pub proof_invocation: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PrivilegeBridgeObservedState {
    pub subject_present: bool,
    pub provider_available: bool,
    pub executor_present: bool,
    pub executor_executable: bool,
    pub membrane_present: bool,
    pub membrane_mode_matches: bool,
    pub membrane_authority_owned: bool,
    pub membrane_valid: bool,
    pub invocation_policy_bounded: bool,
    pub membrane_probe_executed: bool,
    pub unrestricted_shell_denied: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecBridgeInstallReport {
    pub(crate) schema: &'static str,
    pub status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) requested_apply: bool,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_expected_complete: bool,
    pub(crate) execution_plan: HostExecutionPlan,
    pub(crate) executions: Vec<HostExecBridgeCommandExecution>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecBridgeRootlessSmokeReport {
    pub(crate) schema: &'static str,
    pub status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) requested_apply: bool,
    pub(crate) sandbox_root: String,
    pub(crate) bundle_manifest: PrivilegeMembraneBundleManifest,
    pub(crate) install_report: HostExecBridgeInstallReport,
    pub(crate) status_report: HostExecBridgeStatus,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HostExecBridgeCommandExecution {
    pub(crate) role: String,
    pub(crate) argv: Vec<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostExecBridgeStatus {
    pub(crate) schema: String,
    pub status: String,
    pub(crate) executor: String,
    pub(crate) membrane_target: String,
    pub(crate) admitted_prefix: String,
    pub(crate) executor_sha256: Option<String>,
    pub(crate) membrane_sha256: Option<String>,
    pub(crate) checks: HostExecBridgeChecks,
    pub(crate) missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HostExecBridgeChecks {
    pub(crate) executor_exists: bool,
    pub(crate) executor_executable: bool,
    pub(crate) membrane_exists: bool,
    pub(crate) membrane_mode_0440: bool,
    pub(crate) membrane_admits_executor: bool,
    pub(crate) membrane_no_unbounded_executor: bool,
    pub(crate) membrane_no_shell: bool,
    pub(crate) membrane_no_raw_sudo: bool,
}

/// Manifest of the staged privilege membrane: the catalog-derived sudoers
/// file the single `cynic` binary is admitted through. No helper binary is
/// staged: the executor is the deployed release binary itself.
#[derive(Debug, Clone, Serialize)]
pub struct PrivilegeMembraneBundleManifest {
    pub(crate) schema: &'static str,
    pub(crate) mutates: bool,
    pub(crate) binary: String,
    pub(crate) subject: String,
    pub(crate) sudoers_target: String,
    pub(crate) capability_ids: Vec<String>,
    pub(crate) rule_count: usize,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_files: BTreeMap<&'static str, String>,
}
