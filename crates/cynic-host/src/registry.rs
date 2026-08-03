use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
use crate::profiles::HostProfile;
use crate::runtime_layout::current_runtime_layout;
pub use crate::runtime_registry::DEFAULT_REGISTRY_PATH;
use crate::runtime_registry::resolve_runtime_registry_path;
use cynic_protocol::{NodeSubstrateDesiredState, NodeSubstratePower};

#[derive(Debug, Deserialize)]
struct RegistryDocument {
    #[serde(default)]
    network_policy: Option<RegistryNetworkPolicy>,
    #[serde(default)]
    lifecycle: Option<RegistryLifecycle>,
    nodes: Vec<RegistryNode>,
    #[serde(default)]
    fleet_desired: Option<RegistryFleetDesired>,
}

#[derive(Debug, Deserialize)]
struct RegistryFleetDesired {
    #[serde(default)]
    targets: Vec<RegistryFleetTarget>,
}

#[derive(Debug, Deserialize)]
struct RegistryFleetTarget {
    id: Option<String>,
    provider: Option<String>,
    resource_id: Option<String>,
    role: Option<String>,
    desired_power: Option<String>,
    #[serde(default)]
    provisioning: Option<cynic_protocol::NodeSubstrateProvisioningSpec>,
}

#[derive(Debug, Deserialize)]
struct RegistryNode {
    name: Option<String>,
    id: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    role: Option<String>,
    canonical_role: Option<String>,
    #[serde(default)]
    provider_controls: Vec<String>,
    #[serde(default)]
    transports: Option<RegistryNodeTransports>,
    #[serde(default)]
    host_family: Option<String>,
    #[serde(default)]
    substrate: Option<RegistryNodeSubstrate>,
    #[serde(default)]
    services: Vec<RegistryNodeService>,
    #[serde(default)]
    network_links: Vec<RegistryNetworkLink>,
}

#[derive(Debug, Deserialize)]
struct RegistryNodeTransports {
    #[serde(default)]
    ssh: Option<RegistrySshTransport>,
}

#[derive(Debug, Deserialize)]
struct RegistrySshTransport {
    target: Option<String>,
    trust: Option<String>,
    #[serde(default)]
    enrollment_authority: Option<RegistrySshEnrollmentAuthority>,
}

