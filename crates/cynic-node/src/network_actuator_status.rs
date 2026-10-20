mod proofs;
pub(crate) mod types;

use std::fs;
use std::path::Path;

use chrono::Utc;
use cynic_host::filesystem_observation::{path_executable, path_exists, path_mode_equals};
use cynic_host::install_contract::{MODE_0440, MODE_0644};
use cynic_host::runtime_layout::current_runtime_layout;
use serde_json::Value;

use crate::network_actuator_contract::{
    bundle_required_files, executor_path, membrane_admission, membrane_path, policy_path,
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
    executor: String,
    policy_path: String,
    membrane_target: String,
    membrane_admission: String,
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
            executor: executor_path(),
            policy_path: policy_path(),
            membrane_target: membrane_path(),
            membrane_admission: membrane_admission(),
            bundle_dir: runtime_layout.network_actuator_bundle_dir,
            route_command: capabilities.route_tool.command,
            wireguard_command: capabilities.wireguard_tool.command,
            denied_shell_bin: "/bin/sh".to_string(),
            denied_bash_bin: "/usr/bin/bash".to_string(),
        }
    }
}

pub fn activation_status(observation: &NodeObservation) -> NetworkActuatorActivationStatus {
    let root_policy = build_root_policy(observation);
    let context = NetworkActuatorStatusContext::from_current_runtime();
    let membrane_content = fs::read_to_string(&context.membrane_target).unwrap_or_default();
    let bundle_complete = bundle_required_files()
        .iter()
        .all(|file| Path::new(&context.bundle_dir).join(file).is_file());
    let root_authority = root_authority();
    let host_context = network_host_substrate::observe_network_host_runtime_context();
    activation_status_with_host_context(
        root_policy,
        context,
        membrane_content,
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
    membrane_content: String,
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
        executor_exists: path_exists(&context.executor),
        executor_executable: path_executable(&context.executor),
        policy_exists: path_exists(&context.policy_path),
        policy_mode_0644: path_mode_equals(&context.policy_path, MODE_0644.unix),
        membrane_exists: path_exists(&context.membrane_target),
        membrane_mode_0440: path_mode_equals(&context.membrane_target, MODE_0440.unix),
        membrane_admits_network_actuate: membrane_content.contains(&context.membrane_admission),
        membrane_no_raw_ip: forbidden_pattern_blocked(
            &root_policy,
            &membrane_content,
            &context.route_command,
        ),
        membrane_no_raw_wg: forbidden_pattern_blocked(
            &root_policy,
            &membrane_content,
            &context.wireguard_command,
        ) && forbidden_pattern_blocked(
            &root_policy,
            &membrane_content,
            "/usr/bin/wg-quick",
        ),
        membrane_no_shell: forbidden_pattern_blocked(
            &root_policy,
            &membrane_content,
            &context.denied_shell_bin,
        ) && forbidden_pattern_blocked(
            &root_policy,
            &membrane_content,
            &context.denied_bash_bin,
        ),
    };

    let mut missing = filesystem_and_membrane_diagnostics(&checks, bundle_complete);
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
        executor: context.executor,
        policy_path: context.policy_path,
        membrane_target: context.membrane_target,
        membrane_admission: root_policy.membrane_admission.clone(),
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

fn filesystem_and_membrane_diagnostics(
    checks: &NetworkActuatorChecks,
    bundle_complete: bool,
) -> Vec<NetworkDiagnostic> {
    let mut missing = Vec::new();
    if !checks.executor_exists {
        missing.push(diagnostic(
            "executor_exists",
            "filesystem",
            "cynic executor binary is missing",
        ));
    }
    if !checks.executor_executable {
        missing.push(diagnostic(
            "executor_executable",
            "filesystem",
            "cynic executor binary is not executable",
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
    if !checks.membrane_exists {
        missing.push(diagnostic(
            "membrane_exists",
            "filesystem",
            "privilege membrane is missing",
        ));
    }
    if !checks.membrane_mode_0440 {
        missing.push(diagnostic(
            "membrane_mode_0440",
            "filesystem",
            "privilege membrane mode is not 0440",
        ));
    }
    if !checks.membrane_admits_network_actuate {
        missing.push(diagnostic(
            "membrane_admits_network_actuate",
            "membrane",
            "membrane does not admit the network-actuate capability",
        ));
    }
    if !checks.membrane_no_raw_ip {
        missing.push(diagnostic(
            "membrane_no_raw_ip",
            "membrane",
            "raw route tool access is still exposed in the membrane",
        ));
    }
    if !checks.membrane_no_raw_wg {
        missing.push(diagnostic(
            "membrane_no_raw_wg",
            "membrane",
            "raw wireguard access is still exposed in the membrane",
        ));
    }
    if !checks.membrane_no_shell {
        missing.push(diagnostic(
            "membrane_no_shell",
            "membrane",
            "shell execution is still exposed in the membrane",
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
        "executor_exists",
        "executor_executable",
        "policy_exists",
        "policy_mode_0644",
        "membrane_exists",
        "membrane_mode_0440",
        "membrane_admits_network_actuate",
    ]
    .iter()
    .any(|item| has_diagnostic(missing, item))
    {
        actions.push(diagnostic(
            "install_network_actuator_bundle_on_target_node",
            "action",
            "install policy and the bridge privilege membrane on the target node",
        ));
    }
    if ["membrane_no_raw_ip", "membrane_no_raw_wg", "membrane_no_shell"]
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
            "install_privilege_membrane_once",
            "action",
            "install the privilege membrane once so bootstrap can complete",
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
                service_route_active: false,
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
                executor: "/missing/executor".to_string(),
                policy_path: "/missing/policy".to_string(),
                membrane_target: "/missing/membrane".to_string(),
                membrane_admission: "/missing/executor privilege exec network-actuate".to_string(),
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
                .any(|item| item.code == "executor_exists")
        );
        assert!(
            status
                .missing
                .iter()
                .any(|item| item.code == "membrane_exists")
        );
    }
}
