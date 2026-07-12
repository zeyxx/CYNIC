use chrono::Utc;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;

use crate::domain::node_observation::{NodeRuntimeAction, NodeRuntimeRemediation};
use crate::network_actuator_contract::{
    NETWORK_ACTUATOR_HELPER_CONTRACT, helper_path, sudoers_path, sudoers_rule,
};
use crate::network_command_surface::{NetworkCommandSurface, shell_command};
use crate::observe::NodeObservation;

#[derive(Debug, Clone)]
struct NetworkRootPolicyContext {
    helper_path: String,
    sudoers_target: String,
    sudoers_rule: String,
    forbidden_patterns: Vec<String>,
}

impl NetworkRootPolicyContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        Self {
            helper_path: helper_path(),
            sudoers_target: sudoers_path(),
            sudoers_rule: sudoers_rule(),
            forbidden_patterns: runtime_layout.forbidden_raw_invocation_patterns(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkRootPolicy {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) principal: &'static str,
    pub(crate) helper_path: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) actuator_contract: &'static str,
    pub(crate) allowed_profiles: Vec<AllowedProfile>,
    pub(crate) forbidden_invocation_patterns: Vec<ForbiddenInvocationPattern>,
    pub(crate) execution_requirements: Vec<ExecutionRequirement>,
    pub(crate) evidence_requirements: Vec<EvidenceRequirement>,
    pub(crate) denial_contract: Vec<DenialRequirement>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AllowedProfile {
    pub(crate) profile: &'static str,
    purpose: &'static str,
    action: NodeRuntimeAction,
    argv_prefix: Vec<String>,
    mutates_host: bool,
    requires_root: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ForbiddenInvocationPattern {
    pub(crate) pattern: String,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExecutionRequirement {
    pub(crate) phase: &'static str,
    pub(crate) requirement: &'static str,
    pub(crate) commands: Vec<NetworkCommandSurface>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EvidenceRequirement {
    pub(crate) phase: &'static str,
    pub(crate) requirement: &'static str,
    pub(crate) evidence: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DenialRequirement {
    pub(crate) rule: &'static str,
    pub(crate) expected: &'static str,
}

pub(crate) fn build_root_policy(observation: &NodeObservation) -> NetworkRootPolicy {
    build_root_policy_with_context(
        observation,
        NetworkRootPolicyContext::from_current_runtime(),
    )
}

fn build_root_policy_with_context(
    observation: &NodeObservation,
    context: NetworkRootPolicyContext,
) -> NetworkRootPolicy {
    NetworkRootPolicy {
        schema: "cynic.network_root_policy.v1",
        generated_at: Utc::now().to_rfc3339(),
        principal: "cynic",
        helper_path: context.helper_path.clone(),
        sudoers_target: context.sudoers_target,
        sudoers_rule: context.sudoers_rule,
        actuator_contract: NETWORK_ACTUATOR_HELPER_CONTRACT,
        allowed_profiles: allowed_profiles(observation, &context.helper_path),
        forbidden_invocation_patterns: context
            .forbidden_patterns
            .into_iter()
            .map(|pattern| ForbiddenInvocationPattern {
                pattern,
                reason: "raw privileged invocation remains outside the bounded helper contract",
            })
            .collect(),
        execution_requirements: vec![
            ExecutionRequirement {
                phase: "preflight",
                requirement: "capture the node observation before any root apply",
                commands: vec![shell_command(
                    "observe_preflight",
                    "cynic-node observe --json",
                )],
            },
            ExecutionRequirement {
                phase: "request_validation",
                requirement: "requested profile must be present in allowed_profiles",
                commands: Vec::new(),
            },
            ExecutionRequirement {
                phase: "request_validation",
                requirement: "network-actuator apply requires explicit --apply",
                commands: Vec::new(),
            },
        ],
        evidence_requirements: vec![
            EvidenceRequirement {
                phase: "postflight",
                requirement: "capture the node network evidence after apply",
                evidence: "cynic-node network evidence --json",
            },
            EvidenceRequirement {
                phase: "postflight",
                requirement: "postflight status must not degrade relative to preflight",
                evidence: "compare preflight and postflight observation/status surfaces",
            },
            EvidenceRequirement {
                phase: "audit",
                requirement: "mutation result records exact profile and argv",
                evidence: "network_actuator.apply report includes profile, commands and executions",
            },
        ],
        denial_contract: vec![
            DenialRequirement {
                rule: "commands outside sudoers_rule fail non-interactively",
                expected: "sudo -n rejects non-helper privileged commands for the cynic user",
            },
            DenialRequirement {
                rule: "raw ip/wg/nmcli invocations are never present in sudoers",
                expected: "sudoers projection exposes only the bounded helper path",
            },
            DenialRequirement {
                rule: "shell execution through the helper is forbidden",
                expected: "policy and helper reject shell-style escalation paths",
            },
        ],
    }
}

fn allowed_profiles(observation: &NodeObservation, helper: &str) -> Vec<AllowedProfile> {
    observation
        .remediation
        .iter()
        .filter(|remediation| remediation.requires_root)
        .filter_map(|remediation| remediation_to_profile(remediation, helper))
        .collect()
}

fn remediation_to_profile(
    remediation: &NodeRuntimeRemediation,
    helper: &str,
) -> Option<AllowedProfile> {
    match remediation.action {
        NodeRuntimeAction::ActivateSynapseRoute => Some(AllowedProfile {
            profile: "synapse-route",
            purpose: "restore canonical Synapse route on this node",
            action: remediation.action.clone(),
            argv_prefix: vec![
                helper.to_string(),
                "apply".to_string(),
                "--profile".to_string(),
                "synapse-route".to_string(),
                "--apply".to_string(),
            ],
            mutates_host: remediation.mutates_host,
            requires_root: remediation.requires_root,
        }),
        NodeRuntimeAction::ActivateWireguardMesh => Some(AllowedProfile {
            profile: "wireguard-mesh",
            purpose: "activate canonical WireGuard mesh on this node",
            action: remediation.action.clone(),
            argv_prefix: vec![
                helper.to_string(),
                "apply".to_string(),
                "--profile".to_string(),
                "wireguard-mesh".to_string(),
                "--apply".to_string(),
            ],
            mutates_host: remediation.mutates_host,
            requires_root: remediation.requires_root,
        }),
        NodeRuntimeAction::RestoreProviderFallbackRoute => Some(AllowedProfile {
            profile: "provider-fallback-route",
            purpose: "restore temporary provider fallback route for resurrection only",
            action: remediation.action.clone(),
            argv_prefix: vec![
                helper.to_string(),
                "apply".to_string(),
                "--profile".to_string(),
                "provider-fallback-route".to_string(),
                "--apply".to_string(),
            ],
            mutates_host: remediation.mutates_host,
            requires_root: remediation.requires_root,
        }),
        NodeRuntimeAction::RestoreCriticalDns
        | NodeRuntimeAction::RestoreCanonicalDnsProjection
        | NodeRuntimeAction::RestoreCriticalTcp => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{NetworkRootPolicyContext, build_root_policy_with_context};
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::observe::NodeObservation;

    #[test]
    fn root_policy_scopes_profiles_to_required_root_remediations() {
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

        let policy = build_root_policy_with_context(
            &observation,
            NetworkRootPolicyContext {
                helper_path: "/portable/bin/cynic-network-actuator".to_string(),
                sudoers_target: "/portable/etc/sudoers.d/cynic-network-actuator".to_string(),
                sudoers_rule: "cynic ALL=(root) NOPASSWD: /portable/bin/cynic-network-actuator"
                    .to_string(),
                forbidden_patterns: vec![
                    "/portable/bin/ip".to_string(),
                    "/portable/bin/wg".to_string(),
                ],
            },
        );

        assert_eq!(policy.schema, "cynic.network_root_policy.v1");
        assert_eq!(
            policy.sudoers_rule,
            "cynic ALL=(root) NOPASSWD: /portable/bin/cynic-network-actuator"
        );
        assert_eq!(policy.allowed_profiles.len(), 2);
        assert!(
            policy
                .allowed_profiles
                .iter()
                .any(|profile| profile.profile == "synapse-route")
        );
        assert!(
            policy
                .forbidden_invocation_patterns
                .iter()
                .any(|pattern| pattern.pattern == "/portable/bin/ip")
        );
    }
}
