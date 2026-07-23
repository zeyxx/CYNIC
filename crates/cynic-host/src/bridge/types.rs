use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command_surface::HostCommandSurface;
use crate::execution::HostExecutionPlan;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrivilegeBridgeInstallSpec {
    pub target: String,
    pub bundle_complete: bool,
    pub subject: String,
    pub authority_user: String,
    pub authority_group: String,
    pub provider: String,
    pub helper_source: String,
    pub helper_target: String,
    pub helper_mode: u32,
    pub projection_source: String,
    pub projection_target: String,
    pub projection_mode: u32,
    pub expected_projection: String,
    pub proof_invocation: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PrivilegeBridgeObservedState {
    pub subject_present: bool,
    pub provider_available: bool,
    pub helper_present: bool,
    pub helper_executable: bool,
    pub helper_mode_matches: bool,
    pub helper_authority_owned: bool,
    pub projection_present: bool,
    pub projection_mode_matches: bool,
    pub projection_authority_owned: bool,
    pub projection_valid: bool,
    pub invocation_policy_bounded: bool,
    pub bounded_helper_noninteractive: bool,
    pub unrestricted_shell_denied: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecBridgeBundleManifest {
    pub(crate) schema: &'static str,
    pub(crate) mutates: bool,
    pub(crate) helper_source: String,
    pub(crate) helper_target: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) prepare_directories: Vec<HostCommandSurface>,
    pub(crate) required_modes: BTreeMap<&'static str, String>,
    pub(crate) activation_checks: Vec<HostCommandSurface>,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_files: BTreeMap<&'static str, String>,
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
    pub(crate) bundle_manifest: HostExecBridgeBundleManifest,
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
    pub(crate) helper_path: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) helper_sha256: Option<String>,
    pub(crate) sudoers_sha256: Option<String>,
    pub(crate) checks: HostExecBridgeChecks,
    pub(crate) missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HostExecBridgeChecks {
    pub(crate) helper_exists: bool,
    pub(crate) helper_mode_0755: bool,
    pub(crate) helper_executable: bool,
    pub(crate) sudoers_exists: bool,
    pub(crate) sudoers_mode_0440: bool,
    pub(crate) sudoers_rule_matches: bool,
    pub(crate) sudoers_no_unbounded_helper: bool,
    pub(crate) sudoers_no_shell: bool,
    pub(crate) sudoers_no_raw_sudo: bool,
}
