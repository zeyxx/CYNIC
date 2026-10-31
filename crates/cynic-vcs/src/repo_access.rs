use serde::Serialize;
// Re-export pure repo access types from `cynic-vcs` per Foundation §481 (SoC).
//
// The I/O functions (`load_forgejo_desired_state`, `current_repo_access_status`,
// `load_repo_access_entries`, `load_sovereign_remotes`, etc.) remain here in
// `cynic-host` because they depend on `crate::endpoint_observation` and
// `crate::service_catalog` (L1 host substrate).

use crate::{ForgejoDesiredState, ForgejoRepositoryPermission};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};


// Re-export the pure types so existing `crate::repo_access::` paths keep resolving.

#[derive(Debug, Deserialize)]
struct RepoAccessProfileDocument {
    #[allow(dead_code)]
    schema: Option<String>,
    repositories: Option<Vec<RepoAccessEntryRaw>>,
}

#[derive(Debug, Deserialize)]
struct RepoRemotesDocument {
    remotes: Option<Vec<RepoRemoteRaw>>,
}

#[derive(Debug, Deserialize)]
struct RepoRemoteRaw {
    id: Option<String>,
    role: Option<String>,
    owner: Option<String>,
    repository: Option<String>,
    api_url: Option<String>,
    url: Option<String>,
    desired_git_url: Option<String>,
    git_user: Option<String>,
    machine_principal: Option<String>,
    repository_permission: Option<ForgejoRepositoryPermission>,
    backend_service_ref: Option<ServiceRef>,
    admin_service_ref: Option<ServiceRef>,
    git_service_ref: Option<ServiceRef>,
    machine_key_fingerprint: Option<String>,
    ssh_host_key_fingerprint: Option<String>,
    http_port: Option<u16>,
    host_ip: Option<String>,
    host_aliases: Option<Vec<String>>,
    status_observed: Option<String>,
    notes: Option<Vec<String>>,
}


// Pure repository access data models — no I/O, no host dependencies.
//
// Extracted from `cynic-host/src/repo_access.rs` per Foundation §481 (SoC).
// The I/O functions (`load_forgejo_desired_state`, `current_repo_access_status`,
// `load_repo_access_entries`, `load_sovereign_remotes`, etc.) remain in
// `cynic-host` and import these types from here.


