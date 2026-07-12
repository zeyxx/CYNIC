use std::collections::BTreeMap;

use serde::Serialize;

use crate::network_command_surface::NetworkCommandSurface;
use crate::network_diagnostic_surface::NetworkDiagnostic;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorPolicyDocument {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) helper_path: String,
    pub(crate) sudoers_subject: &'static str,
    pub(crate) max_timeout_seconds: u64,
    pub(crate) allowed_profiles: BTreeMap<String, NetworkActuatorAllowedProfile>,
    pub(crate) forbidden_tokens: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct NetworkActuatorAllowedProfile {
    pub(super) description: &'static str,
    pub(super) command_prefixes: Vec<NetworkCommandSurface>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorBundleManifest {
    pub(crate) schema: &'static str,
    pub(crate) mutates: bool,
    pub(crate) helper_source: String,
    pub(crate) helper_target: String,
    pub(crate) policy_source: String,
    pub(crate) policy_target: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) prepare_directories: Vec<NetworkCommandSurface>,
    pub(crate) required_owner: String,
    pub(crate) required_modes: BTreeMap<&'static str, String>,
    pub(crate) activation_checks: Vec<NetworkCommandSurface>,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_files: BTreeMap<&'static str, String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorActivationPlan {
    pub(crate) schema: &'static str,
    pub(crate) mutates: bool,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_expected_complete: bool,
    pub(crate) bundle_files: BTreeMap<&'static str, String>,
    pub(crate) preflight: Vec<NetworkCommandSurface>,
    pub(crate) install_commands: Vec<NetworkCommandSurface>,
    pub(crate) postflight: Vec<NetworkCommandSurface>,
    pub(crate) denial_checks: Vec<NetworkCommandSurface>,
    pub(crate) success_criteria: Vec<NetworkDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorApplyReport {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) profile: String,
    pub(crate) apply: bool,
    pub(crate) helper_contract: &'static str,
    pub(crate) commands: Vec<NetworkCommandSurface>,
    pub(crate) executions: Vec<CommandExecution>,
    pub(crate) violations: Vec<NetworkDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CommandExecution {
    pub(crate) role: String,
    pub(crate) argv: Vec<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}
