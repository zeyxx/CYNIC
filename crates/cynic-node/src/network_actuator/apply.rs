use chrono::Utc;
use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator::execution::run_command;
use crate::network_actuator::policy::build_policy_document;
use crate::network_actuator::types::{NetworkActuatorApplyReport, NetworkActuatorPolicyDocument};
use crate::network_actuator_contract::NETWORK_ACTUATOR_HELPER_CONTRACT;
use crate::network_command_surface::{NetworkCommandSurface, argv_command};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};
use crate::observe::NodeObservation;

#[derive(Debug, Clone)]
struct NetworkActuatorApplyContext {
    mesh_interface: String,
    wg_quick_unit: String,
    systemctl_bin: String,
    ip_bin: String,
    wg_bin: String,
}

impl NetworkActuatorApplyContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            mesh_interface: capabilities.wireguard_tool.mesh_interface.clone(),
            wg_quick_unit: runtime_layout.wg_quick_unit(),
            systemctl_bin: capabilities.service_manager.command.clone(),
            ip_bin: capabilities.route_tool.command.clone(),
            wg_bin: capabilities.wireguard_tool.command.clone(),
        }
    }
}

pub(crate) fn apply_profile(
    observation: &NodeObservation,
    profile: &str,
    apply: bool,
) -> NetworkActuatorApplyReport {
    apply_profile_with_context(
        observation,
        profile,
        apply,
        &NetworkActuatorApplyContext::from_current_runtime(),
    )
}

fn apply_profile_with_context(
    observation: &NodeObservation,
    profile: &str,
    apply: bool,
    context: &NetworkActuatorApplyContext,
) -> NetworkActuatorApplyReport {
    let policy = build_policy_document(observation);
    let violations = validate_profile_request(&policy, profile, apply);
    let commands = profile_commands_with_context(observation, profile, context).unwrap_or_default();
    if !violations.is_empty() || commands.is_empty() {
        return NetworkActuatorApplyReport {
            schema: "cynic.network_actuator.apply.v1",
            generated_at: Utc::now().to_rfc3339(),
            status: "refused",
            profile: profile.to_string(),
            apply,
            helper_contract: NETWORK_ACTUATOR_HELPER_CONTRACT,
            commands,
            executions: Vec::new(),
            violations,
        };
    }

    let executions = commands.iter().map(run_command).collect::<Vec<_>>();
    let success = executions
        .iter()
        .all(|execution| execution.exit_code == Some(0));

    NetworkActuatorApplyReport {
        schema: "cynic.network_actuator.apply.v1",
        generated_at: Utc::now().to_rfc3339(),
        status: if success { "completed" } else { "failed" },
        profile: profile.to_string(),
        apply,
        helper_contract: NETWORK_ACTUATOR_HELPER_CONTRACT,
        commands,
        executions,
        violations: Vec::new(),
    }
}

fn validate_profile_request(
    policy: &NetworkActuatorPolicyDocument,
    profile: &str,
    apply: bool,
) -> Vec<NetworkDiagnostic> {
    let mut violations = Vec::new();
    if profile.trim().is_empty() {
        violations.push(diagnostic(
            "missing_profile",
            "request_validation",
            "profile is required",
        ));
    }
    if !apply {
        violations.push(diagnostic(
            "apply_flag_required",
            "request_validation",
            "apply flag is required for actuator execution",
        ));
    }
    if !policy.allowed_profiles.contains_key(profile) {
        violations.push(diagnostic(
            "profile_not_allowed_for_current_runtime",
            "request_validation",
            "profile is not allowed for the current runtime observation",
        ));
    }
    violations
}

fn profile_commands_with_context(
    observation: &NodeObservation,
    profile: &str,
    context: &NetworkActuatorApplyContext,
) -> Option<Vec<NetworkCommandSurface>> {
    match profile {
        "provider-fallback-route" => {
            observation
                .topology
                .provider_fallback_gateway
                .as_ref()
                .map(|gateway| {
                    vec![
                        argv_command(
                            "mutate_route",
                            vec![
                                context.ip_bin.clone(),
                                "route".to_string(),
                                "replace".to_string(),
                                observation.topology.provider_fallback_cidr.clone(),
                                "via".to_string(),
                                gateway.clone(),
                            ],
                        ),
                        argv_command(
                            "inspect_route",
                            vec![
                                context.ip_bin.clone(),
                                "route".to_string(),
                                "show".to_string(),
                                observation.topology.provider_fallback_cidr.clone(),
                            ],
                        ),
                    ]
                })
        }
        "synapse-route" => Some(vec![
            argv_command(
                "mutate_route",
                vec![
                    context.ip_bin.clone(),
                    "route".to_string(),
                    "replace".to_string(),
                    observation.topology.synapse_cidr.clone(),
                    "dev".to_string(),
                    context.mesh_interface.clone(),
                ],
            ),
            argv_command(
                "inspect_route",
                vec![
                    context.ip_bin.clone(),
                    "route".to_string(),
                    "show".to_string(),
                    observation.topology.synapse_cidr.clone(),
                ],
            ),
        ]),
        "wireguard-mesh" => Some(vec![
            argv_command(
                "restart_service",
                vec![
                    context.systemctl_bin.clone(),
                    "restart".to_string(),
                    context.wg_quick_unit.clone(),
                ],
            ),
            argv_command(
                "service_status",
                vec![
                    context.systemctl_bin.clone(),
                    "is-active".to_string(),
                    context.wg_quick_unit.clone(),
                ],
            ),
            argv_command(
                "inspect_wireguard",
                vec![context.wg_bin.clone(), "show".to_string()],
            ),
        ]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkActuatorApplyContext, apply_profile_with_context, profile_commands_with_context,
    };
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_command_surface::NetworkCommandInvocation;

    fn portable_context() -> NetworkActuatorApplyContext {
        NetworkActuatorApplyContext {
            mesh_interface: "wg-portable".to_string(),
            wg_quick_unit: "wg-quick@wg-portable".to_string(),
            systemctl_bin: "/portable/bin/systemctl".to_string(),
            ip_bin: "/portable/bin/ip".to_string(),
            wg_bin: "/portable/bin/wg".to_string(),
        }
    }

    fn observation() -> crate::observe::NodeObservation {
        crate::observe::NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        )
    }

    #[test]
    fn synapse_route_profile_uses_injected_bins() {
        let commands =
            profile_commands_with_context(&observation(), "synapse-route", &portable_context())
                .unwrap();

        match &commands[0].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/portable/bin/ip");
                assert!(argv.iter().any(|part| part == "wg-portable"));
            }
            _ => panic!("expected argv command"),
        }
    }

    #[test]
    fn wireguard_mesh_profile_uses_injected_bins() {
        let commands =
            profile_commands_with_context(&observation(), "wireguard-mesh", &portable_context())
                .unwrap();

        match &commands[0].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/portable/bin/systemctl");
                assert_eq!(argv[2], "wg-quick@wg-portable");
            }
            _ => panic!("expected argv command"),
        }
        match &commands[2].invocation {
            NetworkCommandInvocation::Argv { argv } => {
                assert_eq!(argv[0], "/portable/bin/wg");
            }
            _ => panic!("expected argv command"),
        }
    }

    #[test]
    fn apply_profile_with_context_preserves_refusal_without_apply() {
        let report =
            apply_profile_with_context(&observation(), "synapse-route", false, &portable_context());

        assert_eq!(report.status, "refused");
        assert!(!report.commands.is_empty());
        assert!(
            report
                .violations
                .iter()
                .any(|item| item.code == "apply_flag_required")
        );
    }
}
