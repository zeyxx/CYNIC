mod proofs;
pub(crate) mod types;

use std::fs;

use chrono::Utc;
use cynic_host::bounded_helper::bundle_complete as helper_bundle_complete;
use cynic_host::filesystem_observation::{path_executable, path_exists, path_mode_equals};
use cynic_host::install_contract::{MODE_0440, MODE_0644, MODE_0755};
use cynic_host::runtime_layout::current_runtime_layout;
use serde_json::Value;

use crate::network_actuator_contract::{
    bundle_required_files, helper_path, policy_path, sudoers_path,
};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};
use crate::network_host_substrate;
use crate::network_root_policy::build_root_policy;
use crate::observe::NodeObservation;

use self::proofs::{proof_requirements, root_authority};
pub(crate) use self::types::NetworkActuatorActivationStatus;
use self::types::{NetworkActuatorChecks, RootAuthorityCheck};

#[derive(Debug, Clone)]
struct NetworkActuatorStatusContext {
    helper_path: String,
    policy_path: String,
    sudoers_path: String,
    bundle_dir: String,
    route_command: String,
    wireguard_command: String,
    denied_shell_bin: String,
    denied_bash_bin: String,
}

impl NetworkActuatorStatusContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            helper_path: helper_path(),
            policy_path: policy_path(),
            sudoers_path: sudoers_path(),
            bundle_dir: runtime_layout.network_actuator_bundle_dir,
            route_command: capabilities.route_tool.command,
            wireguard_command: capabilities.wireguard_tool.command,
            denied_shell_bin: "/bin/sh".to_string(),
            denied_bash_bin: "/usr/bin/bash".to_string(),
        }
    }
}

pub(crate) fn activation_status(observation: &NodeObservation) -> NetworkActuatorActivationStatus {
    let root_policy = build_root_policy(observation);
    let context = NetworkActuatorStatusContext::from_current_runtime();
    let sudoers_content = fs::read_to_string(&context.sudoers_path).unwrap_or_default();
    let bundle_complete = helper_bundle_complete(&context.bundle_dir, &bundle_required_files());
    let root_authority = root_authority();
    let host_context = network_host_substrate::observe_network_host_runtime_context();
    activation_status_with_host_context(
        root_policy,
        context,
        sudoers_content,
        bundle_complete,
        root_authority,
        host_context.host_profile,
        host_context.host_resolution.clone(),
        serialize_value(&host_context.host_substrate),
        serialize_value(&host_context.host_substrate_plan),
        serialize_value(&host_context.host_bootstrap),
        serialize_value(&host_context.host_bootstrap_plan),
        network_host_substrate::blocked_domains(&host_context),
        network_host_substrate::bootstrap_blocked(&host_context),
    )
}

