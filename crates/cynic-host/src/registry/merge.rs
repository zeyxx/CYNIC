//! Registry merge — pure functions for combining registry documents

use crate::registry::types::{
    RegistryDocument, RegistryFleetDesired, RegistryFleetTarget, RegistryKernelRelease,
    RegistryLifecycle, RegistryNetworkLink, RegistryNetworkPlane, RegistryNetworkPolicy,
    RegistryNode, RegistryNodeService, RegistryNodeTransports, RegistryServiceHealth,
    RegistryServiceSupervisor, RegistrySshEnrollmentAuthority, RegistrySshTransport,
};
use std::collections::BTreeMap;

/// Merge strategy for conflicting values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Keep first (base) value
    KeepBase,
    /// Take second (overlay) value
    TakeOverlay,
    /// Fail on conflict
    Fail,
}

/// Merge two registry documents
pub fn merge_registry(
    base: RegistryDocument,
    overlay: RegistryDocument,
    strategy: MergeStrategy,
) -> Result<RegistryDocument, String> {
    let nodes = merge_nodes(base.nodes, overlay.nodes, strategy)?;
    let network_policy =
        merge_network_policy(base.network_policy, overlay.network_policy, strategy);
    let lifecycle = merge_lifecycle(base.lifecycle, overlay.lifecycle, strategy);
    let fleet_desired = merge_fleet_desired(base.fleet_desired, overlay.fleet_desired, strategy);

    Ok(RegistryDocument {
        network_policy,
        lifecycle,
        nodes,
        fleet_desired,
    })
}

/// Merge node lists
fn merge_nodes(
    mut base: Vec<RegistryNode>,
    overlay: Vec<RegistryNode>,
    strategy: MergeStrategy,
) -> Result<Vec<RegistryNode>, String> {
    for overlay_node in overlay {
        // Try to find matching node in base (by id, name, or alias)
        let match_idx = base
            .iter()
            .position(|base_node| nodes_match(base_node, &overlay_node));

        match match_idx {
            Some(idx) => {
                // Merge into existing node
                base[idx] = merge_node(base[idx].clone(), overlay_node, strategy)?;
            }
            None => {
                // Add new node
                base.push(overlay_node);
            }
        }
    }
    Ok(base)
}

/// Check if two nodes represent the same logical node
fn nodes_match(a: &RegistryNode, b: &RegistryNode) -> bool {
    // Match by id
    if let (Some(id_a), Some(id_b)) = (&a.id, &b.id) {
        if !id_a.is_empty() && id_a == id_b {
            return true;
        }
    }
    // Match by name
    if let (Some(name_a), Some(name_b)) = (&a.name, &b.name) {
        if !name_a.is_empty() && name_a == name_b {
            return true;
        }
    }
    // Match by alias
    for alias_a in &a.aliases {
        for alias_b in &b.aliases {
            if alias_a == alias_b && !alias_a.is_empty() {
                return true;
            }
        }
    }
    false
}

/// Merge two nodes
fn merge_node(
    base: RegistryNode,
    overlay: RegistryNode,
    strategy: MergeStrategy,
) -> Result<RegistryNode, String> {
    let id = merge_field(base.id, overlay.id, strategy, "id")?;
    let name = merge_field(base.name, overlay.name, strategy, "name")?;
    let aliases = merge_aliases(base.aliases, overlay.aliases, strategy);
    let role = merge_field(base.role, overlay.role, strategy, "role")?;
    let canonical_role = merge_field(
        base.canonical_role,
        overlay.canonical_role,
        strategy,
        "canonical_role",
    )?;
    let provider_controls =
        merge_provider_controls(base.provider_controls, overlay.provider_controls, strategy);
    let transports = merge_option(
        base.transports,
        overlay.transports,
        strategy,
        merge_transports,
    )?;
    let host_family = merge_field(
        base.host_family,
        overlay.host_family,
        strategy,
        "host_family",
    )?;
    let substrate = merge_option(base.substrate, overlay.substrate, strategy, merge_substrate)?;
    let services = merge_services(base.services, overlay.services, strategy);
    let network_links = merge_network_links(base.network_links, overlay.network_links, strategy);

    Ok(RegistryNode {
        id,
        name,
        aliases,
        role,
        canonical_role,
        provider_controls,
        transports,
        host_family,
        substrate,
        services,
        network_links,
    })
}

