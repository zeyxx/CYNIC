//! Registry validation — pure functions, no I/O

use crate::domain::services::ServiceManagerKind;
use crate::registry::types::{RegistryDocument, RegistryNetworkLink, RegistryNode};

/// Validation error with context
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("duplicate node id: {id}")]
    DuplicateNodeId { id: String },
    #[error("duplicate node name: {name}")]
    DuplicateNodeName { name: String },
    #[error("node {name} has empty id and no aliases")]
    NodeMissingIdentity { name: String },
    #[error("node {name} references unknown provider: {provider}")]
    UnknownProvider { name: String, provider: String },
    #[error("node {name} has duplicate alias: {alias}")]
    DuplicateAlias { name: String, alias: String },
    #[error("network link on node {node} has empty required field: {field}")]
    EmptyNetworkLinkField { node: String, field: String },
    #[error("service {service} on node {node} has empty unit name")]
    EmptyServiceUnit { node: String, service: String },
    #[error("fleet target {id} references missing node")]
    FleetTargetMissingNode { id: String },
    #[error("build_runner_role {role} references missing node")]
    BuildRunnerRoleMissingNode { role: String },
    #[error("host_bootstrap role {role} not found in nodes")]
    HostBootstrapRoleMissing { role: String },
}

/// Validate registry document — returns all errors found
pub fn validate_registry(doc: &RegistryDocument) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Collect node names/ids/aliases for cross-references
    let mut node_ids = std::collections::HashSet::new();
    let mut node_names = std::collections::HashSet::new();
    let mut all_aliases = std::collections::HashSet::new();
    let mut providers = std::collections::HashSet::new();

    for node in &doc.nodes {
        // Check duplicate IDs
        if let Some(id) = &node.id {
            if !id.is_empty() {
                if !node_ids.insert(id.clone()) {
                    errors.push(ValidationError::DuplicateNodeId { id: id.clone() });
                }
            }
        }

        // Check duplicate names
        if let Some(name) = &node.name {
            if !name.is_empty() {
                if !node_names.insert(name.clone()) {
                    errors.push(ValidationError::DuplicateNodeName { name: name.clone() });
                }
            }
        }

        // Check node has identity
        let has_id = node.id.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let has_name = node.name.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let has_alias = !node.aliases.is_empty();
        if !has_id && !has_name && !has_alias {
            let name = node.name.clone().unwrap_or_else(|| "unnamed".to_string());
            errors.push(ValidationError::NodeMissingIdentity { name });
        }

        // Check duplicate aliases
        for alias in &node.aliases {
            if !alias.is_empty() {
                let key = format!("{}::{}", node.name.as_deref().unwrap_or("unnamed"), alias);
                if !all_aliases.insert(key) {
                    errors.push(ValidationError::DuplicateAlias {
                        name: node.name.clone().unwrap_or_else(|| "unnamed".to_string()),
                        alias: alias.clone(),
                    });
                }
            }
        }

        // Collect providers
        for provider in &node.provider_controls {
            providers.insert(provider.clone());
        }

        // Validate network links
        if let Some(name) = node.name.as_deref().or(node.id.as_deref()) {
            for link in &node.network_links {
                validate_network_link(name, link, &mut errors);
            }
        }

        // Validate services
        if let Some(name) = node.name.as_deref().or(node.id.as_deref()) {
            for service in &node.services {
                if let Some(unit) = service.unit.as_deref() {
                    if unit.trim().is_empty() {
                        let service_name = service.name.as_deref().unwrap_or("unnamed");
                        errors.push(ValidationError::EmptyServiceUnit {
                            node: name.to_string(),
                            service: service_name.to_string(),
                        });
                    }
                }
            }
        }
    }

    // Validate fleet_desired targets reference existing nodes
    if let Some(fleet) = &doc.fleet_desired {
        for target in &fleet.targets {
            if let Some(id) = &target.id {
                let exists = doc.nodes.iter().any(|node| {
                    node.id.as_deref() == Some(id.as_str())
                        || node.name.as_deref() == Some(id.as_str())
                        || node.aliases.iter().any(|a| a == id)
                        || node.role.as_deref() == Some(id.as_str())
                        || node.canonical_role.as_deref() == Some(id.as_str())
                });
                if !exists {
                    errors.push(ValidationError::FleetTargetMissingNode { id: id.clone() });
                }
            }
        }
    }

    // Validate build_runner_role references existing node
    if let Some(lifecycle) = &doc.lifecycle {
        if let Some(release) = &lifecycle.kernel_release {
            if let Some(role) = &release.build_runner_role {
                let exists = doc.nodes.iter().any(|node| {
                    node.role.as_deref() == Some(role.as_str())
                        || node.canonical_role.as_deref() == Some(role.as_str())
                });
                if !exists {
                    errors.push(ValidationError::BuildRunnerRoleMissingNode { role: role.clone() });
                }
            }

            // Validate host_bootstrap roles
            if let Some(bootstrap) = &release.host_bootstrap {
                for role in bootstrap.role_profiles.keys() {
                    let exists = doc.nodes.iter().any(|node| {
                        node.role.as_deref() == Some(role.as_str())
                            || node.canonical_role.as_deref() == Some(role.as_str())
                    });
                    if !exists {
                        errors
                            .push(ValidationError::HostBootstrapRoleMissing { role: role.clone() });
                    }
                }
            }
        }
    }

    errors
}

