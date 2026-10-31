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
    RegistrySourceRepository, RegistrySourceWorkspaceLayout, RegistrySshEnrollmentAuthority,
    RegistrySshTransport, ResolvedBuildRunnerTransport, ResolvedDirectManagementLink,
    ResolvedHostIdentity, ResolvedNodeSubstrateTarget, ResolvedProviderControlTransport,
};
pub use validate::{
    ValidationError, check_role_consistency, validate_registry, validate_registry_or_error,
};

// Keep existing public functions for backward compatibility
pub use crate::registry::resolve::{
    resolve_build_runner_transport, resolve_current_host_direct_management_link,
    resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_registry_retired_units,
    resolve_current_registry_service_policy, resolve_framework_transport,
    resolve_host_direct_management_link, resolve_host_profile, resolve_host_profile_resolution,
    resolve_node_substrate_desired_state, resolve_node_substrate_targets,
    resolve_observed_direct_management_link, resolve_provider_control_transport,
    resolve_registry_service_policy, resolve_source_transport_uri,
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

    /// Resolve the minimum free bytes a runner must have before a checkout
    /// may be materialized on it.
    ///
    /// The threshold is declared per node in the registry under
    /// `build_lifecycle.minimum_available_bytes`. `runner` may be a node id
    /// or the runner's ssh target (host or `user@host`), because the CLI
    /// resolves a runner to a host before the gate is armed.
    ///
    /// A node with no declared `build_lifecycle` yields `None`: the gate
    /// stays off for it, and the absence is explicit rather than a silent
    /// zero.
    pub fn minimum_available_bytes_for_runner(
        registry_path: Option<&str>,
        runner: &str,
    ) -> Result<Option<u64>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let wanted = runner.trim();
        if wanted.is_empty() {
            return Ok(None);
        }
        for node in &registry.nodes {
            let node_id = node
                .id
                .clone()
                .or_else(|| node.name.clone())
                .unwrap_or_default();
            let ssh_target = node
                .transports
                .as_ref()
                .and_then(|transports| transports.ssh.as_ref())
                .and_then(|ssh| ssh.target.clone());
            let matches = node_id == wanted
                || ssh_target.as_deref() == Some(wanted)
                || ssh_target
                    .as_deref()
                    .and_then(|target| target.split('@').next_back())
                    == Some(wanted);
            if matches {
                return Ok(
                    cynic_build::cache::resolve_minimum_available_bytes(
                        &path, &node_id,
                    )?,
                );
            }
        }
        Ok(None)
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
        let preferred = registry
            .lifecycle
            .as_ref()
            .and_then(|lifecycle| lifecycle.kernel_release.as_ref())
            .and_then(|release| release.build_runner_default.as_deref())
            .filter(|node| !node.trim().is_empty());
        resolve_role_transport(&path, &registry, role, "build_runner", preferred)
    }

    /// Resolve a source repository clone URI from the registry.
    ///
    /// Scans every node's `source_workspace_layout.repositories` for
    /// `repository_id` and returns its declared `transport_uri`. Explicit
    /// `--ssot-url` wins upstream; this is the registry-driven default that
    /// replaces the hardcoded `DEFAULT_SSOT_URL`. Fails closed with a named
    /// reason when the declaration is absent or ambiguous — a clone must
    /// never silently fall back to an unresolvable default.
    pub fn resolve_source_transport_uri(
        registry_path: Option<&str>,
        repository_id: &str,
    ) -> Result<String, String> {
        if repository_id.trim().is_empty() {
            return Err("source_repository_id_missing".to_string());
        }
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let mut found = Vec::new();
        for node in &registry.nodes {
            if let Some(layout) = node.source_workspace_layout.as_ref() {
                for repository in &layout.repositories {
                    if repository.id.as_deref() == Some(repository_id) {
                        if let Some(uri) = repository
                            .transport_uri
                            .as_ref()
                            .map(|uri| uri.trim().to_string())
                            .filter(|uri| !uri.is_empty())
                        {
                            found.push(uri);
                        }
                    }
                }
            }
        }
        match found.as_slice() {
            [] => Err(format!("source_transport_missing:{repository_id}")),
            [uri] => Ok(uri.clone()),
            _ => Err(format!("source_transport_ambiguous:{repository_id}")),
        }
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
        resolve_role_transport(&path, &registry, role, "framework_target", None)
    }

    /// The build runner's canonical build cache path, relative to the
    /// state root, as the registry declares it.
    ///
    /// WHY A RESOLVER AND NOT A CONSTANT. The runner used to compile the
    /// path in (`state_dir().join("shared-target")`) while the registry
    /// declared another one (`build-targets/bootstrap`). The two claims
    /// about the same machine disagreed, and the tooling that has to
    /// RECLAIM the cache read the registry: it looked for a canonical
    /// target that did not exist, found the real 30 Go of cache outside
    /// the policy, and refused to act.
    ///
    /// A registry that does not declare the path is an explicit refusal.
    /// A guess would put the runner and the governance on two different
    /// caches again, which is the failure this replaces.
    pub fn resolve_canonical_target_relative_path(
        registry_path: Option<&str>,
    ) -> Result<String, String> {
        let path = resolve_registry_path(registry_path)?;
        cynic_build::cache::canonical_target_relative_path(&path)
    }

    /// Resolve a runner by node id, name, or alias (e.g. a compat builder
    /// alongside the default `build_runner` role). Same transport checks as
    /// the role path; fails closed when the node or its SSH transport is
    /// missing.
    pub fn resolve_named_runner_transport(
        registry_path: Option<&str>,
        node_id: &str,
    ) -> Result<ResolvedBuildRunnerTransport, String> {
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
            .ok_or_else(|| "named_runner_node_missing".to_string())?;
        let role = role_for_node(&registry, node_id)?;
        resolve_node_transport(&path, node, &role, "named_runner")
    }

    fn role_for_node(registry: &RegistryDocument, node_id: &str) -> Result<String, String> {
        // A named node must carry exactly one role; the role itself may be
        // shared with other runners.
        let roles: Vec<&str> = registry
            .nodes
            .iter()
            .filter(|node| {
                node.id.as_deref() == Some(node_id)
                    || node.name.as_deref() == Some(node_id)
                    || node.aliases.iter().any(|alias| alias == node_id)
            })
            .filter_map(|node| node.role.as_deref().or(node.canonical_role.as_deref()))
            .collect();
        match roles.as_slice() {
            [role] => Ok(role.to_string()),
            _ => Err("named_runner_role_unresolvable".to_string()),
        }
    }

    /// Projected runner entry for operator/agent discovery.
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct RunnerEntry {
        pub node_id: String,
        pub role: String,
        pub ssh_target: String,
        pub trust: String,
        pub enrollment_principal: String,
        pub default: bool,
    }

    /// List every registry node whose role carries build duties, with the
    /// default `build_runner` role flagged. Read-only projection; no probing.
    pub fn list_runner_nodes(registry_path: Option<&str>) -> Result<Vec<RunnerEntry>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let default_role = registry
            .lifecycle
            .as_ref()
            .and_then(|lifecycle| lifecycle.kernel_release.as_ref())
            .and_then(|release| release.build_runner_role.clone())
            .unwrap_or_else(|| "build_runner".to_string());
        let default_node = registry
            .lifecycle
            .as_ref()
            .and_then(|lifecycle| lifecycle.kernel_release.as_ref())
            .and_then(|release| release.build_runner_default.clone());
        let mut runners = Vec::new();
        for node in &registry.nodes {
            let role = match node.role.as_deref().or(node.canonical_role.as_deref()) {
                Some(role) if role.contains("runner") => role.to_string(),
                _ => continue,
            };
            let transport = node.transports.as_ref().and_then(|t| t.ssh.as_ref());
            let Some(ssh) = transport else { continue };
            let Some(target) = ssh.target.clone() else {
                continue;
            };
            runners.push(RunnerEntry {
                node_id: node
                    .id
                    .clone()
                    .or_else(|| node.name.clone())
                    .unwrap_or_default(),
                role: role.clone(),
                ssh_target: target,
                trust: ssh.trust.clone().unwrap_or_default(),
                enrollment_principal: ssh
                    .enrollment_authority
                    .as_ref()
                    .and_then(|authority| authority.principal.clone())
                    .unwrap_or_default(),
                default: role == default_role
                    && default_node
                        .as_deref()
                        .map_or(true, |name| node_is_named(node, name)),
            });
        }
        runners.sort_by(|left, right| {
            right
                .default
                .cmp(&left.default)
                .then(left.node_id.cmp(&right.node_id))
        });
        Ok(runners)
    }

    fn resolve_role_transport(
        path: &Path,
        registry: &RegistryDocument,
        role: &str,
        concern: &str,
        preferred: Option<&str>,
    ) -> Result<ResolvedBuildRunnerTransport, String> {
        let candidates = registry
            .nodes
            .iter()
            .filter(|node| node.role.as_deref().or(node.canonical_role.as_deref()) == Some(role))
            .collect::<Vec<_>>();
        let selected = match (candidates.as_slice(), preferred) {
            ([], _) => return Err(format!("{concern}_node_missing")),
            ([node], _) => *node,
            (_, None) => return Err(format!("{concern}_node_ambiguous")),
            (_, Some(name)) => {
                let named = candidates
                    .iter()
                    .copied()
                    .filter(|node| node_is_named(node, name))
                    .collect::<Vec<_>>();
                match named.as_slice() {
                    [node] => *node,
                    _ => return Err(format!("{concern}_default_unresolved")),
                }
            }
        };
        resolve_node_transport(path, selected, role, concern)
    }

    /// Validate one already-selected node's SSH transport. Selection (by role
    /// or by name) stays with the caller so a role shared by several runners
    /// does not block an explicitly named one.
    fn resolve_node_transport(
        path: &Path,
        node: &RegistryNode,
        role: &str,
        concern: &str,
    ) -> Result<ResolvedBuildRunnerTransport, String> {
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
        service_cidr: &str,
        registry_path: Option<&str>,
    ) -> Result<Option<ResolvedDirectManagementLink>, String> {
        let hostname = current_hostname()?;
        resolve_host_direct_management_link(&hostname, Some(service_cidr), registry_path)
    }

    pub fn resolve_observed_direct_management_link(
        hostname: &str,
        service_cidr: &str,
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
                    let link = node.matching_direct_management_link(service_cidr)?;
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
                    .matching_direct_management_link(service_cidr)
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
        service_cidr: Option<&str>,
        registry_path: Option<&str>,
    ) -> Result<Option<ResolvedDirectManagementLink>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, node_name))
            .ok_or_else(|| format!("node not found in registry: {node_name}"))?;
        let Some(cidr) = service_cidr
            .filter(|value| !value.trim().is_empty())
            .or_else(|| registry.service_cidr())
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
        reject_unresolvable_family(node)?;
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
        reject_unresolvable_family(node)?;
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
                retired_units: node.retired_units.clone(),
            }))
        }
    }

    /// Units the registry declares this node must NOT run.
    ///
    /// The registry is the single authority for what a host runs, so the
    /// retirement list lives there and not in a hardcoded profile. A node
    /// with no `retired_units` key retires nothing.
    pub fn resolve_registry_retired_units(
        node_name: &str,
        registry_path: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let path = resolve_registry_path(registry_path)?;
        let registry = load_registry(&path)?;
        let node = registry
            .nodes
            .iter()
            .find(|node| node_matches(node, node_name))
            .ok_or_else(|| format!("node not found in registry: {node_name}"))?;
        Ok(node.retired_units.clone())
    }

    pub fn resolve_current_registry_service_policy(
        registry_path: Option<&str>,
    ) -> Result<Option<crate::domain::services::ServicePolicy>, String> {
        let hostname = current_hostname()?;
        resolve_registry_service_policy(&hostname, registry_path)
    }

    /// True when `requested` is the node's id, name or alias.
    fn node_is_named(node: &RegistryNode, requested: &str) -> bool {
        node.id.as_deref() == Some(requested)
            || node.name.as_deref() == Some(requested)
            || node.aliases.iter().any(|alias| alias == requested)
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

    /// Whether the node resolves as declared.
    ///
    /// A node that declares a family the resolver cannot interpret resolves
    /// under the wildcard profile above, which is a substitution nobody chose.
    /// This reports that condition so callers can reject the node instead of
    /// silently running it as something else.
    fn node_family_resolves_as_declared(node: &RegistryNode) -> bool {
        let Some(family) = declared_host_family(node) else {
            // Nothing declared, so nothing was substituted. The role match
            // decides, which is the documented path for undeclared nodes.
            return true;
        };
        host_profile_for_family(family).is_some()
    }

    /// Refuse a node whose declared family the resolver cannot interpret.
    ///
    /// An absent family is left alone: those nodes resolve on role, which is
    /// a declared behaviour rather than a substitution. Only a family that is
    /// present and unknown is rejected, because that node would otherwise run
    /// under the wildcard profile while appearing to be configured.
    fn reject_unresolvable_family(node: &RegistryNode) -> Result<(), String> {
        if node_family_resolves_as_declared(node) {
            return Ok(());
        }
        let family = declared_host_family(node).unwrap_or_default();
        let name = registry_node_name(node).unwrap_or_else(|| "<unnamed>".to_string());
        Err(format!(
            "node {name} declares host_family {family}, which the host profile resolver does not \
             recognise; it would otherwise resolve under the default profile instead of the \
             declared one. Use one of: {}.",
            crate::domain::host_family_drift::RESOLVED_FAMILIES.join(", ")
        ))
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

    #[cfg(test)]
    mod host_family_resolution_tests {
        use super::*;
        use crate::registry::types::RegistryNode;

        fn node_with_family(name: &str, role: Option<&str>, family: Option<&str>) -> RegistryNode {
            RegistryNode {
                name: Some(name.to_string()),
                id: Some(name.to_string()),
                aliases: vec![],
                role: role.map(str::to_string),
                canonical_role: None,
                provider_controls: vec![],
                transports: None,
                host_family: family.map(str::to_string),
                substrate: None,
                services: vec![],
                network_links: vec![],
                source_workspace_layout: None,
                retired_units: Vec::new(),
            }
        }

        #[test]
        fn undeclared_family_is_left_to_the_role_match() {
            // The four nodes in infra/registry.json with no declared family must
            // keep resolving, otherwise this change breaks live infrastructure.
            for node in [
                node_with_family("cynic-host-cortex", Some("cortex_host"), None),
                node_with_family("local-kernel", Some("kernel_primary"), None),
                node_with_family("local-builder", None, None),
                node_with_family("forgejo", Some("forge_primary"), None),
            ] {
                assert!(
                    node_family_resolves_as_declared(&node),
                    "{} declares no family and must not be rejected",
                    node.name.as_deref().unwrap_or("?")
                );
                assert!(reject_unresolvable_family(&node).is_ok());
            }
        }

        #[test]
        fn recognised_families_are_accepted() {
            for family in [
                "linux.debian.systemd",
                "linux.ubuntu.systemd",
                "linux.proxmox.systemd",
                "windows-service",
            ] {
                let node = node_with_family("n", Some("kernel_primary"), Some(family));
                assert!(
                    node_family_resolves_as_declared(&node),
                    "{family} is in the resolver table"
                );
            }
        }

        #[test]
        fn the_value_that_caused_the_silent_fallback_is_rejected() {
            // windows.windows11 was live in the registry and resolved as a Linux
            // network node. It must now fail loudly instead.
            let node = node_with_family(
                "laptop",
                Some("workstation_portable"),
                Some("windows.windows11"),
            );
            assert!(!node_family_resolves_as_declared(&node));
            let error = reject_unresolvable_family(&node).expect_err("must be rejected");
            assert!(error.contains("windows.windows11"), "{error}");
            assert!(
                error.contains("windows-service"),
                "must list valid values: {error}"
            );
        }

        #[test]
        fn the_rejection_message_names_the_consequence() {
            let node = node_with_family(
                "laptop",
                Some("workstation_portable"),
                Some("windows.windows11"),
            );
            let error = reject_unresolvable_family(&node).expect_err("must be rejected");
            assert!(error.contains("laptop"), "must name the node: {error}");
            assert!(
                error.contains("default profile"),
                "must state what would have happened instead: {error}"
            );
        }

        #[test]
        fn a_blank_family_is_not_an_unknown_family() {
            let node = node_with_family("n", Some("kernel_primary"), Some("   "));
            // declared_host_family filters blanks to None, so this is an absent
            // family, not an unrecognised one, and must not be rejected.
            assert!(reject_unresolvable_family(&node).is_ok());
        }

        #[test]
        fn the_laptop_family_now_declared_in_the_registry_resolves_to_windows() {
            let node = node_with_family(
                "cynic-laptop-ax16pro",
                Some("workstation_portable"),
                Some("windows-service"),
            );
            assert!(node_family_resolves_as_declared(&node));
            assert_eq!(profile_for_node(&node), HostProfile::WindowsNode);
        }

        #[test]
        fn an_unrecognised_family_would_have_resolved_to_the_wildcard_profile() {
            // This is the regression being prevented: record what the wildcard
            // produced so a future change to profile_for_node cannot silently
            // alter the substituted profile without this test noticing.
            let node = node_with_family(
                "laptop",
                Some("workstation_portable"),
                Some("windows.windows11"),
            );
            assert_eq!(profile_for_node(&node), HostProfile::NetworkNode);
        }
    }

    #[cfg(test)]
    mod source_transport_resolution_tests {
        use super::resolve::resolve_source_transport_uri;

        fn write_fixture(name: &str, body: &str) -> String {
            let path = std::env::temp_dir().join(format!(
                "cynic-source-transport-{name}-{}",
                std::process::id()
            ));
            std::fs::write(&path, body).expect("fixture must be writable");
            path.to_string_lossy().to_string()
        }

        fn node_with_repository(id: &str, transport_uri: Option<&str>) -> String {
            let uri = transport_uri
                .map(|uri| format!("\"transport_uri\": \"{uri}\""))
                .unwrap_or_default();
            format!(
                r#"{{"id": "builder", "name": "builder",
                    "source_workspace_layout": {{"repositories": [
                        {{"id": "{id}", "relative_path": "CYNIC", {uri}}}]}}}}"#,
            )
        }

        fn registry_with_nodes(nodes: &[String]) -> String {
            format!("{{\"nodes\": [{}]}}", nodes.join(","))
        }

        #[test]
        fn declared_transport_uri_resolves() {
            let path = write_fixture(
                "declared",
                &registry_with_nodes(&[node_with_repository(
                    "cynic",
                    Some("ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git"),
                )]),
            );
            let uri = resolve_source_transport_uri(Some(&path), "cynic")
                .expect("declared transport must resolve");
            assert_eq!(
                uri,
                "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git"
            );
        }

        #[test]
        fn unknown_repository_id_fails_closed() {
            let path = write_fixture(
                "unknown",
                &registry_with_nodes(&[node_with_repository("cynic", Some("ssh://x/y.git"))]),
            );
            let error = resolve_source_transport_uri(Some(&path), "other")
                .expect_err("unknown id must fail");
            assert!(error.contains("source_transport_missing:other"), "{error}");
        }

        #[test]
        fn node_without_layout_is_skipped_not_fatal() {
            let path = write_fixture(
                "skipped",
                &registry_with_nodes(&[
                    "{\"id\": \"plain\", \"name\": \"plain\"}".to_string(),
                    node_with_repository("cynic", Some("ssh://x/y.git")),
                ]),
            );
            let uri = resolve_source_transport_uri(Some(&path), "cynic")
                .expect("layout-less nodes must be skipped");
            assert_eq!(uri, "ssh://x/y.git");
        }

        #[test]
        fn ambiguous_declaration_fails_closed() {
            let path = write_fixture(
                "ambiguous",
                &registry_with_nodes(&[
                    node_with_repository("cynic", Some("ssh://a/y.git")),
                    node_with_repository("cynic", Some("ssh://b/y.git")),
                ]),
            );
            let error = resolve_source_transport_uri(Some(&path), "cynic")
                .expect_err("ambiguous declaration must fail");
            assert!(
                error.contains("source_transport_ambiguous:cynic"),
                "{error}"
            );
        }

        #[test]
        fn live_registry_declares_the_cynic_source_transport() {
            // Pins the trigger path to the live SSOT: the sovereign repository
            // must keep a declared clone URI or every build trigger fails loud.
            let path = format!("{}/../../infra/registry.json", env!("CARGO_MANIFEST_DIR"));
            let uri = resolve_source_transport_uri(Some(&path), "cynic")
                .expect("live registry must declare the cynic source transport");
            assert!(uri.starts_with("ssh://"), "{uri}");
            assert!(uri.ends_with("CYNIC.git"), "{uri}");
        }
    }
}
