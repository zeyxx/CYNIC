use serde::{Deserialize, Serialize};

pub const HOST_HARDENING_REPORT_SCHEMA_V1: &str = "cynic.host_hardening_report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardeningPolicyLevel {
    StrictSovereign,
    Standard,
    PermissiveLab,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshdObservedState {
    pub config_file_path: String,
    pub password_auth_enabled: bool,
    pub permit_root_login: String,
    pub pubkey_auth_enabled: bool,
    pub listen_addresses: Vec<String>,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirewallObservedState {
    pub backend_detected: String, // nftables, iptables, windows_firewall, none
    pub default_input_policy: String, // drop, accept, unknown
    pub wireguard_port_allowed: bool,
    pub direct_wan_ssh_blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardeningAction {
    pub id: String,
    pub target: String,
    pub description: String,
    pub requires_privilege: bool,
    pub rollback_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostHardeningPlan {
    pub schema: &'static str,
    pub policy_level: HardeningPolicyLevel,
    pub target_os: String,
    pub service_manager: String,
    pub firewall_backend: String,
    pub sshd_state: SshdObservedState,
    pub firewall_state: FirewallObservedState,
    pub pending_actions: Vec<HardeningAction>,
    pub is_converged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostHardeningProof {
    pub schema: &'static str,
    pub status: String,
    pub timestamp: String,
    pub actions_applied: usize,
    pub verified_password_auth_disabled: bool,
    pub verified_firewall_drop_active: bool,
    pub verified_ssh_confined_to_mesh: bool,
    pub residual_unknowns: Vec<String>,
}
