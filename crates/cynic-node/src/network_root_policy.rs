use chrono::Utc;
use serde::Serialize;

use crate::domain::node_observation::{NodeRuntimeAction, NodeRuntimeRemediation};
use crate::observe::NodeObservation;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkRootPolicy {
    pub(crate) schema: &'static str,
    generated_at: String,
    principal: &'static str,
    helper_path: &'static str,
    sudoers_target: &'static str,
    pub(crate) sudoers_rule: String,
    actuator_contract: &'static str,
    allowed_profiles: Vec<AllowedProfile>,
    forbidden_invocation_patterns: Vec<&'static str>,
    preflight_requirements: Vec<&'static str>,
    postflight_requirements: Vec<&'static str>,
    denial_contract: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct AllowedProfile {
    profile: &'static str,
    purpose: &'static str,
    action: NodeRuntimeAction,
    argv_prefix: Vec<String>,
    mutates_host: bool,
    requires_root: bool,
}

pub(crate) fn build_root_policy(observation: &NodeObservation) -> NetworkRootPolicy {
    let helper_path = "/usr/local/libexec/cynic-network-actuator";

    NetworkRootPolicy {
        schema: "cynic.network_root_policy.v1",
        generated_at: Utc::now().to_rfc3339(),
        principal: "cynic",
        helper_path,
        sudoers_target: "/etc/sudoers.d/cynic-network-actuator",
        sudoers_rule: format!("cynic ALL=(root) NOPASSWD: {helper_path}"),
        actuator_contract: "single helper binary, argv-prefix allowlist per profile, no shell, no raw ip/wg/nmcli sudoers exposure",
        allowed_profiles: allowed_profiles(observation),
        forbidden_invocation_patterns: vec![
            "/usr/sbin/ip",
            "/usr/bin/ip",
            "/usr/bin/wg",
            "/usr/bin/wg-quick",
            "/usr/bin/nmcli",
            "/bin/sh",
            "/usr/bin/bash",
        ],
        preflight_requirements: vec![
            "cynic-node observe --json captured before apply",
            "requested profile is in allowed_profiles",
            "network-actuator apply requires explicit --apply",
        ],
        postflight_requirements: vec![
            "cynic-node network evidence --json captured after apply",
            "postflight status must not degrade relative to preflight",
            "mutation result records exact profile and argv",
        ],
        denial_contract: vec![
            "commands outside sudoers_rule fail non-interactively",
            "raw ip/wg/nmcli invocations are never present in sudoers",
            "shell execution through the helper is forbidden",
        ],
    }
}

fn allowed_profiles(observation: &NodeObservation) -> Vec<AllowedProfile> {
    observation
        .remediation
        .iter()
        .filter(|remediation| remediation.requires_root)
        .filter_map(remediation_to_profile)
        .collect()
}

fn remediation_to_profile(remediation: &NodeRuntimeRemediation) -> Option<AllowedProfile> {
    match remediation.action {
        NodeRuntimeAction::ActivateSynapseRoute => Some(AllowedProfile {
            profile: "synapse-route",
            purpose: "restore canonical Synapse route on this node",
            action: remediation.action.clone(),
            argv_prefix: vec![
                "/usr/local/libexec/cynic-network-actuator".to_string(),
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
                "/usr/local/libexec/cynic-network-actuator".to_string(),
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
                "/usr/local/libexec/cynic-network-actuator".to_string(),
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
    use super::*;
    use crate::domain::node_observation::NodeRuntimeCapabilities;

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

        let policy = build_root_policy(&observation);

        assert_eq!(policy.schema, "cynic.network_root_policy.v1");
        assert_eq!(
            policy.sudoers_rule,
            "cynic ALL=(root) NOPASSWD: /usr/local/libexec/cynic-network-actuator"
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
                .contains(&"/usr/sbin/ip")
        );
    }
}