/// Merge a field with strategy
fn merge_field<T: Clone>(
    base: Option<T>,
    overlay: Option<T>,
    strategy: MergeStrategy,
    field: &str,
) -> Result<Option<T>, String> {
    match (base, overlay) {
        (Some(b), Some(o)) => match strategy {
            MergeStrategy::KeepBase => Ok(Some(b)),
            MergeStrategy::TakeOverlay => Ok(Some(o)),
            MergeStrategy::Fail => Err(format!("conflict in field '{}'", field)),
        },
        (Some(b), None) => Ok(Some(b)),
        (None, Some(o)) => Ok(Some(o)),
        (None, None) => Ok(None),
    }
}

/// Merge aliases (union)
fn merge_aliases(base: Vec<String>, overlay: Vec<String>, _strategy: MergeStrategy) -> Vec<String> {
    let mut result = base;
    for alias in overlay {
        if !alias.is_empty() && !result.contains(&alias) {
            result.push(alias);
        }
    }
    result
}

/// Merge provider controls (union)
fn merge_provider_controls(
    base: Vec<String>,
    overlay: Vec<String>,
    _strategy: MergeStrategy,
) -> Vec<String> {
    let mut result = base;
    for provider in overlay {
        if !provider.is_empty() && !result.contains(&provider) {
            result.push(provider);
        }
    }
    result
}

/// Merge optional value with custom merger
fn merge_option<T>(
    base: Option<T>,
    overlay: Option<T>,
    strategy: MergeStrategy,
    merger: impl FnOnce(T, T, MergeStrategy) -> Result<T, String>,
) -> Result<Option<T>, String> {
    match (base, overlay) {
        (Some(b), Some(o)) => merger(b, o, strategy).map(Some),
        (Some(b), None) => Ok(Some(b)),
        (None, Some(o)) => Ok(Some(o)),
        (None, None) => Ok(None),
    }
}

/// Merge SSH transports
fn merge_transports(
    base: RegistryNodeTransports,
    overlay: RegistryNodeTransports,
    strategy: MergeStrategy,
) -> Result<RegistryNodeTransports, String> {
    let ssh = merge_option(base.ssh, overlay.ssh, strategy, merge_ssh_transport)?;
    Ok(RegistryNodeTransports { ssh })
}

fn merge_ssh_transport(
    base: RegistrySshTransport,
    overlay: RegistrySshTransport,
    strategy: MergeStrategy,
) -> Result<RegistrySshTransport, String> {
    let target = merge_field(base.target, overlay.target, strategy, "ssh.target")?;
    let trust = merge_field(base.trust, overlay.trust, strategy, "ssh.trust")?;
    let enrollment_authority = merge_option(
        base.enrollment_authority,
        overlay.enrollment_authority,
        strategy,
        merge_enrollment_authority,
    )?;
    Ok(RegistrySshTransport {
        target,
        trust,
        enrollment_authority,
    })
}

fn merge_enrollment_authority(
    base: RegistrySshEnrollmentAuthority,
    overlay: RegistrySshEnrollmentAuthority,
    strategy: MergeStrategy,
) -> Result<RegistrySshEnrollmentAuthority, String> {
    let class = merge_field(base.class, overlay.class, strategy, "enrollment.class")?;
    let principal = merge_field(
        base.principal,
        overlay.principal,
        strategy,
        "enrollment.principal",
    )?;
    let credential_ref = merge_field(
        base.credential_ref,
        overlay.credential_ref,
        strategy,
        "enrollment.credential_ref",
    )?;
    Ok(RegistrySshEnrollmentAuthority {
        class,
        principal,
        credential_ref,
    })
}

/// Merge substrate
fn merge_substrate(
    base: crate::registry::types::RegistryNodeSubstrate,
    overlay: crate::registry::types::RegistryNodeSubstrate,
    strategy: MergeStrategy,
) -> Result<crate::registry::types::RegistryNodeSubstrate, String> {
    let family = merge_field(base.family, overlay.family, strategy, "substrate.family")?;
    Ok(crate::registry::types::RegistryNodeSubstrate { family })
}

/// Merge services
fn merge_services(
    mut base: Vec<RegistryNodeService>,
    overlay: Vec<RegistryNodeService>,
    strategy: MergeStrategy,
) -> Vec<RegistryNodeService> {
    for overlay_service in overlay {
        let match_idx = base
            .iter()
            .position(|base_service| services_match(base_service, &overlay_service));

        match match_idx {
            Some(idx) => {
                base[idx] = merge_service(base[idx].clone(), overlay_service, strategy);
            }
            None => {
                base.push(overlay_service);
            }
        }
    }
    base
}