pub const REPO_ACCESS_STATUS_SCHEMA: &str = "cynic.runtime_principals.repo_access.status.v1";
pub const REPO_REMOTES_AUTHORITY_REL: &str = "infra/repo-remotes.json";
pub const REGISTRY_AUTHORITY_REL: &str = "infra/registry.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepoAccessEntry {
    pub path: String,
    pub owner: Option<String>,
    pub added_by: Option<String>,
    pub discovered: bool,
    pub added_at: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoAccessReadiness {
    pub path: String,
    pub owner: Option<String>,
    pub source: String,
    pub discovered: bool,
    pub added_by: Option<String>,
    pub added_at: String,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoAccessStatusReport {
    pub schema: &'static str,
    pub mutates: bool,
    pub profile_path: String,
    pub profile_exists: bool,
    pub canonical_project_root: Option<String>,
    pub known_repos: Vec<String>,
    pub sovereign_remotes: Vec<SovereignRemoteReadiness>,
    pub entries: Vec<RepoAccessReadiness>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SovereignRemoteReadiness {
    pub id: String,
    pub role: String,
    pub owner: Option<String>,
    pub repository: Option<String>,
    pub api_url: Option<String>,
    pub api_host: Option<String>,
    pub api_resolved_addresses: Vec<String>,
    pub api_resolution_status: String,
    pub api_tcp_reachability_status: String,
    pub url: String,
    pub desired_git_url: Option<String>,
    pub http_port: Option<u16>,
    pub endpoint_host: Option<String>,
    pub resolved_addresses: Vec<String>,
    pub desired_git_host: Option<String>,
    pub desired_git_resolved_addresses: Vec<String>,
    pub desired_git_resolution_status: String,
    pub machine_key_fingerprint: Option<String>,
    pub ssh_host_key_fingerprint: Option<String>,
    pub host_ip: Option<String>,
    pub host_aliases: Vec<String>,
    pub host_resolution_status: String,
    pub tcp_reachability_status: String,
    pub service_health_status: String,
    pub repo_auth_readiness: String,
    pub status_observed: Option<String>,
    pub notes: Vec<String>,
}


#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ServiceRef {
    pub node: String,
    pub service: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedServiceEndpoint {
    pub service_ref: ServiceRef,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub endpoint: String,
}

#[derive(Debug, Deserialize)]
struct Registry {
    network_policy: Option<NetworkPolicy>,
    nodes: Option<Vec<Node>>,
}

#[derive(Debug, Deserialize)]
struct NetworkPolicy {
    reservations: Option<Vec<Reservation>>,
}

#[derive(Debug, Deserialize)]
struct Reservation {
    name: Option<String>,
    dns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Node {
    id: Option<String>,
    name: Option<String>,
    services: Option<Vec<Service>>,
}

#[derive(Debug, Deserialize)]
struct Service {
    name: Option<String>,
    scheme: Option<String>,
    port: Option<u16>,
}

pub fn resolve_service_ref(
    registry_path: &Path,
    reference: &ServiceRef,
) -> Result<ResolvedServiceEndpoint, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("read {}: {error}", registry_path.display()))?;
    let registry: Registry = serde_json::from_str(&raw)
        .map_err(|error| format!("parse {}: {error}", registry_path.display()))?;
    let nodes = registry.nodes.unwrap_or_default();
    let node = nodes
        .iter()
        .find(|node| {
            node.id.as_deref() == Some(reference.node.as_str())
                || node.name.as_deref() == Some(reference.node.as_str())
        })
        .ok_or_else(|| format!("service node not declared: {}", reference.node))?;
    let service = node
        .services
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|service| service.name.as_deref() == Some(reference.service.as_str()))
        .ok_or_else(|| {
            format!(
                "service not declared: {}/{}",
                reference.node, reference.service
            )
        })?;
    let scheme = required(service.scheme.clone(), "service.scheme")?;
    let port = service
        .port
        .ok_or_else(|| "service.port missing".to_string())?;
    let host = registry
        .network_policy
        .and_then(|policy| policy.reservations)
        .unwrap_or_default()
        .into_iter()
        .find(|reservation| reservation.name.as_deref() == Some(reference.node.as_str()))
        .and_then(|reservation| reservation.dns)
        .and_then(|names| names.into_iter().find(|name| !name.trim().is_empty()))
        .ok_or_else(|| format!("service DNS reservation missing: {}", reference.node))?;
    let endpoint = if default_port(&scheme) == Some(port) {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}:{port}")
    };
    Ok(ResolvedServiceEndpoint {
        service_ref: reference.clone(),
        scheme,
        host,
        port,
        endpoint,
    })
}

fn required(value: Option<String>, field: &str) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} missing"))
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        "ssh" => Some(22),
        _ => None,
    }
}


pub fn load_forgejo_desired_state(
    canonical_project_root: &Path,
) -> Result<ForgejoDesiredState, String> {
    let path = canonical_project_root.join(REPO_REMOTES_AUTHORITY_REL);
    let raw =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let document: RepoRemotesDocument =
        serde_json::from_str(&raw).map_err(|error| format!("parse {}: {error}", path.display()))?;
    let remote = document
        .remotes
        .unwrap_or_default()
        .into_iter()
        .find(|remote| remote.id.as_deref() == Some("forgejo"))
        .ok_or_else(|| "forgejo remote authority missing".to_string())?;
    let admin = resolve_service_ref(
        &canonical_project_root.join(REGISTRY_AUTHORITY_REL),
        &remote
            .admin_service_ref
            .clone()
            .ok_or_else(|| "forgejo admin_service_ref missing".to_string())?,
    )?;
    let git = resolve_service_ref(
        &canonical_project_root.join(REGISTRY_AUTHORITY_REL),
        &remote
            .git_service_ref
            .clone()
            .ok_or_else(|| "forgejo git_service_ref missing".to_string())?,
    )?;
    let owner = required(remote.owner, "owner")?;
    let repository = required(remote.repository, "repository")?;
    let git_user = required(remote.git_user, "git_user")?;

    Ok(ForgejoDesiredState {
        instance_id: required(remote.id, "id")?,
        owner: owner.clone(),
        repository: repository.clone(),
        api_url: format!("{}/api/v1", admin.endpoint),
        git_ssh_url: format!(
            "ssh://{git_user}@{}:{}/{owner}/{repository}.git",
            git.host, git.port,
        ),
        machine_principal: required(remote.machine_principal, "machine_principal")?,
        repository_permission: remote
            .repository_permission
            .ok_or_else(|| "forgejo authority field missing: repository_permission".to_string())?,
        machine_key_fingerprint: required(
            remote.machine_key_fingerprint,
            "machine_key_fingerprint",
        )?,
    })
}


#[derive(Debug, Deserialize)]
struct RepoAccessEntryRaw {
    path: Option<String>,
    owner: Option<String>,
    added_by: Option<String>,
    discovered: Option<bool>,
    added_at: Option<String>,
    source: Option<String>,
}

