use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use chrono::Utc;
use serde::Serialize;

use crate::network_root_policy::build_root_policy;
use crate::observe::NodeObservation;

const NETWORK_ACTUATOR_POLICY_PATH: &str = "/etc/cynic/network-actuator.policy.json";
const NETWORK_ACTUATOR_HELPER_PATH: &str = "/usr/local/libexec/cynic-network-actuator";
const NETWORK_ACTUATOR_SUDOERS_PATH: &str = "/etc/sudoers.d/cynic-network-actuator";
const NETWORK_ACTUATOR_DEFAULT_BUNDLE_DIR: &str = "/tmp/cynic-network-actuator";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkActuatorActivationStatus {
    pub(crate) schema: &'static str,
    generated_at: String,
    pub(crate) status: &'static str,
    helper_path: &'static str,
    policy_path: &'static str,
    sudoers_target: &'static str,
    sudoers_rule: String,
    bundle_dir: &'static str,
    bundle_complete: bool,
    bundle_evidence_source: &'static str,
    next_actions: Vec<&'static str>,
    proof_requirements: Vec<NetworkActuatorProofRequirement>,
    root_authority: RootAuthorityCheck,
    checks: NetworkActuatorChecks,
    missing: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct NetworkActuatorChecks {
    helper_exists: bool,
    helper_mode_0755: bool,
    helper_executable: bool,
    policy_exists: bool,
    policy_mode_0644: bool,
    sudoers_exists: bool,
    sudoers_mode_0440: bool,
    sudoers_rule_matches: bool,
    sudoers_no_raw_ip: bool,
    sudoers_no_raw_wg: bool,
    sudoers_no_shell: bool,
}

#[derive(Debug, Clone, Serialize)]
struct NetworkActuatorProofRequirement {
    check: &'static str,
    command: &'static str,
    expected: &'static str,
    evidence_source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct RootAuthorityCheck {
    command: &'static str,
    sudo_noninteractive_available: bool,
    exit_code: Option<i32>,
    stderr: String,
}

pub(crate) fn activation_status(observation: &NodeObservation) -> NetworkActuatorActivationStatus {
    let root_policy = build_root_policy(observation);
    let sudoers_content = fs::read_to_string(NETWORK_ACTUATOR_SUDOERS_PATH).unwrap_or_default();
    let bundle_complete = bundle_complete(NETWORK_ACTUATOR_DEFAULT_BUNDLE_DIR);
    let root_authority = root_authority();

    let checks = NetworkActuatorChecks {
        helper_exists: path_exists(NETWORK_ACTUATOR_HELPER_PATH),
        helper_mode_0755: path_mode_equals(NETWORK_ACTUATOR_HELPER_PATH, 0o755),
        helper_executable: path_executable(NETWORK_ACTUATOR_HELPER_PATH),
        policy_exists: path_exists(NETWORK_ACTUATOR_POLICY_PATH),
        policy_mode_0644: path_mode_equals(NETWORK_ACTUATOR_POLICY_PATH, 0o644),
        sudoers_exists: path_exists(NETWORK_ACTUATOR_SUDOERS_PATH),
        sudoers_mode_0440: path_mode_equals(NETWORK_ACTUATOR_SUDOERS_PATH, 0o440),
        sudoers_rule_matches: sudoers_content.contains(&root_policy.sudoers_rule),
        sudoers_no_raw_ip: !sudoers_content.contains("/usr/sbin/ip")
            && !sudoers_content.contains("/usr/bin/ip"),
        sudoers_no_raw_wg: !sudoers_content.contains("/usr/bin/wg")
            && !sudoers_content.contains("/usr/bin/wg-quick"),
        sudoers_no_shell: !sudoers_content.contains("/bin/sh")
            && !sudoers_content.contains("/usr/bin/bash"),
    };

    let mut missing = Vec::new();
    if !checks.helper_exists {
        missing.push("helper_exists");
    }
    if !checks.helper_mode_0755 {
        missing.push("helper_mode_0755");
    }
    if !checks.helper_executable {
        missing.push("helper_executable");
    }
    if !checks.policy_exists {
        missing.push("policy_exists");
    }
    if !checks.policy_mode_0644 {
        missing.push("policy_mode_0644");
    }
    if !checks.sudoers_exists {
        missing.push("sudoers_exists");
    }
    if !checks.sudoers_mode_0440 {
        missing.push("sudoers_mode_0440");
    }
    if !checks.sudoers_rule_matches {
        missing.push("sudoers_rule_matches");
    }
    if !checks.sudoers_no_raw_ip {
        missing.push("sudoers_no_raw_ip");
    }
    if !checks.sudoers_no_raw_wg {
        missing.push("sudoers_no_raw_wg");
    }
    if !checks.sudoers_no_shell {
        missing.push("sudoers_no_shell");
    }
    if !bundle_complete {
        missing.push("bundle_complete");
    }

    NetworkActuatorActivationStatus {
        schema: "cynic.network_actuator.activation_status.v1",
        generated_at: Utc::now().to_rfc3339(),
        status: if missing.is_empty() {
            "active"
        } else {
            "missing_activation"
        },
        helper_path: NETWORK_ACTUATOR_HELPER_PATH,
        policy_path: NETWORK_ACTUATOR_POLICY_PATH,
        sudoers_target: NETWORK_ACTUATOR_SUDOERS_PATH,
        sudoers_rule: root_policy.sudoers_rule,
        bundle_dir: NETWORK_ACTUATOR_DEFAULT_BUNDLE_DIR,
        bundle_complete,
        bundle_evidence_source: "default_bundle_dir",
        root_authority,
        next_actions: next_actions(&missing),
        proof_requirements: proof_requirements(),
        checks,
        missing,
    }
}

fn path_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

fn path_mode_equals(path: impl AsRef<Path>, expected: u32) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o777 == expected)
        .unwrap_or(false)
}

