use super::{
    NetworkRootPolicyContext, allowed_profiles, build_root_policy_with_context,
    remediation_to_profile,
};
use crate::domain::node_observation::{
    NodeRuntimeAction, NodeRuntimeCapabilities, NodeRuntimeRemediation,
    SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS, SURFACE_NETWORK_MESH_STATUS,
};
use crate::observe::NodeObservation;

fn portable_context() -> NetworkRootPolicyContext {
    NetworkRootPolicyContext {
        executor: "/portable/bin/cynic".to_string(),
        membrane_target: "/portable/etc/sudoers.d/cynic".to_string(),
        membrane_admission: "/portable/bin/cynic privilege exec network-actuate".to_string(),
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
        "sudo -n rejects privileged commands outside the membrane for the cynic user"
    );
    assert_eq!(policy.executor, "/portable/bin/cynic");
    assert_eq!(
        policy.membrane_target,
        "/portable/etc/sudoers.d/cynic"
    );
}

#[kani::proof]
fn proof_remediation_to_profile_maps_only_root_network_actions() {
    let helper = "/portable/bin/cynic-network-actuator";
    let remediation = NodeRuntimeRemediation {
        action: NodeRuntimeAction::ActivateWireguardMesh,
        reason: "mesh missing",
        requires_root: true,
        mutates_host: true,
        canonical_surface: SURFACE_NETWORK_MESH_STATUS,
        fallback_surface: "provider fallback".to_string(),
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
        reason: "dns degraded",
        requires_root: false,
        mutates_host: false,
        canonical_surface: SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS,
        fallback_surface: "provider fallback dns".to_string(),
    };

    assert!(remediation_to_profile(&remediation, helper).is_none());
}

#[kani::proof]
fn proof_allowed_profiles_only_contains_root_remediations() {
    let observation = NodeObservation {
        remediation: vec![
            NodeRuntimeRemediation {
                action: NodeRuntimeAction::ActivateSynapseRoute,
                reason: "route missing",
                requires_root: true,
                mutates_host: true,
                canonical_surface: SURFACE_NETWORK_MESH_STATUS,
                fallback_surface: "provider fallback route".to_string(),
            },
            NodeRuntimeRemediation {
                action: NodeRuntimeAction::RestoreCriticalDns,
                reason: "dns missing",
                requires_root: false,
                mutates_host: false,
                canonical_surface: SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS,
                fallback_surface: "provider fallback dns".to_string(),
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