fn services_match(a: &RegistryNodeService, b: &RegistryNodeService) -> bool {
    // Match by unit name (most stable)
    match (&a.unit, &b.unit) {
        (Some(u_a), Some(u_b)) if !u_a.is_empty() && u_a == u_b => true,
        // Fallback to name
        (Some(n_a), Some(n_b)) if !n_a.is_empty() && n_a == n_b => true,
        _ => false,
    }
}

fn merge_service(
    base: RegistryNodeService,
    overlay: RegistryNodeService,
    strategy: MergeStrategy,
) -> RegistryNodeService {
    let name = merge_field_opt(base.name, overlay.name, strategy);
    let artifact = merge_field_opt(base.artifact, overlay.artifact, strategy);
    let port = merge_field_opt(base.port, overlay.port, strategy);
    let unit = merge_field_opt(base.unit, overlay.unit, strategy);
    let supervisor = merge_option(
        base.supervisor,
        overlay.supervisor,
        strategy,
        merge_supervisor,
    )
    .ok()
    .flatten();
    let health = merge_option(base.health, overlay.health, strategy, merge_health)
        .ok()
        .flatten();

    RegistryNodeService {
        name,
        artifact,
        port,
        unit,
        supervisor,
        health,
    }
}

fn merge_field_opt<T: Clone>(
    base: Option<T>,
    overlay: Option<T>,
    strategy: MergeStrategy,
) -> Option<T> {
    merge_field(base, overlay, strategy, "").ok().flatten()
}

fn merge_supervisor(
    base: RegistryServiceSupervisor,
    overlay: RegistryServiceSupervisor,
    strategy: MergeStrategy,
) -> Result<RegistryServiceSupervisor, String> {
    let provider = merge_field(
        base.provider,
        overlay.provider,
        strategy,
        "supervisor.provider",
    )?;
    Ok(RegistryServiceSupervisor { provider })
}

fn merge_health(
    base: RegistryServiceHealth,
    overlay: RegistryServiceHealth,
    strategy: MergeStrategy,
) -> Result<RegistryServiceHealth, String> {
    let kind = merge_field(base.kind, overlay.kind, strategy, "health.kind")?;
    let port = merge_field_opt(base.port, overlay.port, strategy);
    let process = merge_field_opt(base.process, overlay.process, strategy);
    Ok(RegistryServiceHealth {
        kind,
        port,
        process,
    })
}

/// Merge network links
fn merge_network_links(
    mut base: Vec<RegistryNetworkLink>,
    overlay: Vec<RegistryNetworkLink>,
    _strategy: MergeStrategy,
) -> Vec<RegistryNetworkLink> {
    for overlay_link in overlay {
        let match_idx = base
            .iter()
            .position(|base_link| network_links_match(base_link, &overlay_link));

        match match_idx {
            Some(idx) => {
                base[idx] = merge_network_link(base[idx].clone(), overlay_link);
            }
            None => {
                base.push(overlay_link);
            }
        }
    }
    base
}

fn network_links_match(a: &RegistryNetworkLink, b: &RegistryNetworkLink) -> bool {
    match (&a.interface, &b.interface) {
        (Some(i_a), Some(i_b)) if !i_a.is_empty() && i_a == i_b => true,
        _ => false,
    }
}

fn merge_network_link(
    base: RegistryNetworkLink,
    overlay: RegistryNetworkLink,
) -> RegistryNetworkLink {
    RegistryNetworkLink {
        plane: overlay.plane.or(base.plane),
        provider: overlay.provider.or(base.provider),
        interface: overlay.interface.or(base.interface),
        connection: overlay.connection.or(base.connection),
        address: overlay.address.or(base.address),
        gateway: overlay.gateway.or(base.gateway),
        routes: {
            let mut routes = base.routes;
            for route in overlay.routes {
                if !routes.contains(&route) {
                    routes.push(route);
                }
            }
            routes
        },
        default_route: overlay.default_route.or(base.default_route),
    }
}

/// Merge network policy
fn merge_network_policy(
    base: Option<RegistryNetworkPolicy>,
    overlay: Option<RegistryNetworkPolicy>,
    strategy: MergeStrategy,
) -> Option<RegistryNetworkPolicy> {
    merge_option(base, overlay, strategy, |mut b, o, s| {
        for (key, plane) in o.planes {
            match b.planes.get(&key) {
                Some(_) => match s {
                    MergeStrategy::KeepBase => {}
                    MergeStrategy::TakeOverlay => {
                        b.planes.insert(key, plane);
                    }
                    MergeStrategy::Fail => {
                        return Err(format!("network policy plane conflict: {}", key));
                    }
                },
                None => {
                    b.planes.insert(key, plane);
                }
            }
        }
        Ok(b)
    })
    .ok()
    .flatten()
}

