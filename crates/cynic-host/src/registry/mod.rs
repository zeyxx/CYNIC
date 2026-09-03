//! Registry module — split into focused submodules
//!
//! - `types`: Pure data structures (no I/O, no policy)
//! - `scan`: Discovery and loading (read-only)
//! - `validate`: Pure validation functions
//! - `persist`: I/O operations with atomic write + read-back proof

pub mod persist;
pub mod scan;
pub mod types;
pub mod validate;

// Re-export main public API
pub use crate::runtime_registry::DEFAULT_REGISTRY_PATH;
pub use persist::{
    PersistError, WriteProof, load_and_validate_registry, write_registry, write_registry_atomic,
};
pub use scan::{
    ScanError, discover_registry_sources, find_node, load_registry, resolve_registry_path,
};
pub use types::{
    HostProfileResolution, ObservedManagementLink, RegistryDocument, RegistryFleetDesired,
    RegistryFleetTarget, RegistryHostBootstrap, RegistryHostBootstrapRoleProfile,
    RegistryKernelRelease, RegistryLifecycle, RegistryNetworkLink, RegistryNetworkPlane,
    RegistryNetworkPolicy, RegistryNode, RegistryNodeService, RegistryNodeSubstrate,
    RegistryNodeTransports, RegistryServiceHealth, RegistryServiceSupervisor,
    RegistrySshEnrollmentAuthority, RegistrySshTransport, ResolvedBuildRunnerTransport,
    ResolvedDirectManagementLink, ResolvedHostIdentity, ResolvedNodeSubstrateTarget,
    ResolvedProviderControlTransport,
};
pub use validate::{
    ValidationError, check_role_consistency, validate_registry, validate_registry_or_error,
};

// Keep existing public functions for backward compatibility
pub use crate::registry::resolve::{
    resolve_build_runner_transport, resolve_current_host_direct_management_link,
    resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_current_registry_service_policy, resolve_framework_transport,
    resolve_host_direct_management_link, resolve_host_profile, resolve_host_profile_resolution,
    resolve_node_substrate_desired_state, resolve_node_substrate_targets,
    resolve_observed_direct_management_link, resolve_provider_control_transport,
    resolve_registry_service_policy,
};

pub mod resolve {
    use super::*;
    use crate::profiles::HostProfile;
    use crate::runtime_layout::current_runtime_layout;
    use crate::runtime_registry::resolve_runtime_registry_path;
    use cynic_protocol::{NodeSubstrateDesiredState, NodeSubstratePower};
    use std::path::{Path, PathBuf};

    // Re-export the resolution functions from the original registry.rs
    // These will be moved here in the next step

    fn resolve_registry_path(explicit_path: Option<&str>) -> Result<PathBuf, String> {
        resolve_runtime_registry_path(explicit_path.map(Path::new))
    }

    fn current_hostname() -> Result<String, String> {
        let trimmed = crate::runtime_layout::observed_hostname();
        if trimmed.is_empty() {
            Err("hostname resolution returned empty output".to_string())
        } else {
            Ok(trimmed)
        }
    }

