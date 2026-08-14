use cynic_host::network_actuation::{NetworkActuationRuntime, build_network_actuation_policy};
use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator::types::NetworkActuatorPolicyDocument;
use crate::network_actuator_contract::{NETWORK_ACTUATOR_SUDOERS_SUBJECT, helper_path};
use crate::network_root_policy::build_root_policy;
use crate::observe::NodeObservation;

#[derive(Debug, Clone)]
struct NetworkActuatorPolicyContext {
    helper_path: String,
    systemctl_bin: String,
    wg_quick_unit: String,
    ip_bin: String,
    wg_bin: String,
    nmcli_bin: String,
}

impl NetworkActuatorPolicyContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            helper_path: helper_path(),
            systemctl_bin: capabilities.service_manager.command.clone(),
            wg_quick_unit: runtime_layout.wg_quick_unit(),
            ip_bin: capabilities.route_tool.command.clone(),
            wg_bin: capabilities.wireguard_tool.command.clone(),
            nmcli_bin: capabilities.network_manager.command.clone(),
        }
    }
}

pub(crate) fn build_policy_document(
    observation: &NodeObservation,
) -> NetworkActuatorPolicyDocument {
    let root_policy = build_root_policy(observation);
    build_policy_document_with_context(
        &root_policy,
        NetworkActuatorPolicyContext::from_current_runtime(),
    )
}

fn build_policy_document_with_context(
    root_policy: &crate::network_root_policy::NetworkRootPolicy,
    context: NetworkActuatorPolicyContext,
) -> NetworkActuatorPolicyDocument {
    let runtime = NetworkActuationRuntime {
        helper_path: context.helper_path,
        sudoers_subject: NETWORK_ACTUATOR_SUDOERS_SUBJECT,
        max_timeout_seconds: 60,
        service_manager_bin: context.systemctl_bin,
        wireguard_unit: context.wg_quick_unit,
        route_bin: context.ip_bin,
        wireguard_bin: context.wg_bin,
        network_manager_bin: context.nmcli_bin,
    };
    build_network_actuation_policy(
        root_policy
            .allowed_profiles
            .iter()
            .map(|profile| profile.profile),
        &runtime,
    )
}

#[cfg(test)]
mod tests {
    use super::{NetworkActuatorPolicyContext, build_policy_document_with_context};
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_root_policy::build_root_policy;

    #[test]
    fn policy_document_uses_injected_runtime_context() {
        let observation = crate::observe::NodeObservation::synthetic_for_tests(
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
        let policy = build_policy_document_with_context(
            &root_policy,
            NetworkActuatorPolicyContext {
                helper_path: "/portable/bin/cynic-network-actuator".to_string(),
                systemctl_bin: "/portable/bin/systemctl".to_string(),
                wg_quick_unit: "wg-quick@wg-portable".to_string(),
                ip_bin: "/portable/bin/ip".to_string(),
                wg_bin: "/portable/bin/wg".to_string(),
                nmcli_bin: "/portable/bin/nmcli".to_string(),
            },
        );

        assert_eq!(policy.helper_path, "/portable/bin/cynic-network-actuator");
        let wireguard = policy.allowed_profiles.get("wireguard-mesh").unwrap();
        let rendered = match &wireguard.command_prefixes[0].invocation {
            crate::network_command_surface::NetworkCommandInvocation::Argv { argv } => argv.clone(),
            _ => Vec::new(),
        };
        assert_eq!(rendered[0], "/portable/bin/systemctl");
        assert_eq!(rendered[2], "wg-quick@wg-portable");
        let route = policy.allowed_profiles.get("synapse-route").unwrap();
        let rendered_route = match &route.command_prefixes[0].invocation {
            crate::network_command_surface::NetworkCommandInvocation::Argv { argv } => argv.clone(),
            _ => Vec::new(),
        };
        assert_eq!(rendered_route[0], "/portable/bin/ip");
    }
}