#[derive(Debug, Deserialize)]
struct RegistrySshEnrollmentAuthority {
    class: Option<String>,
    principal: Option<String>,
    credential_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryNetworkPolicy {
    #[serde(default)]
    planes: BTreeMap<String, RegistryNetworkPlane>,
}

#[derive(Debug, Deserialize)]
struct RegistryNetworkPlane {
    #[serde(default)]
    cidr: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryNetworkLink {
    #[serde(default)]
    plane: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    interface: Option<String>,
    #[serde(default)]
    connection: Option<String>,
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    gateway: Option<String>,
    #[serde(default)]
    routes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryNodeSubstrate {
    family: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryNodeService {
    name: Option<String>,
    artifact: Option<String>,
    port: Option<u16>,
    unit: Option<String>,
    #[serde(default)]
    supervisor: Option<RegistryServiceSupervisor>,
    #[serde(default)]
    health: Option<RegistryServiceHealth>,
}

#[derive(Debug, Deserialize)]
struct RegistryServiceSupervisor {
    provider: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryServiceHealth {
    kind: Option<String>,
    port: Option<u16>,
    process: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryLifecycle {
    #[serde(default)]
    kernel_release: Option<RegistryKernelRelease>,
}

#[derive(Debug, Deserialize)]
struct RegistryKernelRelease {
    #[serde(default)]
    build_runner_role: Option<String>,
    #[serde(default)]
    host_bootstrap: Option<RegistryHostBootstrap>,
}

#[derive(Debug, Deserialize)]
struct RegistryHostBootstrap {
    #[serde(default)]
    role_profiles: BTreeMap<String, RegistryHostBootstrapRoleProfile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedDirectManagementLink {
    pub interface: String,
    pub connection: String,
    pub address: String,
    pub gateway: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservedManagementLink {
    pub interface: String,
    pub connection: String,
    pub address: String,
    pub gateway: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedHostIdentity {
    pub node: String,
    pub role: Option<String>,
    pub match_source: String,
    pub hostname_match: bool,
    pub network_match: bool,
    pub metadata: Vec<String>,
    pub link: ResolvedDirectManagementLink,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedBuildRunnerTransport {
    pub node_id: String,
    pub role: String,
    pub ssh_target: String,
    pub trust: String,
    pub enrollment_authority_class: String,
    pub enrollment_principal: String,
    pub enrollment_credential_ref: String,
    pub registry_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedProviderControlTransport {
    pub provider: String,
    pub node_id: String,
    pub ssh_target: String,
    pub trust: String,
    pub registry_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedNodeSubstrateTarget {
    pub target_id: String,
    pub role: String,
    pub desired: NodeSubstrateDesiredState,
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
        let target_id = required_registry_value(target.id.as_deref(), "fleet_target_id_missing")?;
        let role = required_registry_value(target.role.as_deref(), "fleet_target_role_missing")?;
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
        .filter(|trust| trust == "known_hosts")
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
        .filter(|value| value == "known_hosts")
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
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'@'))
}

impl RegistryNetworkLink {
    fn to_direct_management_link(&self) -> Option<ResolvedDirectManagementLink> {
        let interface = self.interface.as_ref()?.trim();
        let connection = self.connection.as_ref()?.trim();
        let address = self.address.as_ref()?.trim();
        let gateway = self.gateway.as_ref()?.trim();
        if interface.is_empty() || connection.is_empty() || address.is_empty() || gateway.is_empty()
        {
            return None;
        }
        Some(ResolvedDirectManagementLink {
            interface: interface.to_string(),
            connection: connection.to_string(),
            address: address.to_string(),
            gateway: gateway.to_string(),
        })
    }
}

impl RegistryDocument {
    fn synapse_cidr(&self) -> Option<&str> {
        self.network_policy.as_ref().and_then(|policy| {
            policy
                .planes
                .get("synapse")
                .and_then(|plane| plane.cidr.as_deref())
        })
    }
}

impl RegistryNode {
    fn matching_direct_management_link(
        &self,
        synapse_cidr: &str,
    ) -> Option<ResolvedDirectManagementLink> {
        self.network_links.iter().find_map(|link| {
            let Some(plane) = link.plane.as_deref() else {
                return None;
            };
            if plane != "management" {
                return None;
            }
            if let Some(provider) = link.provider.as_deref() {
                if provider != "network-manager" {
                    return None;
                }
            }
            let route_match = link.routes.iter().any(|route| route.trim() == synapse_cidr);
            if !route_match {
                return None;
            }
            link.to_direct_management_link()
        })
    }
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
            let node =
                hostname_node.ok_or_else(|| format!("node not found in registry: {hostname}"))?;
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

#[derive(Debug, Deserialize)]
struct RegistryHostBootstrapRoleProfile {
    target_stage: String,
    #[serde(default)]
    required_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostProfileResolution {
    pub hostname_observed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_role: Option<String>,
    pub profile_applied: &'static str,
    pub match_source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_bootstrap_target_stage: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub host_bootstrap_required_domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_path: Option<String>,
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

pub fn resolve_current_host_profile(registry_path: Option<&str>) -> Result<HostProfile, String> {
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
) -> Result<Option<ServicePolicy>, String> {
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
                ServiceUnitPolicy {
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
        Ok(Some(ServicePolicy {
            unit_dir: match manager {
                ServiceManagerKind::OpenRc => "/etc/init.d".to_string(),
                ServiceManagerKind::Systemd => current_runtime_layout().service_unit_dir,
            },
            manager,
            units,
        }))
    }
}

pub fn resolve_current_registry_service_policy(
    registry_path: Option<&str>,
) -> Result<Option<ServicePolicy>, String> {
    let hostname = current_hostname()?;
    resolve_registry_service_policy(&hostname, registry_path)
}

fn resolve_registry_path(explicit_path: Option<&str>) -> Result<PathBuf, String> {
    resolve_runtime_registry_path(explicit_path.map(Path::new))
}

fn load_registry(path: &Path) -> Result<RegistryDocument, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read registry {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {}: {error}", path.display()))
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
    units: &[ServiceUnitPolicy],
) -> ServiceManagerKind {
    match declared_host_family(node).and_then(host_profile_for_family) {
        Some(HostProfile::AlpineOpenRcNetworkNode) => ServiceManagerKind::OpenRc,
        Some(
            HostProfile::NetworkNode | HostProfile::ProxmoxHypervisor | HostProfile::BuildRunner,
        ) => ServiceManagerKind::Systemd,
        None if units
            .iter()
            .any(|unit| unit.supervisor_provider.as_deref() == Some("openrc")) =>
        {
            ServiceManagerKind::OpenRc
        }
        None => current_service_manager_kind(),
    }
}

fn current_service_manager_kind() -> ServiceManagerKind {
    service_manager_kind_from_runtime_kind(
        &crate::runtime_layout::current_runtime_layout().service_manager_kind(),
    )
}

fn service_manager_kind_from_runtime_kind(kind: &str) -> ServiceManagerKind {
    match kind {
        "openrc" => ServiceManagerKind::OpenRc,
        _ => ServiceManagerKind::Systemd,
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
        "debian-systemd" | "linux.debian.systemd" | "ubuntu-systemd" | "linux.ubuntu.systemd" => {
            Some(HostProfile::NetworkNode)
        }
        "alpine-openrc" | "linux.alpine.openrc" => Some(HostProfile::AlpineOpenRcNetworkNode),
        "proxmox" | "proxmox-hypervisor" | "linux.proxmox.systemd" => {
            Some(HostProfile::ProxmoxHypervisor)
        }
        _ => None,
    }
}

fn host_bootstrap_target_stage(registry: &RegistryDocument, node: &RegistryNode) -> Option<String> {
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

fn current_hostname() -> Result<String, String> {
    let trimmed = crate::runtime_layout::observed_hostname();
    if trimmed.is_empty() {
        Err("hostname resolution returned empty output".to_string())
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_build_runner_transport, resolve_current_host_direct_management_link,
        resolve_current_host_profile, resolve_current_host_profile_resolution,
        resolve_framework_transport, resolve_host_profile, resolve_registry_service_policy,
    };
    use crate::profiles::HostProfile;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolves_hypervisor_node_to_proxmox_profile() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {"name":"cynic-host-proxmox","role":"hypervisor","aliases":["pve"]},
                {"name":"cynic-vm-v2","role":"kernel_primary"}
              ]
            }"#,
        );
        let profile = resolve_host_profile(
            "cynic-host-proxmox",
            Some(path.to_str().expect("utf8 temp path")),
        )
        .expect("profile resolved");
        assert_eq!(profile, HostProfile::ProxmoxHypervisor);
    }

    #[test]
    fn resolves_alias_to_same_host_profile() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {"name":"cynic-host-proxmox","role":"hypervisor","aliases":["pve"]}
              ]
            }"#,
        );
        let profile = resolve_host_profile("pve", Some(path.to_str().expect("utf8 temp path")))
            .expect("profile resolved");
        assert_eq!(profile, HostProfile::ProxmoxHypervisor);
    }

    #[test]
    fn resolves_unique_trusted_build_runner_transport() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [{
                "id":"builder-1",
                "role":"build_runner",
                "transports":{"ssh":{"target":"cynic@builder.internal","trust":"known_hosts","enrollment_authority":{"class":"machine_enrollment","principal":"cynic-enroll","credential_ref":"operator-ssh-agent"}}}
              }],
              "lifecycle":{"kernel_release":{"build_runner_role":"build_runner"}}
            }"#,
        );
        let transport =
            resolve_build_runner_transport(Some(path.to_str().expect("utf8 temp path")))
                .expect("transport resolves");
        assert_eq!(transport.node_id, "builder-1");
        assert_eq!(transport.ssh_target, "cynic@builder.internal");
        assert_eq!(transport.trust, "known_hosts");
        assert_eq!(transport.enrollment_authority_class, "machine_enrollment");
        assert_eq!(transport.enrollment_principal, "cynic-enroll");
        assert_eq!(transport.enrollment_credential_ref, "operator-ssh-agent");
    }

    #[test]
    fn build_runner_transport_without_machine_enrollment_authority_is_blocked() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [{
                "id":"builder-1",
                "role":"build_runner",
                "transports":{"ssh":{"target":"builder.internal","trust":"known_hosts"}}
              }],
              "lifecycle":{"kernel_release":{"build_runner_role":"build_runner"}}
            }"#,
        );
        let error = resolve_build_runner_transport(Some(path.to_str().expect("utf8 temp path")))
            .expect_err("implicit enrollment authority must block");
        assert_eq!(error, "build_runner_enrollment_authority_missing");
    }

