use cynic_host::registry::HostProfileResolution;
use serde::Serialize;
use serde_json::Value;

use crate::network_command_surface::NetworkCommandSurface;
use crate::network_diagnostic_surface::NetworkDiagnostic;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorActivationStatus {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) host_profile: &'static str,
    pub(crate) helper_path: String,
    pub(crate) policy_path: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_complete: bool,
    pub(crate) bundle_evidence_source: &'static str,
    pub(crate) next_actions: Vec<NetworkDiagnostic>,
    pub(crate) proof_requirements: Vec<NetworkActuatorProofRequirement>,
    pub(crate) root_authority: RootAuthorityCheck,
    pub(crate) checks: NetworkActuatorChecks,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) host_resolution: Option<HostProfileResolution>,
    pub(crate) host_substrate: Value,
    pub(crate) host_substrate_plan: Value,
    pub(crate) host_bootstrap: Value,
    pub(crate) host_bootstrap_plan: Value,
    pub(crate) missing: Vec<NetworkDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorChecks {
    pub(crate) helper_exists: bool,
    pub(crate) helper_mode_0755: bool,
    pub(crate) helper_executable: bool,
    pub(crate) policy_exists: bool,
    pub(crate) policy_mode_0644: bool,
    pub(crate) sudoers_exists: bool,
    pub(crate) sudoers_mode_0440: bool,
    pub(crate) sudoers_rule_matches: bool,
    pub(crate) sudoers_no_raw_ip: bool,
    pub(crate) sudoers_no_raw_wg: bool,
    pub(crate) sudoers_no_shell: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorProofRequirement {
    pub(crate) check: &'static str,
    pub(crate) commands: Vec<NetworkCommandSurface>,
    pub(crate) expected: &'static str,
    pub(crate) evidence_source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RootAuthorityCheck {
    pub(crate) command: String,
    pub(crate) sudo_noninteractive_available: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stderr: String,
}
