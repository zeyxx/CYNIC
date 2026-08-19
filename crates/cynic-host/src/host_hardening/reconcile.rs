use cynic_protocol::hardening::{
    HardeningAction, HardeningPolicyLevel, HostHardeningPlan, HOST_HARDENING_REPORT_SCHEMA_V1,
};

use super::observe::{observe_firewall_state, observe_sshd_state};
use super::types::SSHD_CYNIC_DROP_IN_PATH;

pub fn plan_host_hardening(
    policy_level: HardeningPolicyLevel,
    base_config_path: Option<&str>,
) -> HostHardeningPlan {
    let sshd_state = observe_sshd_state(base_config_path);
    let firewall_state = observe_firewall_state();

    let mut pending_actions = Vec::new();

    // 1. Règle SSH Password Authentication
    if sshd_state.password_auth_enabled {
        pending_actions.push(HardeningAction {
            id: "SSH-DISABLE-PASSWORD-AUTH".to_string(),
            target: SSHD_CYNIC_DROP_IN_PATH.to_string(),
            description: "Disable password authentication in sshd; enforce Ed25519/RSA keys only.".to_string(),
            requires_privilege: true,
            rollback_action: format!("rm -f {SSHD_CYNIC_DROP_IN_PATH} && systemctl reload sshd"),
        });
    }

    // 2. Règle SSH Root Password Login
    if sshd_state.permit_root_login == "yes" {
        pending_actions.push(HardeningAction {
            id: "SSH-PROHIBIT-ROOT-PASSWORD".to_string(),
            target: SSHD_CYNIC_DROP_IN_PATH.to_string(),
            description: "Set PermitRootLogin prohibit-password (keys only for root).".to_string(),
            requires_privilege: true,
            rollback_action: format!("rm -f {SSHD_CYNIC_DROP_IN_PATH} && systemctl reload sshd"),
        });
    }

    // 3. Règle Pare-feu Stealth Drop
    if !firewall_state.direct_wan_ssh_blocked && policy_level == HardeningPolicyLevel::StrictSovereign {
        pending_actions.push(HardeningAction {
            id: "FW-RESTRICT-SSH-TO-SYNAPSE".to_string(),
            target: "/etc/nftables.conf".to_string(),
            description: "Restrict SSH port 22 listening to synapse0 interface (10.10.10.0/24) and drop WAN probes.".to_string(),
            requires_privilege: true,
            rollback_action: "nft flush ruleset".to_string(),
        });
    }

    let is_converged = pending_actions.is_empty();

    HostHardeningPlan {
        schema: HOST_HARDENING_REPORT_SCHEMA_V1,
        policy_level,
        target_os: std::env::consts::OS.to_string(),
        service_manager: if cfg!(target_os = "linux") { "systemd".to_string() } else { "process".to_string() },
        firewall_backend: firewall_state.backend_detected.clone(),
        sshd_state,
        firewall_state,
        pending_actions,
        is_converged,
    }
}