    #[test]
    fn framework_transport_resolves_the_explicit_seed_role() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [{
                "id":"storage-1",
                "role":"storage_primary",
                "transports":{"ssh":{"target":"storage.internal","trust":"known_hosts","enrollment_authority":{"class":"machine_enrollment","principal":"root","credential_ref":"registry-machine-ssh-identity"}}}
              }]
            }"#,
        );

        let transport = resolve_framework_transport(
            Some(path.to_str().expect("utf8 temp path")),
            "storage_primary",
        )
        .expect("transport resolves");

        assert_eq!(transport.node_id, "storage-1");
        assert_eq!(transport.role, "storage_primary");
        assert_eq!(transport.ssh_target, "storage.internal");
    }

    #[test]
    fn missing_build_runner_transport_remains_blocked() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [{"id":"builder-1","role":"build_runner"}],
              "lifecycle":{"kernel_release":{"build_runner_role":"build_runner"}}
            }"#,
        );
        let error = resolve_build_runner_transport(Some(path.to_str().expect("utf8 temp path")))
            .expect_err("missing transport must block");
        assert_eq!(error, "build_runner_transport_missing");
    }

    #[test]
    fn resolves_non_hypervisor_to_network_profile() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {"name":"cynic-host-cortex","role":"cortex_host","aliases":["ubuntu"]}
              ]
            }"#,
        );
        let profile = resolve_host_profile(
            "cynic-host-cortex",
            Some(path.to_str().expect("utf8 temp path")),
        )
        .expect("profile resolved");
        assert_eq!(profile, HostProfile::NetworkNode);
    }

    #[test]
    fn resolves_current_host_profile_from_hostname_env_alias() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {"name":"cynic-host-cortex","role":"cortex_host","aliases":["ubuntu"]}
              ]
            }"#,
        );
        unsafe {
            std::env::set_var("HOSTNAME", "ubuntu");
        }
        let profile = resolve_current_host_profile(Some(path.to_str().expect("utf8 temp path")))
            .expect("profile resolved");
        assert_eq!(profile, HostProfile::NetworkNode);
        let resolution =
            resolve_current_host_profile_resolution(Some(path.to_str().expect("utf8 temp path")))
                .expect("resolution resolved");
        assert_eq!(resolution.hostname_observed, "ubuntu");
        assert_eq!(
            resolution.matched_node.as_deref(),
            Some("cynic-host-cortex")
        );
        assert_eq!(resolution.profile_applied, "network-node");
        assert_eq!(resolution.match_source, "alias");
    }

    #[test]
    fn resolves_current_host_direct_management_link_from_network_links() {
        let path = write_registry_fixture(
            r#"{
              "network_policy": {
                "planes": {
                  "synapse": { "cidr": "10.10.10.0/24" }
                }
              },
              "nodes": [
                {
                  "name":"cynic-host-cortex",
                  "role":"cortex_host",
                  "network_links": [
                    {
                      "plane": "management",
                      "provider": "network-manager",
                      "interface": "enp42s0",
                      "connection": "Connexion filaire 1",
                      "address": "192.168.100.2/24",
                      "gateway": "192.168.100.1",
                      "routes": ["10.10.10.0/24"],
                      "default_route": false
                    }
                  ]
                }
              ]
            }"#,
        );
        unsafe {
            std::env::set_var("HOSTNAME", "cynic-host-cortex");
        }
        let link = resolve_current_host_direct_management_link(
            "10.10.10.0/24",
            Some(path.to_str().expect("utf8 temp path")),
        )
        .expect("projection resolves");
        let link = link.expect("link present");

        assert_eq!(link.interface, "enp42s0");
        assert_eq!(link.connection, "Connexion filaire 1");
        assert_eq!(link.address, "192.168.100.2/24");
        assert_eq!(link.gateway, "192.168.100.1");
    }

    #[test]
    fn returns_no_direct_management_link_when_registry_missing_match() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {
                  "name":"cynic-host-cortex",
                  "role":"cortex_host",
                  "network_links": [
                    {
                      "plane": "management",
                      "provider": "network-manager",
                      "interface": "enp42s0",
                      "connection": "Connexion filaire 1",
                      "address": "192.168.100.2/24",
                      "gateway": "192.168.100.1",
                      "routes": ["10.11.11.0/24"],
                      "default_route": false
                    }
                  ]
                }
              ]
            }"#,
        );
        unsafe {
            std::env::set_var("HOSTNAME", "cynic-host-cortex");
        }
        let link = resolve_current_host_direct_management_link(
            "10.10.10.0/24",
            Some(path.to_str().expect("utf8 temp path")),
        )
        .expect("projection resolves");

        assert!(link.is_none());
    }

    #[test]
    fn resolves_registry_declared_service_units() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {
                  "name":"local-kernel",
                  "role":"kernel_primary",
                  "services": [
                    {
                      "name":"kernel",
                      "unit":"cynic-kernel.service",
                      "supervisor":{"provider":"process-port"},
                      "health":{"kind":"tcp","port":3030}
                    },
                    {
                      "name":"node",
                      "unit":"cynic-node.service",
                      "supervisor":{"provider":"openrc"},
                      "health":{"kind":"process","process":"cynic-node"}
                    }
                  ]
                }
              ]
            }"#,
        );
        let policy = resolve_registry_service_policy(
            "local-kernel",
            Some(path.to_str().expect("utf8 temp path")),
        )
        .expect("service policy resolves")
        .expect("service policy present");
        assert_eq!(
            policy.manager,
            crate::domain::services::ServiceManagerKind::OpenRc
        );
        assert_eq!(policy.unit_dir, "/etc/init.d");
        let units = policy
            .units
            .iter()
            .map(|unit| unit.unit_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(units, vec!["cynic-kernel.service", "cynic-node.service"]);
        assert_eq!(
            policy.units[0].supervisor_provider.as_deref(),
            Some("process-port")
        );
        assert_eq!(policy.units[0].health_kind.as_deref(), Some("tcp"));
        assert_eq!(policy.units[0].health_port, Some(3030));
        assert_eq!(
            policy.units[1].supervisor_provider.as_deref(),
            Some("openrc")
        );
        assert_eq!(policy.units[1].health_kind.as_deref(), Some("process"));
        assert_eq!(policy.units[1].process_name.as_deref(), Some("cynic-node"));
    }

    #[test]
    fn resolves_openrc_registry_node_to_alpine_profile() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {
                  "name":"alpine-node",
                  "role":"kernel_primary",
                  "services": [
                    {
                      "name":"node",
                      "unit":"cynic-node",
                      "supervisor":{"provider":"openrc"},
                      "health":{"kind":"process","process":"cynic-node"}
                    }
                  ]
                }
              ]
            }"#,
        );

        let profile = resolve_host_profile("alpine-node", Some(path.to_str().expect("utf8 path")))
            .expect("profile resolved");

        assert_eq!(profile, HostProfile::AlpineOpenRcNetworkNode);
        assert_eq!(profile.name(), "alpine-openrc");
    }

    #[test]
    fn resolves_explicit_host_family_before_service_heuristics() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {
                  "name":"debian-node",
                  "role":"kernel_primary",
                  "host_family":"linux.debian.systemd",
                  "services": [
                    {
                      "name":"legacy-openrc-shaped-unit",
                      "unit":"cynic-node",
                      "supervisor":{"provider":"openrc"}
                    }
                  ]
                }
              ]
            }"#,
        );

        let profile = resolve_host_profile("debian-node", Some(path.to_str().expect("utf8 path")))
            .expect("profile resolved");
        let policy =
            resolve_registry_service_policy("debian-node", Some(path.to_str().expect("utf8 path")))
                .expect("policy resolved")
                .expect("policy present");

        assert_eq!(profile, HostProfile::NetworkNode);
        assert_eq!(
            policy.manager,
            crate::domain::services::ServiceManagerKind::Systemd
        );
    }

    #[test]
    fn resolves_explicit_alpine_host_family_without_openrc_service_hint() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {
                  "name":"alpine-node",
                  "role":"kernel_primary",
                  "host_family":"linux.alpine.openrc",
                  "services": [
                    {
                      "name":"node",
                      "unit":"cynic-node",
                      "supervisor":{"provider":"process-port"}
                    }
                  ]
                }
              ]
            }"#,
        );

        let profile = resolve_host_profile("alpine-node", Some(path.to_str().expect("utf8 path")))
            .expect("profile resolved");
        let policy =
            resolve_registry_service_policy("alpine-node", Some(path.to_str().expect("utf8 path")))
                .expect("policy resolved")
                .expect("policy present");

        assert_eq!(profile, HostProfile::AlpineOpenRcNetworkNode);
        assert_eq!(
            policy.manager,
            crate::domain::services::ServiceManagerKind::OpenRc
        );
        assert_eq!(policy.unit_dir, "/etc/init.d");
    }

    #[test]
    fn service_manager_fallback_is_capability_driven_not_systemd_hardcoded() {
        assert_eq!(
            super::service_manager_kind_from_runtime_kind("openrc"),
            crate::domain::services::ServiceManagerKind::OpenRc
        );
        assert_eq!(
            super::service_manager_kind_from_runtime_kind("systemd"),
            crate::domain::services::ServiceManagerKind::Systemd
        );
    }

    #[test]
    fn resolves_substrate_family_alias_to_host_profile() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {
                  "name":"alpine-node",
                  "role":"kernel_primary",
                  "substrate":{"family":"linux.alpine.openrc"}
                }
              ]
            }"#,
        );

        let profile = resolve_host_profile("alpine-node", Some(path.to_str().expect("utf8 path")))
            .expect("profile resolved");

        assert_eq!(profile, HostProfile::AlpineOpenRcNetworkNode);
    }

    #[test]
    fn substrate_desired_state_preserves_missing_provider_mapping_as_unknown() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {"id":"builder","role":"build_runner"}
              ]
            }"#,
        );

        let desired = super::resolve_node_substrate_desired_state(
            Some(path.to_str().expect("path")),
            "builder",
        )
        .expect("desired state");

        assert_eq!(desired.node_id, "builder");
        assert_eq!(desired.provider, None);
        assert_eq!(desired.resource_id, None);
        assert_eq!(
            desired.desired_power,
            cynic_protocol::NodeSubstratePower::Unknown
        );
    }

    #[test]
    fn substrate_desired_state_projects_role_target_from_fleet_registry() {
        let path = write_registry_fixture(
            r#"{
              "nodes": [
                {"id":"builder","role":"build_runner"}
              ],
              "fleet_desired": {
                "targets": [
                  {
                    "id":"builder-primary",
                    "provider":"provider-adapter",
                    "resource_id":"machine/42",
                    "role":"build_runner",
                    "desired_power":"running"
                  }
                ]
              }
            }"#,
        );

        let desired = super::resolve_node_substrate_desired_state(
            Some(path.to_str().expect("path")),
            "builder",
        )
        .expect("desired state");

        assert_eq!(desired.provider.as_deref(), Some("provider-adapter"));
        assert_eq!(desired.resource_id.as_deref(), Some("machine/42"));
        assert_eq!(
            desired.desired_power,
            cynic_protocol::NodeSubstratePower::Running
        );
        let targets = super::resolve_node_substrate_targets(Some(path.to_str().expect("path")))
            .expect("targets");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_id, "builder-primary");
        assert_eq!(targets[0].role, "build_runner");
        assert_eq!(targets[0].desired, desired);
    }

    fn write_registry_fixture(contents: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cynic-host-registry-{unique}.json"));
        std::fs::write(&path, contents).expect("write fixture");
        path
    }
}