/// Validate a single network link
fn validate_network_link(
    node_name: &str,
    link: &RegistryNetworkLink,
    errors: &mut Vec<ValidationError>,
) {
    let mut check_field = |field_name: &str, value: &Option<String>| {
        if let Some(v) = value {
            if v.trim().is_empty() {
                errors.push(ValidationError::EmptyNetworkLinkField {
                    node: node_name.to_string(),
                    field: field_name.to_string(),
                });
            }
        }
    };

    check_field("plane", &link.plane);
    check_field("provider", &link.provider);
    check_field("interface", &link.interface);
    check_field("connection", &link.connection);
    check_field("address", &link.address);
    check_field("gateway", &link.gateway);
}

/// Validate registry and return Result
pub fn validate_registry_or_error(doc: &RegistryDocument) -> Result<(), Vec<ValidationError>> {
    let errors = validate_registry(doc);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check role consistency across registry
pub fn check_role_consistency(doc: &RegistryDocument) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for roles without nodes
    let mut roles_with_nodes = std::collections::HashSet::new();
    for node in &doc.nodes {
        if let Some(role) = &node.role {
            roles_with_nodes.insert(role.clone());
        }
        if let Some(role) = &node.canonical_role {
            roles_with_nodes.insert(role.clone());
        }
    }

    // Check fleet_desired roles have nodes
    if let Some(fleet) = &doc.fleet_desired {
        for target in &fleet.targets {
            if let Some(role) = &target.role {
                if !roles_with_nodes.contains(role) {
                    warnings.push(format!(
                        "fleet target role '{}' has no matching node in registry",
                        role
                    ));
                }
            }
        }
    }

    // Check lifecycle roles
    if let Some(lifecycle) = &doc.lifecycle {
        if let Some(release) = &lifecycle.kernel_release {
            if let Some(role) = &release.build_runner_role {
                if !roles_with_nodes.contains(role) {
                    warnings.push(format!(
                        "build_runner_role '{}' has no matching node in registry",
                        role
                    ));
                }
            }
        }
    }

    warnings
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
    fn passes_valid_registry() {
        let doc = minimal_doc();
        assert!(validate_registry(&doc).is_empty());
    }

    #[test]
    fn detects_duplicate_node_ids() {
        let mut doc = minimal_doc();
        doc.nodes.push(RegistryNode {
            name: Some("other".to_string()),
            id: Some("test-node".to_string()), // duplicate
            aliases: vec![],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        });
        let errors = validate_registry(&doc);
        assert!(matches!(
            errors.first(),
            Some(ValidationError::DuplicateNodeId { .. })
        ));
    }

    #[test]
    fn detects_duplicate_node_names() {
        let mut doc = minimal_doc();
        doc.nodes.push(RegistryNode {
            name: Some("test-node".to_string()), // duplicate
            id: Some("other".to_string()),
            aliases: vec![],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        });
        let errors = validate_registry(&doc);
        assert!(matches!(
            errors.first(),
            Some(ValidationError::DuplicateNodeName { .. })
        ));
    }

    #[test]
    fn detects_node_without_identity() {
        let mut doc = minimal_doc();
        doc.nodes.push(RegistryNode {
            name: None,
            id: None,
            aliases: vec![],
            role: Some("kernel_primary".to_string()),
            canonical_role: None,
            provider_controls: vec![],
            transports: None,
            host_family: None,
            substrate: None,
            services: vec![],
            network_links: vec![],
        });
        let errors = validate_registry(&doc);
        assert!(matches!(
            errors.first(),
            Some(ValidationError::NodeMissingIdentity { .. })
        ));
    }

    #[test]
    fn validates_network_link_fields() {
        let mut doc = minimal_doc();
        doc.nodes[0].network_links.push(RegistryNetworkLink {
            plane: Some("management".to_string()),
            provider: Some("network-manager".to_string()),
            interface: Some("".to_string()), // empty!
            connection: Some("conn".to_string()),
            address: Some("192.168.1.1/24".to_string()),
            gateway: Some("192.168.1.1".to_string()),
            routes: vec![],
            default_route: None,
        });
        let errors = validate_registry(&doc);
        assert!(matches!(
            errors.first(),
            Some(ValidationError::EmptyNetworkLinkField { field, .. }) if field == "interface"
        ));
    }

    #[test]
    fn validates_fleet_target_references_existing_node() {
        let mut doc = minimal_doc();
        doc.fleet_desired = Some(RegistryFleetDesired {
            targets: vec![RegistryFleetTarget {
                id: Some("nonexistent".to_string()),
                provider: Some("provider".to_string()),
                resource_id: Some("res".to_string()),
                role: Some("kernel_primary".to_string()),
                desired_power: Some("running".to_string()),
                provisioning: None,
            }],
        });
        let errors = validate_registry(&doc);
        assert!(matches!(
            errors.first(),
            Some(ValidationError::FleetTargetMissingNode { .. })
        ));
    }

    #[test]
    fn validates_build_runner_role() {
        let mut doc = minimal_doc();
        doc.lifecycle = Some(RegistryLifecycle {
            kernel_release: Some(RegistryKernelRelease {
                build_runner_role: Some("nonexistent_role".to_string()),
                host_bootstrap: None,
            }),
        });
        let errors = validate_registry(&doc);
        assert!(matches!(
            errors.first(),
            Some(ValidationError::BuildRunnerRoleMissingNode { .. })
        ));
    }
}
