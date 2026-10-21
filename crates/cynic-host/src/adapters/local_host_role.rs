//! Resolve the local host's fleet role from the registry.
//!
//! WHY THIS EXISTS.
//!
//! The deny-by-default execution policy needs to know whether the host
//! asking to run a program is allowed to compile. That fact is already
//! declared: `infra/registry.json` gives every node a `role`, and the
//! builder carries the `build_runner` role. This adapter is the bridge
//! from the declared registry to the pure policy decision.
//!
//! The registry is the single source of truth. A host that is not in the
//! registry, or whose role is not understood, resolves to `Host`: it may
//! not compile. Failing closed here is what makes the local-build
//! refusal structural rather than advisory.
//!
//! This adapter reads the registry and the observed hostname. It performs
//! no execution.

use cynic_protocol::execution_policy::HostRole;

use crate::registry::{
    RegistryDocument, RegistryNode, load_registry, resolve_registry_path,
};

/// The local host's fleet role, resolved from the registry.
///
/// Returns `Host` when the host cannot be found, when it carries no
/// role, or when its role is not a role this policy understands. The
/// refusal is a normal outcome: a host outside the registry is not a
/// build runner.
pub fn local_host_role(registry_path: Option<&str>) -> HostRole {
    local_host_role_from(registry_path, &crate::runtime_layout::observed_hostname())
}

/// The role resolution with the hostname supplied, so the decision is
/// testable without touching the host.
pub fn local_host_role_from(registry_path: Option<&str>, hostname: &str) -> HostRole {
    if hostname.trim().is_empty() || hostname == "unknown" {
        return HostRole::Host;
    }
    let path = match resolve_registry_path(registry_path) {
        Ok(path) => path,
        Err(_) => return HostRole::Host,
    };
    let registry = match load_registry(&path) {
        Ok(registry) => registry,
        Err(_) => return HostRole::Host,
    };
    role_of_host(&registry, hostname)
}

/// The role of the node matching the hostname. A node matches when the
/// registry names it by name, by id, or by alias — the same identity the
/// registry module uses when it resolves a transport.
pub fn role_of_host(registry: &RegistryDocument, hostname: &str) -> HostRole {
    let node = registry
        .nodes
        .iter()
        .find(|node| node_matches(node, hostname));
    let Some(node) = node else {
        return HostRole::Host;
    };
    let role = node
        .role
        .as_deref()
        .or(node.canonical_role.as_deref())
        .unwrap_or("");
    HostRole::parse(role)
}

/// Whether the registry names this node by the requested identity.
fn node_matches(node: &RegistryNode, requested: &str) -> bool {
    node.name.as_deref() == Some(requested)
        || node.id.as_deref() == Some(requested)
        || node.aliases.iter().any(|alias| alias == requested)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RegistryNode;

    fn node(name: &str, role: Option<&str>) -> RegistryNode {
        RegistryNode {
            name: Some(name.to_string()),
            id: Some(name.to_string()),
            aliases: Vec::new(),
            role: role.map(|role| role.to_string()),
            canonical_role: None,
            provider_controls: Vec::new(),
            transports: None,
            host_family: None,
            substrate: None,
            services: Vec::new(),
            network_links: Vec::new(),
            source_workspace_layout: None,
        }
    }

    fn document(nodes: Vec<RegistryNode>) -> RegistryDocument {
        RegistryDocument {
            network_policy: None,
            lifecycle: None,
            nodes,
            fleet_desired: None,
        }
    }

    #[test]
    fn a_builder_node_resolves_to_the_build_runner_role() {
        let registry = document(vec![node("cynic-builder", Some("build_runner"))]);
        assert_eq!(
            role_of_host(&registry, "cynic-builder"),
            HostRole::BuildRunner
        );
    }

    #[test]
    fn a_host_node_cannot_compile() {
        let registry = document(vec![node("cortex", Some("cortex_host"))]);
        assert_eq!(role_of_host(&registry, "cortex"), HostRole::Host);
    }

    #[test]
    fn a_node_without_a_role_cannot_compile() {
        let registry = document(vec![node("mystery", None)]);
        assert_eq!(role_of_host(&registry, "mystery"), HostRole::Host);
    }

    #[test]
    fn an_unknown_host_cannot_compile() {
        let registry = document(vec![node("cynic-builder", Some("build_runner"))]);
        assert_eq!(role_of_host(&registry, "somewhere-else"), HostRole::Host);
    }

    #[test]
    fn an_alias_resolves_to_the_node_role() {
        let mut builder = node("cynic-builder", Some("build_runner"));
        builder.aliases = vec!["builder".to_string()];
        let registry = document(vec![builder]);
        assert_eq!(role_of_host(&registry, "builder"), HostRole::BuildRunner);
    }
}
