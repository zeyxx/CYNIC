use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator_status::types::{NetworkActuatorProofRequirement, RootAuthorityCheck};
use crate::network_root_policy::NetworkRootPolicy;

#[derive(Debug, Clone)]
struct NetworkActuatorProofContext {
    sudo_bin: String,
    sudo_true_command: String,
    route_bin: String,
    wireguard_bin: String,
    denied_shell_bin: String,
    default_bundle_dir: String,
}

impl NetworkActuatorProofContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            sudo_bin: runtime_layout.sudo_bin(),
            sudo_true_command: runtime_layout.sudo_noninteractive_command(&["true"]),
            route_bin: capabilities.route_tool.command.clone(),
            wireguard_bin: capabilities.wireguard_tool.command.clone(),
            denied_shell_bin: "/bin/sh".to_string(),
            default_bundle_dir: runtime_layout.network_actuator_bundle_dir.clone(),
        }
    }
}

pub(crate) fn proof_requirements(
    root_policy: &NetworkRootPolicy,
) -> Vec<NetworkActuatorProofRequirement> {
    proof_requirements_with_context(
        root_policy,
        &NetworkActuatorProofContext::from_current_runtime(),
    )
}

fn proof_requirements_with_context(
    root_policy: &NetworkRootPolicy,
    context: &NetworkActuatorProofContext,
) -> Vec<NetworkActuatorProofRequirement> {
    let mut requirements = vec![
        NetworkActuatorProofRequirement {
            check: "bundle_complete",
            commands: vec![crate::network_command_surface::argv_command(
                "bundle",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-q".to_string(),
                    "-p".to_string(),
                    "cynic-node".to_string(),
                    "--".to_string(),
                    "network".to_string(),
                    "actuator-bundle".to_string(),
                    "--output-dir".to_string(),
                    context.default_bundle_dir.clone(),
                ],
            )],
            expected: "bundle contains manifest.json, cynic-network-actuator, network-actuator.policy.json and cynic-network-actuator.sudoers",
            evidence_source: "default_bundle_dir",
        },
        NetworkActuatorProofRequirement {
            check: "helper_and_policy_installed",
            commands: vec![crate::network_command_surface::argv_command(
                "status",
                vec![
                    "cargo".to_string(),
                    "run".to_string(),
                    "-p".to_string(),
                    "cynic-node".to_string(),
                    "--".to_string(),
                    "network".to_string(),
                    "actuator-status".to_string(),
                    "--json".to_string(),
                ],
            )],
            expected: "helper, policy and sudoers are installed with expected modes",
            evidence_source: "local_filesystem",
        },
    ];

    for denial in &root_policy.denial_contract {
        let commands = match denial.rule {
            "commands outside sudoers_rule fail non-interactively" => {
                vec![crate::network_command_surface::argv_command(
                    "deny_route",
                    vec![
                        context.sudo_bin.clone(),
                        "-n".to_string(),
                        context.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                    ],
                )]
            }
            "raw ip/wg/nmcli invocations are never present in sudoers" => {
                vec![crate::network_command_surface::argv_command(
                    "deny_wireguard",
                    vec![
                        context.sudo_bin.clone(),
                        "-n".to_string(),
                        context.wireguard_bin.clone(),
                        "show".to_string(),
                    ],
                )]
            }
            "shell execution through the helper is forbidden" => {
                vec![crate::network_command_surface::argv_command(
                    "deny_shell",
                    vec![
                        context.sudo_bin.clone(),
                        "-n".to_string(),
                        context.denied_shell_bin.clone(),
                        "-c".to_string(),
                        "true".to_string(),
                    ],
                )]
            }
            _ => Vec::new(),
        };
        requirements.push(NetworkActuatorProofRequirement {
            check: denial.rule,
            commands,
            expected: denial.expected,
            evidence_source: "network_root_policy",
        });
    }

    for requirement in &root_policy.evidence_requirements {
        requirements.push(NetworkActuatorProofRequirement {
            check: requirement.requirement,
            commands: Vec::new(),
            expected: requirement.evidence,
            evidence_source: requirement.phase,
        });
    }

    requirements
}

pub(crate) fn root_authority() -> RootAuthorityCheck {
    root_authority_with_context(&NetworkActuatorProofContext::from_current_runtime())
}

fn root_authority_with_context(context: &NetworkActuatorProofContext) -> RootAuthorityCheck {
    match capture_command(&context.sudo_bin, &["-n", "true"]) {
        Ok(output) => RootAuthorityCheck {
            command: context.sudo_true_command.clone(),
            sudo_noninteractive_available: output.success,
            exit_code: output.exit_code,
            stderr: output.stderr,
        },
        Err(error) => RootAuthorityCheck {
            command: context.sudo_true_command.clone(),
            sudo_noninteractive_available: false,
            exit_code: None,
            stderr: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{NetworkActuatorProofContext, proof_requirements_with_context};
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_root_policy::build_root_policy;
    use crate::observe::NodeObservation;

    #[test]
    fn proof_requirements_use_injected_bins() {
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
        let requirements = proof_requirements_with_context(
            &root_policy,
            &NetworkActuatorProofContext {
                sudo_bin: "/portable/bin/sudo".to_string(),
                sudo_true_command: "/portable/bin/sudo -n true".to_string(),
                route_bin: "/portable/bin/ip".to_string(),
                wireguard_bin: "/portable/bin/wg".to_string(),
                denied_shell_bin: "/portable/bin/sh".to_string(),
                default_bundle_dir: "/portable/runtime/actuator".to_string(),
            },
        );

        assert!(requirements.iter().any(|item| {
            item.check == "bundle_complete"
                && item.commands.iter().any(|command| {
                    crate::network_command_surface::render_command(command)
                        .contains("/portable/runtime/actuator")
                })
        }));
        assert!(requirements.iter().any(|item| {
            item.check == "raw ip/wg/nmcli invocations are never present in sudoers"
                && item.commands.iter().any(|command| {
                    crate::network_command_surface::render_command(command)
                        .contains("/portable/bin/wg")
                })
        }));
        assert!(requirements.iter().any(|item| {
            item.check == "shell execution through the helper is forbidden"
                && item.commands.iter().any(|command| {
                    crate::network_command_surface::render_command(command)
                        .contains("/portable/bin/sh")
                })
        }));
    }
}