/// Merge lifecycle
fn merge_lifecycle(
    base: Option<RegistryLifecycle>,
    overlay: Option<RegistryLifecycle>,
    strategy: MergeStrategy,
) -> Option<RegistryLifecycle> {
    merge_option(base, overlay, strategy, |b, o, s| {
        let kernel_release =
            merge_option(b.kernel_release, o.kernel_release, s, merge_kernel_release)?;
        Ok(RegistryLifecycle { kernel_release })
    })
    .ok()
    .flatten()
}

fn merge_kernel_release(
    base: RegistryKernelRelease,
    overlay: RegistryKernelRelease,
    strategy: MergeStrategy,
) -> Result<RegistryKernelRelease, String> {
    let build_runner_role = merge_field(
        base.build_runner_role,
        overlay.build_runner_role,
        strategy,
        "kernel_release.build_runner_role",
    )?;
    let host_bootstrap = merge_option(
        base.host_bootstrap,
        overlay.host_bootstrap,
        strategy,
        merge_host_bootstrap,
    )?;
    Ok(RegistryKernelRelease {
        build_runner_role,
        host_bootstrap,
    })
}

fn merge_host_bootstrap(
    base: crate::registry::types::RegistryHostBootstrap,
    overlay: crate::registry::types::RegistryHostBootstrap,
    strategy: MergeStrategy,
) -> Result<crate::registry::types::RegistryHostBootstrap, String> {
    let mut role_profiles = base.role_profiles;
    for (role, profile) in overlay.role_profiles {
        match role_profiles.get(&role) {
            Some(_) => match strategy {
                MergeStrategy::KeepBase => {}
                MergeStrategy::TakeOverlay => {
                    role_profiles.insert(role, profile);
                }
                MergeStrategy::Fail => {
                    return Err(format!("host_bootstrap role conflict: {}", role));
                }
            },
            None => {
                role_profiles.insert(role, profile);
            }
        }
    }
    Ok(crate::registry::types::RegistryHostBootstrap { role_profiles })
}

/// Merge fleet desired
fn merge_fleet_desired(
    base: Option<RegistryFleetDesired>,
    overlay: Option<RegistryFleetDesired>,
    strategy: MergeStrategy,
) -> Option<RegistryFleetDesired> {
    merge_option(base, overlay, strategy, |mut b, o, s| {
        for overlay_target in o.targets {
            let match_idx = b
                .targets
                .iter()
                .position(|base_target| base_target.id == overlay_target.id);

            match match_idx {
                Some(idx) => {
                    b.targets[idx] = merge_fleet_target(b.targets[idx].clone(), overlay_target, s)?;
                }
                None => {
                    b.targets.push(overlay_target);
                }
            }
        }
        Ok(b)
    })
    .ok()
    .flatten()
}

fn merge_fleet_target(
    base: RegistryFleetTarget,
    overlay: RegistryFleetTarget,
    strategy: MergeStrategy,
) -> Result<RegistryFleetTarget, String> {
    let id = merge_field(base.id, overlay.id, strategy, "fleet_target.id")?;
    let provider = merge_field(
        base.provider,
        overlay.provider,
        strategy,
        "fleet_target.provider",
    )?;
    let resource_id = merge_field(
        base.resource_id,
        overlay.resource_id,
        strategy,
        "fleet_target.resource_id",
    )?;
    let role = merge_field(base.role, overlay.role, strategy, "fleet_target.role")?;
    let desired_power = merge_field(
        base.desired_power,
        overlay.desired_power,
        strategy,
        "fleet_target.desired_power",
    )?;
    let provisioning = merge_option(
        base.provisioning,
        overlay.provisioning,
        strategy,
        |a, b, s| {
            // For provisioning spec, use the same strategy
            merge_field(Some(a), Some(b), s, "provisioning")
                .map_err(|e| e)
                .and_then(|opt| opt.ok_or_else(|| "provisioning merge returned None".to_string()))
        },
    )?;

    Ok(RegistryFleetTarget {
        id,
        provider,
        resource_id,
        role,
        desired_power,
        provisioning,
    })
}