fn activation_status_with_host_context(
    root_policy: crate::network_root_policy::NetworkRootPolicy,
    context: NetworkActuatorStatusContext,
    sudoers_content: String,
    bundle_complete: bool,
    root_authority: RootAuthorityCheck,
    host_profile: &'static str,
    host_resolution: Option<cynic_host::registry::HostProfileResolution>,
    host_substrate: Value,
    host_substrate_plan: Value,
    host_bootstrap: Value,
    host_bootstrap_plan: Value,
    blocked_domains: Vec<&'static str>,
    bootstrap_blocked: bool,
) -> NetworkActuatorActivationStatus {
    let checks = NetworkActuatorChecks {
        helper_exists: path_exists(&context.helper_path),
        helper_mode_0755: path_mode_equals(&context.helper_path, MODE_0755.unix),
        helper_executable: path_executable(&context.helper_path),
        policy_exists: path_exists(&context.policy_path),
        policy_mode_0644: path_mode_equals(&context.policy_path, MODE_0644.unix),
        sudoers_exists: path_exists(&context.sudoers_path),
        sudoers_mode_0440: path_mode_equals(&context.sudoers_path, MODE_0440.unix),
        sudoers_rule_matches: sudoers_content.contains(&root_policy.sudoers_rule),
        sudoers_no_raw_ip: forbidden_pattern_blocked(
            &root_policy,
            &sudoers_content,
            &context.route_command,
        ),
        sudoers_no_raw_wg: forbidden_pattern_blocked(
            &root_policy,
            &sudoers_content,
            &context.wireguard_command,
        ) && forbidden_pattern_blocked(
            &root_policy,
            &sudoers_content,
            "/usr/bin/wg-quick",
        ),
        sudoers_no_shell: forbidden_pattern_blocked(
            &root_policy,
            &sudoers_content,
            &context.denied_shell_bin,
        ) && forbidden_pattern_blocked(
            &root_policy,
            &sudoers_content,
            &context.denied_bash_bin,
        ),
    };

    let mut missing = filesystem_and_sudoers_diagnostics(&checks, bundle_complete);
    for blocker in blocked_domains {
        missing.push(host_substrate_diagnostic(blocker));
    }
    if bootstrap_blocked {
        missing.push(diagnostic(
            "host_bootstrap_blocked",
            "bootstrap",
            "host bootstrap chain is blocked",
        ));
    }

    NetworkActuatorActivationStatus {
        schema: "cynic.network_actuator.activation_status.v1",
        generated_at: Utc::now().to_rfc3339(),
        status: if missing.is_empty() {
            "active"
        } else {
            "missing_activation"
        },
        host_profile,
        helper_path: context.helper_path,
        policy_path: context.policy_path,
        sudoers_target: context.sudoers_path,
        sudoers_rule: root_policy.sudoers_rule.clone(),
        bundle_dir: context.bundle_dir,
        bundle_complete,
        bundle_evidence_source: "default_bundle_dir",
        root_authority,
        next_actions: next_actions(&missing),
        proof_requirements: proof_requirements(&root_policy),
        checks,
        host_resolution,
        host_substrate,
        host_substrate_plan,
        host_bootstrap,
        host_bootstrap_plan,
        missing,
    }
}

fn filesystem_and_sudoers_diagnostics(
    checks: &NetworkActuatorChecks,
    bundle_complete: bool,
) -> Vec<NetworkDiagnostic> {
    let mut missing = Vec::new();
    if !checks.helper_exists {
        missing.push(diagnostic(
            "helper_exists",
            "filesystem",
            "network actuator helper is missing",
        ));
    }
    if !checks.helper_mode_0755 {
        missing.push(diagnostic(
            "helper_mode_0755",
            "filesystem",
            "network actuator helper mode is not 0755",
        ));
    }
    if !checks.helper_executable {
        missing.push(diagnostic(
            "helper_executable",
            "filesystem",
            "network actuator helper is not executable",
        ));
    }
    if !checks.policy_exists {
        missing.push(diagnostic(
            "policy_exists",
            "filesystem",
            "network actuator policy file is missing",
        ));
    }
    if !checks.policy_mode_0644 {
        missing.push(diagnostic(
            "policy_mode_0644",
            "filesystem",
            "network actuator policy mode is not 0644",
        ));
    }
    if !checks.sudoers_exists {
        missing.push(diagnostic(
            "sudoers_exists",
            "filesystem",
            "network actuator sudoers projection is missing",
        ));
    }
    if !checks.sudoers_mode_0440 {
        missing.push(diagnostic(
            "sudoers_mode_0440",
            "filesystem",
            "network actuator sudoers mode is not 0440",
        ));
    }
    if !checks.sudoers_rule_matches {
        missing.push(diagnostic(
            "sudoers_rule_matches",
            "sudoers",
            "sudoers rule does not match the expected actuator rule",
        ));
    }
    if !checks.sudoers_no_raw_ip {
        missing.push(diagnostic(
            "sudoers_no_raw_ip",
            "sudoers",
            "raw route tool access is still exposed in sudoers",
        ));
    }
    if !checks.sudoers_no_raw_wg {
        missing.push(diagnostic(
            "sudoers_no_raw_wg",
            "sudoers",
            "raw wireguard access is still exposed in sudoers",
        ));
    }
    if !checks.sudoers_no_shell {
        missing.push(diagnostic(
            "sudoers_no_shell",
            "sudoers",
            "shell execution is still exposed in sudoers",
        ));
    }
    if !bundle_complete {
        missing.push(diagnostic(
            "bundle_complete",
            "bundle",
            "network actuator bundle is incomplete",
        ));
    }
    missing
}

