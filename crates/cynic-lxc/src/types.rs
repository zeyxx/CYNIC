use serde::{Deserialize, Serialize};

use cynic_protocol::BreakglassAdmission;

pub const REQUEST_SCHEMA: &str = "cynic.lxc_exec_bridge.request.v1";
pub const RESPONSE_SCHEMA: &str = "cynic.lxc_exec_bridge.response.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LxcBridgeAuthority {
    RuntimeOperator,
    MachineEnrollment,
    RecoveryBreakglass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LxcBridgeRequest {
    pub schema: String,
    pub vmid: u64,
    pub purpose: String,
    pub agent_id: String,
    pub authority: LxcBridgeAuthority,
    /// Required only for RecoveryBreakglass authority. It is an inspectable
    /// admission record, never secret material.
    #[serde(default)]
    pub breakglass_admission: Option<BreakglassAdmission>,
    pub argv: Vec<String>,
    pub timeout_seconds: u64,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum LxcBridgeDecision {
    Observe { pct_argv: Vec<String> },
    Apply { pct_argv: Vec<String> },
    Blocked { blockers: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LxcBridgeResponse {
    pub schema: &'static str,
    pub status: String,
    pub vmid: u64,
    pub purpose: String,
    pub authority: LxcBridgeAuthority,
    pub mutates: bool,
    pub requested_apply: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub output_secret_classification: &'static str,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LxcBridgeInstallPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub helper_source: String,
    pub helper_target: String,
    pub sudoers_target: String,
    pub install_argv: Vec<Vec<String>>,
    pub proofs: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LxcBridgeBundleReport {
    pub schema: &'static str,
    pub status: String,
    pub output_dir: String,
    pub files: Vec<String>,
    pub helper_sha256: Option<String>,
    pub secret_exposed: bool,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LxcBridgeStatus {
    pub schema: &'static str,
    pub status: &'static str,
    pub helper_path: String,
    pub helper_present: bool,
    pub helper_executable: bool,
    pub helper_root_owned: Option<bool>,
    pub sudoers_path: String,
    pub sudoers_present: bool,
    pub sudoers_mode_0440: Option<bool>,
    pub sudoers_bounded: Option<bool>,
    pub blockers: Vec<String>,
}