    fn load_registry(path: &Path) -> Result<RegistryDocument, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read registry {}: {error}", path.display()))?;
        serde_json::from_str(&raw)
            .map_err(|error| format!("failed to parse registry {}: {error}", path.display()))
    }

    pub fn resolve_node_substrate_targets(
        registry_path: Option<&str>,
    ) -> Result<Vec<ResolvedNodeSubstrateTarget>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        resolve_node_substrate_targets_from_registry(&registry)
    }

    fn resolve_node_substrate_targets_from_registry(
        registry: &RegistryDocument,
    ) -> Result<Vec<ResolvedNodeSubstrateTarget>, String> {
        let targets = registry
            .fleet_desired
            .as_ref()
            .map(|fleet| fleet.targets.as_slice())
            .unwrap_or_default();
        let mut resolved = Vec::with_capacity(targets.len());
        for target in targets {
            let target_id =
                required_registry_value(target.id.as_deref(), "fleet_target_id_missing")?;
            let role =
                required_registry_value(target.role.as_deref(), "fleet_target_role_missing")?;
            let nodes = registry
                .nodes
                .iter()
                .filter(|node| {
                    node.id.as_deref() == Some(target_id.as_str())
                        || node.name.as_deref() == Some(target_id.as_str())
                        || node.aliases.iter().any(|alias| alias == &target_id)
                        || node.role.as_deref() == Some(role.as_str())
                        || node.canonical_role.as_deref() == Some(role.as_str())
                })
                .collect::<Vec<_>>();
            let [node] = nodes.as_slice() else {
                return Err(if nodes.is_empty() {
                    format!("node_substrate_target_node_missing:{target_id}")
                } else {
                    format!("node_substrate_target_node_ambiguous:{target_id}")
                });
            };
            let node_id = node
                .id
                .clone()
                .or_else(|| node.name.clone())
                .ok_or_else(|| format!("node_substrate_target_node_id_missing:{target_id}"))?;
            resolved.push(ResolvedNodeSubstrateTarget {
                target_id,
                role,
                desired: NodeSubstrateDesiredState {
                    node_id,
                    provider: target.provider.clone(),
                    resource_id: target.resource_id.clone(),
                    desired_power: target
                        .desired_power
                        .as_deref()
                        .map(parse_desired_power)
                        .unwrap_or(NodeSubstratePower::Unknown),
                    provisioning: target.provisioning.clone(),
                },
            });
        }
        Ok(resolved)
    }

    fn required_registry_value(value: Option<&str>, blocker: &str) -> Result<String, String> {
        value
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| blocker.to_string())
    }

    pub fn resolve_provider_control_transport(
        registry_path: Option<&str>,
        provider: &str,
    ) -> Result<ResolvedProviderControlTransport, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let candidates = registry
            .nodes
            .iter()
            .filter(|node| {
                node.provider_controls
                    .iter()
                    .any(|candidate| candidate == provider)
            })
            .collect::<Vec<_>>();
        let [node] = candidates.as_slice() else {
            return Err(if candidates.is_empty() {
                "provider_control_node_missing".to_string()
            } else {
                "provider_control_node_ambiguous".to_string()
            });
        };
        let node_id = node
            .id
            .clone()
            .or_else(|| node.name.clone())
            .ok_or_else(|| "provider_control_node_id_missing".to_string())?;
        let ssh = node
            .transports
            .as_ref()
            .and_then(|transports| transports.ssh.as_ref())
            .ok_or_else(|| "provider_control_transport_missing".to_string())?;
        let ssh_target = ssh
            .target
            .clone()
            .filter(|target| !target.trim().is_empty() && !target.starts_with('-'))
            .ok_or_else(|| "provider_control_ssh_target_missing".to_string())?;
        let trust = ssh
            .trust
            .clone()
            .filter(|trust| trust == "ssh_trust_anchor")
            .ok_or_else(|| "provider_control_ssh_trust_unproven".to_string())?;
        Ok(ResolvedProviderControlTransport {
            provider: provider.to_string(),
            node_id,
            ssh_target,
            trust,
            registry_path: path.display().to_string(),
        })
    }

    pub fn resolve_node_substrate_desired_state(
        registry_path: Option<&str>,
        node_id: &str,
    ) -> Result<NodeSubstrateDesiredState, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| {
                node.id.as_deref() == Some(node_id)
                    || node.name.as_deref() == Some(node_id)
                    || node.aliases.iter().any(|alias| alias == node_id)
            })
            .ok_or_else(|| "node_substrate_registry_node_missing".to_string())?;
        let canonical_node_id = node
            .id
            .clone()
            .or_else(|| node.name.clone())
            .ok_or_else(|| "node_substrate_registry_node_id_missing".to_string())?;
        let targets = resolve_node_substrate_targets_from_registry(&registry)?
            .into_iter()
            .filter(|target| target.desired.node_id == canonical_node_id)
            .collect::<Vec<_>>();
        let target = match targets.as_slice() {
            [] => None,
            [target] => Some(target),
            _ => return Err("node_substrate_fleet_target_ambiguous".to_string()),
        };
        Ok(NodeSubstrateDesiredState {
            node_id: canonical_node_id,
            provider: target.and_then(|target| target.desired.provider.clone()),
            resource_id: target.and_then(|target| target.desired.resource_id.clone()),
            desired_power: target
                .map(|target| target.desired.desired_power.clone())
                .unwrap_or(NodeSubstratePower::Unknown),
            provisioning: target.and_then(|target| target.desired.provisioning.clone()),
        })
    }

    fn parse_desired_power(value: &str) -> NodeSubstratePower {
        match value {
            "running" => NodeSubstratePower::Running,
            "stopped" => NodeSubstratePower::Stopped,
            _ => NodeSubstratePower::Unknown,
        }
    }

    pub fn resolve_build_runner_transport(
        registry_path: Option<&str>,
    ) -> Result<ResolvedBuildRunnerTransport, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let role = registry
            .lifecycle
            .as_ref()
            .and_then(|lifecycle| lifecycle.kernel_release.as_ref())
            .and_then(|release| release.build_runner_role.as_deref())
            .filter(|role| !role.trim().is_empty())
            .ok_or_else(|| "build_runner_role_missing".to_string())?;
        resolve_role_transport(&path, &registry, role, "build_runner")
    }

    pub fn resolve_framework_transport(
        registry_path: Option<&str>,
        role: &str,
    ) -> Result<ResolvedBuildRunnerTransport, String> {
        if role.trim().is_empty() {
            return Err("framework_target_role_missing".to_string());
        }
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        resolve_role_transport(&path, &registry, role, "framework_target")
    }

    fn resolve_role_transport(
        path: &Path,
        registry: &RegistryDocument,
        role: &str,
        concern: &str,
    ) -> Result<ResolvedBuildRunnerTransport, String> {
        let candidates = registry
            .nodes
            .iter()
            .filter(|node| node.role.as_deref().or(node.canonical_role.as_deref()) == Some(role))
            .collect::<Vec<_>>();
        let [node] = candidates.as_slice() else {
            return Err(if candidates.is_empty() {
                format!("{concern}_node_missing")
            } else {
                format!("{concern}_node_ambiguous")
            });
        };
        let node_id = node
            .id
            .clone()
            .or_else(|| node.name.clone())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("{concern}_node_id_missing"))?;
        let ssh = node
            .transports
            .as_ref()
            .and_then(|transports| transports.ssh.as_ref())
            .ok_or_else(|| format!("{concern}_transport_missing"))?;
        let ssh_target = ssh
            .target
            .clone()
            .filter(|value| !value.trim().is_empty() && !value.starts_with('-'))
            .ok_or_else(|| format!("{concern}_ssh_target_missing"))?;
        let trust = ssh
            .trust
            .clone()
            .filter(|value| value == "ssh_trust_anchor")
            .ok_or_else(|| format!("{concern}_ssh_trust_unproven"))?;
        let enrollment = ssh
            .enrollment_authority
            .as_ref()
            .ok_or_else(|| format!("{concern}_enrollment_authority_missing"))?;
        let enrollment_authority_class = enrollment
            .class
            .clone()
            .filter(|value| value == "machine_enrollment")
            .ok_or_else(|| format!("{concern}_enrollment_authority_not_machine"))?;
        let enrollment_principal = enrollment
            .principal
            .clone()
            .filter(|value| safe_transport_identity(value))
            .ok_or_else(|| format!("{concern}_enrollment_principal_invalid"))?;
        let enrollment_credential_ref = enrollment
            .credential_ref
            .clone()
            .filter(|value| safe_transport_identity(value))
            .ok_or_else(|| format!("{concern}_enrollment_credential_ref_invalid"))?;
        Ok(ResolvedBuildRunnerTransport {
            node_id,
            role: role.to_string(),
            transport_kind: "ssh_bootstrap".to_string(),
            ssh_target,
            trust,
            enrollment_authority_class,
            enrollment_principal,
            enrollment_credential_ref,
            registry_path: path.display().to_string(),
        })
    }

    fn safe_transport_identity(value: &str) -> bool {
        !value.is_empty()
            && !value.starts_with('-')
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'@')
            })
    }

    pub fn resolve_current_host_direct_management_link(
        synapse_cidr: &str,
        registry_path: Option<&str>,
    ) -> Result<Option<ResolvedDirectManagementLink>, String> {
        let hostname = current_hostname()?;
        resolve_host_direct_management_link(&hostname, Some(synapse_cidr), registry_path)
    }

    pub fn resolve_observed_direct_management_link(
        hostname: &str,
        synapse_cidr: &str,
        observed: Option<&ObservedManagementLink>,
        registry_path: Option<&str>,
    ) -> Result<ResolvedHostIdentity, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let hostname_node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, hostname));
        let network_matches = observed.map(|observed| {
            registry
                .nodes
                .iter()
                .filter_map(|node| {
                    let link = node.matching_direct_management_link(synapse_cidr)?;
                    (link.interface == observed.interface
                        && link.connection == observed.connection
                        && link.address == observed.address
                        && link.gateway == observed.gateway)
                        .then_some((node, link))
                })
                .collect::<Vec<_>>()
        });

        let (node, link, match_source, network_match) = match network_matches {
            Some(matches) if matches.len() == 1 => {
                let (node, link) = matches.into_iter().next().expect("one match");
                (node, link, "management_network", true)
            }
            Some(matches) if matches.is_empty() => {
                return Err("observed_management_link_not_declared_in_registry".to_string());
            }
            Some(_) => return Err("observed_management_identity_ambiguous".to_string()),
            None => {
                let node = hostname_node
                    .ok_or_else(|| format!("node not found in registry: {hostname}"))?;
                let link = node
                    .matching_direct_management_link(synapse_cidr)
                    .ok_or_else(|| "direct_management_link_not_declared_in_registry".to_string())?;
                (node, link, "hostname", false)
            }
        };
        let node_name = node
            .name
            .clone()
            .or_else(|| node.id.clone())
            .ok_or_else(|| "registry_node_missing_name".to_string())?;
        let hostname_match = node_matches(node, hostname);
        let mut metadata = vec![format!("identity_observation_source={match_source}")];
        if network_match && !hostname_match {
            metadata.push("hostname_registry_alias_mismatch".to_string());
        }
        Ok(ResolvedHostIdentity {
            node: node_name,
            role: node.role.clone().or_else(|| node.canonical_role.clone()),
            match_source: match_source.to_string(),
            hostname_match,
            network_match,
            metadata,
            link,
        })
    }

    pub fn resolve_host_direct_management_link(
        node_name: &str,
        synapse_cidr: Option<&str>,
        registry_path: Option<&str>,
    ) -> Result<Option<ResolvedDirectManagementLink>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, node_name))
            .ok_or_else(|| format!("node not found in registry: {node_name}"))?;
        let Some(cidr) = synapse_cidr
            .filter(|value| !value.trim().is_empty())
            .or_else(|| registry.synapse_cidr())
        else {
            return Ok(None);
        };
        if cidr.is_empty() {
            return Ok(None);
        }
        Ok(node.matching_direct_management_link(cidr.trim()))
    }

    pub fn resolve_host_profile(
        node_name: &str,
        registry_path: Option<&str>,
    ) -> Result<HostProfile, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, node_name))
            .ok_or_else(|| format!("node not found in registry: {node_name}"))?;
        Ok(profile_for_node(node))
    }

    pub fn resolve_current_host_profile(
        registry_path: Option<&str>,
    ) -> Result<HostProfile, String> {
        let hostname = current_hostname()?;
        resolve_host_profile(&hostname, registry_path)
    }

    pub fn resolve_host_profile_resolution(
        node_name: &str,
        registry_path: Option<&str>,
    ) -> Result<HostProfileResolution, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, node_name))
            .ok_or_else(|| format!("node not found in registry: {node_name}"))?;
        let profile = profile_for_node(node);
        let matched_name = registry_node_name(node).unwrap_or_else(|| node_name.to_string());
        Ok(HostProfileResolution {
            hostname_observed: node_name.to_string(),
            matched_node: Some(matched_name.clone()),
            matched_role: node.role.clone(),
            canonical_role: node.canonical_role.clone(),
            profile_applied: profile.name(),
            match_source: if matched_name == node_name {
                "node_name"
            } else {
                "alias"
            },
            host_bootstrap_target_stage: host_bootstrap_target_stage(&registry, node),
            host_bootstrap_required_domains: host_bootstrap_required_domains(&registry, node),
            registry_path: Some(path.display().to_string()),
        })
    }

    pub fn resolve_current_host_profile_resolution(
        registry_path: Option<&str>,
    ) -> Result<HostProfileResolution, String> {
        let hostname = current_hostname()?;
        resolve_host_profile_resolution(&hostname, registry_path)
    }

    pub fn resolve_registry_service_policy(
        node_name: &str,
        registry_path: Option<&str>,
    ) -> Result<Option<crate::domain::services::ServicePolicy>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, node_name))
            .ok_or_else(|| format!("node not found in registry: {node_name}"))?;
        let units = node
            .services
            .iter()
            .filter_map(|service| {
                let unit = service.unit.as_deref()?.trim();
                if unit.is_empty() {
                    None
                } else {
                    Some((
                        service.name.as_deref().unwrap_or(unit),
                        service.artifact.as_deref(),
                        service
                            .health
                            .as_ref()
                            .and_then(|health| health.port)
                            .or(service.port),
                        service
                            .health
                            .as_ref()
                            .and_then(|health| health.process.as_deref())
                            .or(service.artifact.as_deref()),
                        service
                            .supervisor
                            .as_ref()
                            .and_then(|supervisor| supervisor.provider.clone()),
                        service
                            .health
                            .as_ref()
                            .and_then(|health| health.kind.clone()),
                        unit,
                    ))
                }
            })
            .map(
                |(name, artifact, port, process, supervisor_provider, health_kind, unit)| {
                    crate::domain::services::ServiceUnitPolicy {
                        service_id: name.to_string(),
                        unit_name: unit.to_string(),
                        enable_on_boot: true,
                        restart_mode: "on-failure".to_string(),
                        source: format!("registry:{}", path.display()),
                        health_port: port,
                        process_name: process.or(artifact).map(str::to_string),
                        supervisor_provider,
                        health_kind,
                    }
                },
            )
            .collect::<Vec<_>>();
        if units.is_empty() {
            Ok(None)
        } else {
            let manager = service_manager_for_node(node, &units);
            Ok(Some(crate::domain::services::ServicePolicy {
                unit_dir: match manager {
                    crate::domain::services::ServiceManagerKind::OpenRc => {
                        "/etc/init.d".to_string()
                    }
                    crate::domain::services::ServiceManagerKind::Systemd => {
                        current_runtime_layout().service_unit_dir
                    }
                },
                manager,
                units,
            }))
        }
    }

    pub fn resolve_current_registry_service_policy(
        registry_path: Option<&str>,
    ) -> Result<Option<crate::domain::services::ServicePolicy>, String> {
        let hostname = current_hostname()?;
        resolve_registry_service_policy(&hostname, registry_path)
    }

    fn node_matches(node: &RegistryNode, requested: &str) -> bool {
        registry_node_name(node).as_deref() == Some(requested)
            || node.aliases.iter().any(|alias| alias == requested)
    }

    fn registry_node_name(node: &RegistryNode) -> Option<String> {
        node.name
            .clone()
            .or_else(|| node.id.clone())
            .filter(|value| !value.is_empty())
    }

    fn profile_for_node(node: &RegistryNode) -> HostProfile {
        if let Some(profile) = declared_host_family(node).and_then(host_profile_for_family) {
            return profile;
        }

        match node.role.as_deref() {
            Some("hypervisor") => HostProfile::ProxmoxHypervisor,
            // Build runners must not inherit mesh DNS/WireGuard substrate.
            Some("build_runner") | Some("builder") | Some("ci") => HostProfile::BuildRunner,
            _ if node.services.iter().any(|service| {
                service
                    .supervisor
                    .as_ref()
                    .and_then(|s| s.provider.as_deref())
                    == Some("openrc")
            }) =>
            {
                HostProfile::AlpineOpenRcNetworkNode
            }
            _ => HostProfile::NetworkNode,
        }
    }

    fn service_manager_for_node(
        node: &RegistryNode,
        units: &[crate::domain::services::ServiceUnitPolicy],
    ) -> crate::domain::services::ServiceManagerKind {
        match declared_host_family(node).and_then(host_profile_for_family) {
            Some(HostProfile::AlpineOpenRcNetworkNode) => {
                crate::domain::services::ServiceManagerKind::OpenRc
            }
            Some(
                HostProfile::NetworkNode
                | HostProfile::ProxmoxHypervisor
                | HostProfile::BuildRunner
                | HostProfile::WindowsNode,
            ) => crate::domain::services::ServiceManagerKind::Systemd,
            None if units
                .iter()
                .any(|unit| unit.supervisor_provider.as_deref() == Some("openrc")) =>
            {
                crate::domain::services::ServiceManagerKind::OpenRc
            }
            Some(HostProfile::ManagementNode) => current_service_manager_kind(),
            None => current_service_manager_kind(),
        }
    }

    fn current_service_manager_kind() -> crate::domain::services::ServiceManagerKind {
        service_manager_kind_from_runtime_kind(
            &crate::runtime_layout::current_runtime_layout().service_manager_kind(),
        )
    }

    fn service_manager_kind_from_runtime_kind(
        kind: &str,
    ) -> crate::domain::services::ServiceManagerKind {
        match kind {
            "openrc" => crate::domain::services::ServiceManagerKind::OpenRc,
            _ => crate::domain::services::ServiceManagerKind::Systemd,
        }
    }

    fn declared_host_family(node: &RegistryNode) -> Option<&str> {
        node.host_family
            .as_deref()
            .or_else(|| {
                node.substrate
                    .as_ref()
                    .and_then(|substrate| substrate.family.as_deref())
            })
            .map(str::trim)
            .filter(|family| !family.is_empty())
    }

    fn host_profile_for_family(family: &str) -> Option<HostProfile> {
        match family {
            "debian-systemd"
            | "linux.debian.systemd"
            | "ubuntu-systemd"
            | "linux.ubuntu.systemd" => Some(HostProfile::NetworkNode),
            "alpine-openrc" | "linux.alpine.openrc" => Some(HostProfile::AlpineOpenRcNetworkNode),
            "proxmox" | "proxmox-hypervisor" | "linux.proxmox.systemd" => {
                Some(HostProfile::ProxmoxHypervisor)
            }
            "windows" | "windows-service" | "windows.generic" => Some(HostProfile::WindowsNode),
            _ => None,
        }
    }

    fn host_bootstrap_target_stage(
        registry: &RegistryDocument,
        node: &RegistryNode,
    ) -> Option<String> {
        let role = node.role.as_deref()?;
        registry
            .lifecycle
            .as_ref()?
            .kernel_release
            .as_ref()?
            .host_bootstrap
            .as_ref()?
            .role_profiles
            .get(role)
            .map(|profile| profile.target_stage.clone())
    }

    fn host_bootstrap_required_domains(
        registry: &RegistryDocument,
        node: &RegistryNode,
    ) -> Vec<String> {
        let Some(role) = node.role.as_deref() else {
            return Vec::new();
        };
        registry
            .lifecycle
            .as_ref()
            .and_then(|lifecycle| lifecycle.kernel_release.as_ref())
            .and_then(|release| release.host_bootstrap.as_ref())
            .and_then(|bootstrap| bootstrap.role_profiles.get(role))
            .map(|profile| profile.required_domains.clone())
            .unwrap_or_default()
    }
}
