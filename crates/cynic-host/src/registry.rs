use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
use crate::profiles::HostProfile;
use crate::runtime_layout::current_runtime_layout;
pub use crate::runtime_registry::DEFAULT_REGISTRY_PATH;
use crate::runtime_registry::resolve_runtime_registry_path;

#[derive(Debug, Deserialize)]
struct RegistryDocument {
    #[serde(default)]
    lifecycle: Option<RegistryLifecycle>,
    nodes: Vec<RegistryNode>,
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
    host_family: Option<String>,
    #[serde(default)]
    substrate: Option<RegistryNodeSubstrate>,
    #[serde(default)]
    services: Vec<RegistryNodeService>,
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
    host_bootstrap: Option<RegistryHostBootstrap>,
}

#[derive(Debug, Deserialize)]
struct RegistryHostBootstrap {
    #[serde(default)]
    role_profiles: BTreeMap<String, RegistryHostBootstrapRoleProfile>,
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
        Some(HostProfile::NetworkNode | HostProfile::ProxmoxHypervisor) => {
            ServiceManagerKind::Systemd
        }
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
        resolve_current_host_profile, resolve_current_host_profile_resolution,
        resolve_host_profile, resolve_registry_service_policy,
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