fn merge_field_opt_inner<T: Clone>(
    base: Option<T>,
    overlay: Option<T>,
    strategy: MergeStrategy,
) -> Result<Option<T>, String> {
    merge_field(base, overlay, strategy, "").map_err(|e| e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::types::{
        RegistryDocument, RegistryFleetDesired, RegistryFleetTarget, RegistryKernelRelease,
        RegistryLifecycle, RegistryNetworkLink, RegistryNetworkPlane, RegistryNetworkPolicy,
        RegistryNode, RegistryNodeService, RegistryNodeTransports, RegistryServiceHealth,
        RegistryServiceSupervisor, RegistrySshEnrollmentAuthority, RegistrySshTransport,
    };
    use std::collections::BTreeMap;

    fn minimal_node(name: &str) -> RegistryNode {
        RegistryNode {
            name: Some(name.to_string()),
            id: Some(name.to_string()),
            aliases: vec![],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        }
    }

    fn minimal_doc() -> RegistryDocument {
        RegistryDocument {
            network_policy: None,
            lifecycle: None,
            nodes: vec![minimal_node("test-node")],
            fleet_desired: None,
        }
    }

    #[test]
    fn merges_disjoint_nodes() {
        let base = minimal_doc();
        let mut overlay = minimal_doc();
        overlay.nodes[0].name = Some("other-node".to_string());
        overlay.nodes[0].id = Some("other-node".to_string());

        let merged = merge_registry(base, overlay, MergeStrategy::TakeOverlay).unwrap();
        assert_eq!(merged.nodes.len(), 2);
    }

    #[test]
    fn merges_same_node_keep_base() {
        let base = minimal_doc();
        let mut overlay = minimal_doc();
        overlay.nodes[0].role = Some("hypervisor".to_string());

        let merged = merge_registry(base, overlay, MergeStrategy::KeepBase).unwrap();
        assert_eq!(merged.nodes[0].role.as_deref(), Some("kernel_primary"));
    }

    #[test]
    fn merges_same_node_take_overlay() {
        let base = minimal_doc();
        let mut overlay = minimal_doc();
        overlay.nodes[0].role = Some("hypervisor".to_string());

        let merged = merge_registry(base, overlay, MergeStrategy::TakeOverlay).unwrap();
        assert_eq!(merged.nodes[0].role.as_deref(), Some("hypervisor"));
    }

    #[test]
    fn fails_on_conflict() {
        let base = minimal_doc();
        let mut overlay = minimal_doc();
        overlay.nodes[0].role = Some("hypervisor".to_string());

        let err = merge_registry(base, overlay, MergeStrategy::Fail).unwrap_err();
        assert!(err.contains("conflict"));
    }

    #[test]
    fn merges_aliases_union() {
        let base = RegistryNode {
            name: Some("node".to_string()),
            id: Some("node".to_string()),
            aliases: vec!["alias1".to_string()],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        };
        let overlay = RegistryNode {
            name: Some("node".to_string()),
            id: Some("node".to_string()),
            aliases: vec!["alias2".to_string()],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        };

        let merged = merge_node(base, overlay, MergeStrategy::TakeOverlay).unwrap();
        assert!(merged.aliases.contains(&"alias1".to_string()));
        assert!(merged.aliases.contains(&"alias2".to_string()));
    }

    #[test]
    fn merges_network_links_by_interface() {
        let base = RegistryNetworkLink {
            plane: Some("management".to_string()),
            provider: Some("network-manager".to_string()),
            interface: Some("eth0".to_string()),
            connection: Some("conn1".to_string()),
            address: Some("192.168.1.1/24".to_string()),
            gateway: Some("192.168.1.1".to_string()),
            routes: vec!["10.0.0.0/8".to_string()],
            default_route: Some(false),
        };
        let overlay = RegistryNetworkLink {
            plane: Some("management".to_string()),
            provider: Some("network-manager".to_string()),
            interface: Some("eth0".to_string()),
            connection: Some("conn2".to_string()),
            address: Some("10.0.0.1/24".to_string()),
            gateway: Some("10.0.0.1".to_string()),
            routes: vec!["172.16.0.0/12".to_string()],
            default_route: Some(true),
        };

        let merged = merge_network_link(base, overlay);
        assert_eq!(merged.interface.as_deref(), Some("eth0"));
        assert_eq!(merged.connection.as_deref(), Some("conn2")); // overlay wins
        assert_eq!(merged.address.as_deref(), Some("10.0.0.1/24")); // overlay wins
        assert!(merged.routes.contains(&"10.0.0.0/8".to_string()));
        assert!(merged.routes.contains(&"172.16.0.0/12".to_string()));
        assert_eq!(merged.default_route, Some(true)); // overlay wins
    }
}