fn next_actions(missing: &[NetworkDiagnostic]) -> Vec<NetworkDiagnostic> {
    let mut actions = Vec::new();
    if has_diagnostic(missing, "bundle_complete") {
        actions.push(diagnostic(
            "build_network_actuator_bundle",
            "action",
            "build or regenerate the network actuator bundle",
        ));
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
    .any(|item| has_diagnostic(missing, item))
    {
        actions.push(diagnostic(
            "install_network_actuator_bundle_on_target_node",
            "action",
            "install helper, policy and sudoers projection on the target node",
        ));
    }
    if ["sudoers_no_raw_ip", "sudoers_no_raw_wg", "sudoers_no_shell"]
        .iter()
        .any(|item| has_diagnostic(missing, item))
    {
        actions.push(diagnostic(
            "prove_direct_network_commands_remain_denied",
            "action",
            "capture proof that direct route, wireguard and shell commands remain denied",
        ));
    }
    if [
        "host_principal_substrate_blocked",
        "host_privilege_substrate_blocked",
        "host_package_substrate_blocked",
    ]
    .iter()
    .any(|item| has_diagnostic(missing, item))
    {
        actions.push(diagnostic(
            "repair_host_substrate_baseline",
            "action",
            "repair host substrate baseline before retrying actuator activation",
        ));
    }
    if has_diagnostic(missing, "host_bootstrap_blocked") {
        actions.push(diagnostic(
            "install_host_exec_bridge_once",
            "action",
            "install the host exec bridge once so bootstrap can complete",
        ));
    }
    actions
}

fn has_diagnostic(missing: &[NetworkDiagnostic], code: &str) -> bool {
    missing.iter().any(|item| item.code == code)
}

fn forbidden_pattern_blocked(
    root_policy: &crate::network_root_policy::NetworkRootPolicy,
    sudoers_content: &str,
    pattern: &str,
) -> bool {
    if root_policy
        .forbidden_invocation_patterns
        .iter()
        .any(|item| item.pattern == pattern)
    {
        !sudoers_content.contains(pattern)
    } else {
        true
    }
}

fn host_substrate_diagnostic(code: &'static str) -> NetworkDiagnostic {
    match code {
        "host_principal_substrate_blocked" => diagnostic(
            code,
            "host_substrate",
            "host principal substrate is blocked",
        ),
        "host_privilege_substrate_blocked" => diagnostic(
            code,
            "host_substrate",
            "host privilege substrate is blocked",
        ),
        "host_package_substrate_blocked" => {
            diagnostic(code, "host_substrate", "host package substrate is blocked")
        }
        "host_service_substrate_blocked" => {
            diagnostic(code, "host_substrate", "host service substrate is blocked")
        }
        other => diagnostic(other, "host_substrate", "host substrate blocker is present"),
    }
}

fn serialize_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value)
        .unwrap_or_else(|error| serde_json::json!({ "serialization_error": error.to_string() }))
}

#[cfg(test)]
mod tests {
    use super::{NetworkActuatorStatusContext, activation_status_with_host_context};
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_actuator_status::types::RootAuthorityCheck;
    use crate::network_root_policy::build_root_policy;
    use crate::observe::NodeObservation;

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
        let root_policy = build_root_policy(&observation);

        let status = activation_status_with_host_context(
            root_policy,
            NetworkActuatorStatusContext {
                helper_path: "/missing/helper".to_string(),
                policy_path: "/missing/policy".to_string(),
                sudoers_path: "/missing/sudoers".to_string(),
                bundle_dir: "/missing/bundle".to_string(),
                route_command: "/portable/bin/ip".to_string(),
                wireguard_command: "/portable/bin/wg".to_string(),
                denied_shell_bin: "/portable/bin/sh".to_string(),
                denied_bash_bin: "/portable/bin/bash".to_string(),
            },
            String::new(),
            false,
            RootAuthorityCheck {
                command: "sudo -n true".to_string(),
                sudo_noninteractive_available: false,
                exit_code: Some(1),
                stderr: "password required".to_string(),
            },
            "test-host",
            None,
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({}),
            Vec::new(),
            false,
        );

        assert_eq!(status.schema, "cynic.network_actuator.activation_status.v1");
        assert_eq!(status.status, "missing_activation");
        assert!(
            status
                .missing
                .iter()
                .any(|item| item.code == "helper_exists")
        );
        assert!(
            status
                .missing
                .iter()
                .any(|item| item.code == "sudoers_exists")
        );
    }
}
