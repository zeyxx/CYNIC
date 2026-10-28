use crate::actuator::types::{NetworkActuationRuntime, NetworkActuationTarget};
use crate::command_surface::argv_command;

pub fn build_network_actuation_commands(
    profile: &str,
    target: &NetworkActuationTarget,
    runtime: &NetworkActuationRuntime,
) -> Option<Vec<crate::command_surface::HostCommandSurface>> {
    match profile {
        "provider-fallback-route" => target.provider_fallback_gateway.as_ref().map(|gateway| {
            vec![
                argv_command(
                    "mutate_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "replace".to_string(),
                        target.provider_fallback_cidr.clone(),
                        "via".to_string(),
                        gateway.clone(),
                    ],
                ),
                argv_command(
                    "inspect_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                        target.provider_fallback_cidr.clone(),
                    ],
                ),
            ]
        }),
        "synapse-route" => Some(vec![
            argv_command(
                "mutate_route",
                vec![
                    runtime.route_bin.clone(),
                    "route".to_string(),
                    "replace".to_string(),
                    target.service_cidr.clone(),
                    "dev".to_string(),
                    target.mesh_interface.clone(),
                ],
            ),
            argv_command(
                "inspect_route",
                vec![
                    runtime.route_bin.clone(),
                    "route".to_string(),
                    "show".to_string(),
                    target.service_cidr.clone(),
                ],
            ),
        ]),
        "wireguard-mesh" => Some(vec![
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
        ]),
        "direct-management-link" => {
            let interface = target.management_interface.as_ref()?;
            let connection = target.management_connection.as_ref()?;
            let address = target.management_address.as_ref()?;
            let gateway = target.management_gateway.as_ref()?;
            Some(vec![
                argv_command(
                    "configure_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "modify".to_string(),
                        connection.clone(),
                        "connection.interface-name".to_string(),
                        interface.clone(),
                        "ipv4.method".to_string(),
                        "manual".to_string(),
                        "ipv4.addresses".to_string(),
                        address.clone(),
                        "ipv4.gateway".to_string(),
                        String::new(),
                        "ipv4.dns".to_string(),
                        String::new(),
                        "ipv4.never-default".to_string(),
                        "yes".to_string(),
                        "ipv4.routes".to_string(),
                        format!("{} {}", target.service_cidr, gateway),
                        "ipv4.dad-timeout".to_string(),
                        "3000".to_string(),
                        "ipv6.never-default".to_string(),
                        "yes".to_string(),
                    ],
                ),
                argv_command(
                    "activate_management_link",
                    vec![
                        runtime.network_manager_bin.clone(),
                        "connection".to_string(),
                        "up".to_string(),
                        connection.clone(),
                    ],
                ),
                argv_command(
                    "inspect_route",
                    vec![
                        runtime.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                        target.service_cidr.clone(),
                    ],
                ),
            ])
        }
        _ => None,
    }
}