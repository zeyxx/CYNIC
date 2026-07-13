use super::{
    NetworkRootPolicyContext, allowed_profiles, build_root_policy_with_context,
    remediation_to_profile,
};
use crate::domain::node_observation::{
    NodeRuntimeAction, NodeRuntimeCapabilities, NodeRuntimeRemediation,
};
use crate::observe::NodeObservation;

fn portable_context() -> NetworkRootPolicyContext {
    NetworkRootPolicyContext {
        helper_path: "/portable/bin/cynic-network-actuator".to_string(),
        sudoers_target: "/portable/etc/sudoers.d/cynic-network-actuator".to_string(),
        sudoers_rule: "cynic ALL=(root) NOPASSWD: /portable/bin/cynic-network-actuator".to_string(),
        forbidden_patterns: vec![
            "/portable/bin/ip".to_string(),
            "/portable/bin/wg".to_string(),
        ],
    }
}

#[kani::proof]
fn proof_root_policy_keeps_denial_contract_shape() {
    let observation = NodeObservation::synthetic_for_tests(
        NodeRuntimeCapabilities {
            provider_fallback_active: false,
            synapse_route_active: true,
            wireguard_active: true,
            critical_dns_ok: true,
            critical_dns_canonical: true,
            critical_tcp_open: true,
        },
        Some("192.168.0.10".to_string()),
    );
    let policy = build_root_policy_with_context(&observation, portable_context());

    assert_eq!(policy.denial_contract.len(), 3);
    assert_eq!(
        policy.denial_contract[0].expected,
        "sudo -n rejects non-helper privileged commands for the cynic user"
    );
    assert_eq!(policy.helper_path, "/portable/bin/cynic-network-actuator");
    assert_eq!(
        policy.sudoers_target,
        "/portable/etc/sudoers.d/cynic-network-actuator"
    );
}

#[kani::proof]
fn proof_remediation_to_profile_maps_only_root_network_actions() {
    let helper = "/portable/bin/cynic-network-actuator";
    let remediation = NodeRuntimeRemediation {
        action: NodeRuntimeAction::ActivateWireguardMesh,
        reason: "mesh missing".to_string(),
        requires_root: true,
        mutates_host: true,
    };

    let profile = remediation_to_profile(&remediation, helper).expect("profile");
    assert_eq!(profile.profile, "wireguard-mesh");
    assert_eq!(profile.argv_prefix[0], helper);
    assert_eq!(profile.argv_prefix[4], "--apply");
}

#[kani::proof]
fn proof_dns_remediation_never_becomes_root_profile() {
    let helper = "/portable/bin/cynic-network-actuator";
    let remediation = NodeRuntimeRemediation {
        action: NodeRuntimeAction::RestoreCriticalDns,
        reason: "dns degraded".to_string(),
        requires_root: false,
        mutates_host: false,
    };

    assert!(remediation_to_profile(&remediation, helper).is_none());
}

#[kani::proof]
fn proof_allowed_profiles_only_contains_root_remediations() {
    let observation = NodeObservation {
        remediation: vec![
            NodeRuntimeRemediation {
                action: NodeRuntimeAction::ActivateSynapseRoute,
                reason: "route missing".to_string(),
                requires_root: true,
                mutates_host: true,
            },
            NodeRuntimeRemediation {
                action: NodeRuntimeAction::RestoreCriticalDns,
                reason: "dns missing".to_string(),
                requires_root: false,
                mutates_host: false,
            },
        ],
        ..NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: false,
                synapse_route_active: false,
                wireguard_active: true,
                critical_dns_ok: false,
                critical_dns_canonical: false,
                critical_tcp_open: true,
            },
            Some("192.168.0.10".to_string()),
        )
    };

    let profiles = allowed_profiles(&observation, "/portable/bin/cynic-network-actuator");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].profile, "synapse-route");
    assert!(profiles[0].requires_root);
    assert!(profiles[0].mutates_host);
}
