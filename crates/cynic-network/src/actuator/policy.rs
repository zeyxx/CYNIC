use std::collections::BTreeMap;

use crate::actuator::types::{
    NetworkActuationAllowedProfile, NetworkActuationPolicyDocument, NetworkActuationRuntime,
};
use crate::command_surface::argv_command;

pub fn build_network_actuation_policy<'a>(
    allowed_profiles: impl IntoIterator<Item = &'a str>,
    runtime: &NetworkActuationRuntime,
) -> NetworkActuationPolicyDocument {
    let mut profiles = BTreeMap::new();
    for profile in allowed_profiles {
        let Some(allowed) = allowed_profile(profile, runtime) else {
            continue;
        };
        profiles.insert(profile.to_string(), allowed);
    }
    NetworkActuationPolicyDocument {
        schema: "cynic.network_actuator.policy.v1",
        status: "planned",
        executor_path: runtime.executor_path.clone(),
        sudoers_subject: runtime.sudoers_subject,
        max_timeout_seconds: runtime.max_timeout_seconds,
        allowed_profiles: profiles,
        forbidden_tokens: vec![
            "sh", "bash", "-c", "curl", "wget", "nc", "python", "python3", "perl", "ruby", ">",
            ">>", "|", "&&", ";",
        ],
    }
}

fn allowed_profile(
    profile: &str,
    runtime: &NetworkActuationRuntime,
) -> Option<NetworkActuationAllowedProfile> {
    match profile {
        "provider-fallback-route" => Some(route_profile(
            "Restore temporary provider fallback route for resurrection only.",
            &runtime.route_bin,
        )),
        "synapse-route" => Some(route_profile(
            "Restore canonical Synapse route on the local node.",
            &runtime.route_bin,
        )),
        "wireguard-mesh" => Some(NetworkActuationAllowedProfile {
            description: "Activate or inspect canonical WireGuard mesh on the local node.",
            command_prefixes: vec![
                argv_command(
                    "start_service",
                    vec![
                        runtime.service_manager_bin.clone(),
                        "start".to_string(),
                        runtime.wireguard_unit.clone(),
                    ],
                ),
                argv_command(
                    "restart_service",
                    vec![
                        runtime.service_manager_bin.clone(),
                        "restart".to_string(),
                        runtime.wireguard_unit.clone(),
                    ],
                ),
                argv_command(
                    "service_status",
                    vec![
                        runtime.service_manager_bin.clone(),
                        "is-active".to_string(),
                        runtime.wireguard_unit.clone(),
                    ],
                ),
                argv_command(
                    "inspect_wireguard",
                    vec![runtime.wireguard_bin.clone(), "show".to_string()],
                ),
            ],
        }),
        "direct-management-link" => Some(NetworkActuationAllowedProfile {
            description: "Enroll a directly attached management link without replacing the operator default route.",
            command_prefixes: vec![
                argv_command(
                    "configure_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "modify".to_string(),
                    ],
                ),
                argv_command(
                    "activate_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "up".to_string(),
                    ],
                ),
                argv_command(
                    "inspect_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                    ],
                ),
            ],
        }),
        _ => None,
    }
}

fn route_profile(description: &'static str, route_bin: &str) -> NetworkActuationAllowedProfile {
    NetworkActuationAllowedProfile {
        description,
        command_prefixes: vec![
            argv_command(
                "mutate_route",
                vec![
                    route_bin.to_string(),
                    "route".to_string(),
                    "replace".to_string(),
                ],
            ),
            argv_command(
                "inspect_route",
                vec![
                    route_bin.to_string(),
                    "route".to_string(),
                    "show".to_string(),
                ],
            ),
        ],
    }
}