fn path_executable(path: impl AsRef<Path>) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o100 != 0)
        .unwrap_or(false)
}

fn bundle_complete(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let required = [
        "manifest.json",
        "cynic-network-actuator",
        "network-actuator.policy.json",
        "cynic-network-actuator.sudoers",
    ];
    path.exists() && required.iter().all(|name| path.join(name).exists())
}

fn next_actions(missing: &[&'static str]) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if missing.contains(&"bundle_complete") {
        actions.push("build_network_actuator_bundle");
    }
    if [
        "helper_exists",
        "helper_mode_0755",
        "helper_executable",
        "policy_exists",
        "policy_mode_0644",
        "sudoers_exists",
        "sudoers_mode_0440",
        "sudoers_rule_matches",
    ]
    .iter()
    .any(|item| missing.contains(item))
    {
        actions.push("install_network_actuator_bundle_on_target_node");
    }
    if ["sudoers_no_raw_ip", "sudoers_no_raw_wg", "sudoers_no_shell"]
        .iter()
        .any(|item| missing.contains(item))
    {
        actions.push("prove_direct_network_commands_remain_denied");
    }
    actions
}

fn proof_requirements() -> Vec<NetworkActuatorProofRequirement> {
    vec![
        NetworkActuatorProofRequirement {
            check: "bundle_complete",
            command: "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator",
            expected: "bundle contains manifest.json, cynic-network-actuator, network-actuator.policy.json and cynic-network-actuator.sudoers",
            evidence_source: "default_bundle_dir",
        },
        NetworkActuatorProofRequirement {
            check: "helper_and_policy_installed",
            command: "cargo run -p cynic-node -- network actuator-status --json",
            expected: "helper, policy and sudoers are installed with expected modes",
            evidence_source: "local_filesystem",
        },
        NetworkActuatorProofRequirement {
            check: "direct_commands_denied",
            command: "sudo -n /usr/sbin/ip route show ; sudo -n /usr/bin/wg show",
            expected: "non-helper direct commands remain unauthorized for cynic",
            evidence_source: "operator_runtime_check",
        },
    ]
}

fn root_authority() -> RootAuthorityCheck {
    match Command::new("sudo").args(["-n", "true"]).output() {
        Ok(output) => RootAuthorityCheck {
            command: "sudo -n true",
            sudo_noninteractive_available: output.status.success(),
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(error) => RootAuthorityCheck {
            command: "sudo -n true",
            sudo_noninteractive_available: false,
            exit_code: None,
            stderr: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::node_observation::NodeRuntimeCapabilities;

    #[test]
    fn activation_status_reports_missing_when_files_are_absent() {
        let observation = NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );

        let status = activation_status(&observation);

        assert_eq!(status.schema, "cynic.network_actuator.activation_status.v1");
        assert_eq!(status.status, "missing_activation");
        assert!(status.missing.contains(&"helper_exists"));
        assert!(status.missing.contains(&"sudoers_exists"));
    }
}