pub fn current_repo_access_status() -> RepoAccessStatusReport {
    let profile_path = current_repo_access_profile_path();
    let canonical_project_root = current_project_root();
    repo_access_status(&profile_path, canonical_project_root.as_deref())
}

pub fn current_repo_access_profile_path() -> PathBuf {
    env::var("CYNIC_REPO_ACCESS_PROFILE")
        .map(PathBuf::from)
        .ok()
        .or_else(|| current_project_root().map(|root| root.join("infra/repo-access-profile.json")))
        .unwrap_or_else(|| PathBuf::from("infra/repo-access-profile.json"))
}

pub fn current_project_root() -> Option<PathBuf> {
    current_project_root_from(env::current_dir().ok(), env::current_exe().ok())
}

pub fn repo_access_status(
    profile_path: &Path,
    canonical_project_root: Option<&Path>,
) -> RepoAccessStatusReport {
    let entries = load_repo_access_entries(profile_path);
    let sovereign_remotes = load_sovereign_remotes(canonical_project_root);
    let canonical = canonical_project_root
        .and_then(resolve_existing_dir)
        .map(path_to_string);

    let mut known_repos = canonical.iter().cloned().collect::<Vec<_>>();
    for entry in &entries {
        if let Some(resolved) = resolve_existing_dir(Path::new(&entry.path)) {
            let resolved_text = path_to_string(resolved);
            if canonical.as_deref() == Some(resolved_text.as_str()) {
                continue;
            }
            known_repos.push(resolved_text);
        }
    }
    known_repos.sort();
    known_repos.dedup();

    let readiness = entries
        .into_iter()
        .map(|entry| {
            let repo_path = PathBuf::from(&entry.path);
            RepoAccessReadiness {
                path: path_to_string(&repo_path),
                owner: entry.owner,
                source: entry.source,
                discovered: entry.discovered,
                added_by: entry.added_by,
                added_at: entry.added_at,
                exists: repo_path.exists(),
            }
        })
        .collect::<Vec<_>>();

    RepoAccessStatusReport {
        schema: REPO_ACCESS_STATUS_SCHEMA,
        mutates: false,
        profile_path: path_to_string(profile_path),
        profile_exists: profile_path.is_file(),
        canonical_project_root: canonical,
        known_repos,
        sovereign_remotes,
        entries: readiness,
    }
}

pub fn load_repo_access_entries(profile_path: &Path) -> Vec<RepoAccessEntry> {
    let raw = match fs::read_to_string(profile_path) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    let document = match serde_json::from_str::<RepoAccessProfileDocument>(&raw) {
        Ok(document) => document,
        Err(_) => return Vec::new(),
    };
    let repositories = document.repositories.unwrap_or_default();
    let mut entries = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in repositories {
        let Some(path) = item
            .path
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let normalized = normalize_existing_or_literal(Path::new(&path));
        if !seen.insert(normalized.clone()) {
            continue;
        }
        entries.push(RepoAccessEntry {
            path: normalized,
            owner: none_if_empty(item.owner),
            added_by: none_if_empty(item.added_by),
            discovered: item.discovered.unwrap_or(false),
            added_at: item
                .added_at
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(default_added_at),
            source: item
                .source
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "sync".to_string()),
        });
    }
    entries
}

fn load_sovereign_remotes(_canonical_project_root: Option<&Path>) -> Vec<SovereignRemoteReadiness> {
    Vec::new()
}

fn find_project_root(start: Option<&Path>) -> Option<PathBuf> {
    let start = start?;
    let initial = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
    for candidate in initial.ancestors() {
        if candidate.join("Cargo.toml").is_file()
            && candidate.join("contracts/foundation.json").is_file()
            && candidate.join("infra").is_dir()
            && candidate.join("crates").is_dir()
        {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

fn current_project_root_from(
    current_dir: Option<PathBuf>,
    current_exe: Option<PathBuf>,
) -> Option<PathBuf> {
    find_project_root(current_dir.as_deref()).or_else(|| find_project_root(current_exe.as_deref()))
}

fn resolve_existing_dir(path: &Path) -> Option<PathBuf> {
    let resolved = path.canonicalize().ok()?;
    if resolved.is_dir() {
        Some(resolved)
    } else {
        None
    }
}

fn normalize_existing_or_literal(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .map(path_to_string)
        .unwrap_or_else(|| path_to_string(path))
}

fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

fn none_if_empty(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn default_added_at() -> String {
    "1970-01-01T00:00:00+00:00".to_string()
}


