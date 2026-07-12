use std::collections::BTreeMap;

use serde::Serialize;

use crate::command_surface::HostCommandSurface;
use crate::execution::HostExecutionPlan;

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
pub struct HostExecBridgeActivationPlan {
    pub(crate) schema: &'static str,
    pub(crate) mutates: bool,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_expected_complete: bool,
    pub(crate) bundle_files: BTreeMap<&'static str, String>,
    pub(crate) preflight: Vec<HostCommandSurface>,
    pub(crate) install_commands: Vec<HostCommandSurface>,
    pub(crate) postflight: Vec<HostCommandSurface>,
    pub(crate) denial_checks: Vec<HostCommandSurface>,
    pub(crate) success_criteria: Vec<&'static str>,
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

#[derive(Debug, Clone, Serialize)]
pub struct HostExecBridgeStatus {
    pub(crate) schema: &'static str,
    pub status: &'static str,
    pub(crate) helper_path: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) checks: HostExecBridgeChecks,
    pub(crate) missing: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
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
