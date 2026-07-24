use cynic_protocol::{ForgejoDesiredState, ForgejoRepositoryPermission};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::endpoint_observation::observe_tcp_endpoint;
use crate::service_catalog::{ServiceRef, resolve_service_ref};

const REPO_ACCESS_STATUS_SCHEMA: &str = "cynic.runtime_principals.repo_access.status.v1";
const REPO_REMOTES_AUTHORITY_REL: &str = "infra/repo-remotes.json";
const REGISTRY_AUTHORITY_REL: &str = "infra/registry.json";

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

fn required(value: Option<String>, field: &str) -> Result<String, String> {
    none_if_empty(value).ok_or_else(|| format!("forgejo authority field missing: {field}"))
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

fn load_sovereign_remotes(canonical_project_root: Option<&Path>) -> Vec<SovereignRemoteReadiness> {
    let Some(root) = canonical_project_root else {
        return Vec::new();
    };
    let path = root.join(REPO_REMOTES_AUTHORITY_REL);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    let document = match serde_json::from_str::<RepoRemotesDocument>(&raw) {
        Ok(document) => document,
        Err(_) => return Vec::new(),
    };
    document
        .remotes
        .unwrap_or_default()
        .into_iter()
        .filter_map(|remote| {
            let id = none_if_empty(remote.id)?;
            let role = none_if_empty(remote.role).unwrap_or_else(|| "unknown".to_string());
            let owner = none_if_empty(remote.owner.clone());
            let repository = none_if_empty(remote.repository.clone());
            let (url, api_url, desired_git_url, http_port, host_aliases) = if id == "forgejo" {
                let backend = resolve_service_ref(
                    &root.join(REGISTRY_AUTHORITY_REL),
                    remote.backend_service_ref.as_ref()?,
                )
                .ok()?;
                let admin = resolve_service_ref(
                    &root.join(REGISTRY_AUTHORITY_REL),
                    remote.admin_service_ref.as_ref()?,
                )
                .ok()?;
                let git = resolve_service_ref(
                    &root.join(REGISTRY_AUTHORITY_REL),
                    remote.git_service_ref.as_ref()?,
                )
                .ok()?;
                let owner = owner.as_deref()?;
                let repository = repository.as_deref()?;
                let git_user = none_if_empty(remote.git_user.clone())?;
                (
                    format!("{}/{owner}/{repository}.git", backend.endpoint),
                    Some(format!("{}/api/v1", admin.endpoint)),
                    Some(format!(
                        "ssh://{git_user}@{}:{}/{owner}/{repository}.git",
                        git.host, git.port,
                    )),
                    Some(backend.port),
                    vec![backend.host, admin.host, git.host],
                )
            } else {
                (
                    none_if_empty(remote.url.clone())?,
                    none_if_empty(remote.api_url.clone()),
                    none_if_empty(remote.desired_git_url.clone()),
                    remote.http_port,
                    remote.host_aliases.clone().unwrap_or_default(),
                )
            };
            Some(SovereignRemoteReadiness {
                endpoint_host: endpoint_host(&url),
                resolved_addresses: resolved_address_strings(&url, remote.http_port),
                desired_git_host: desired_git_url.as_deref().and_then(endpoint_host),
                desired_git_resolved_addresses: desired_git_url
                    .as_deref()
                    .map(|url| resolved_address_strings(url, None))
                    .unwrap_or_default(),
                desired_git_resolution_status: desired_git_url
                    .as_deref()
                    .map(|url| host_resolution_status(url, None))
                    .unwrap_or_else(|| "undeclared".to_string()),
                id,
                role,
                owner,
                repository,
                api_host: api_url.as_deref().and_then(endpoint_host),
                api_resolved_addresses: api_url
                    .as_deref()
                    .map(|url| resolved_address_strings(url, None))
                    .unwrap_or_default(),
                api_resolution_status: api_url
                    .as_deref()
                    .map(|url| host_resolution_status(url, None))
                    .unwrap_or_else(|| "undeclared".to_string()),
                api_tcp_reachability_status: api_url
                    .as_deref()
                    .map(|url| tcp_reachability_status(url, None))
                    .unwrap_or_else(|| "undeclared".to_string()),
                api_url,
                url: url.clone(),
                desired_git_url,
                machine_key_fingerprint: none_if_empty(remote.machine_key_fingerprint),
                ssh_host_key_fingerprint: none_if_empty(remote.ssh_host_key_fingerprint),
                http_port,
                host_ip: none_if_empty(remote.host_ip),
                host_aliases: host_aliases
                    .into_iter()
                    .filter_map(|alias| none_if_empty(Some(alias)))
                    .collect(),
                host_resolution_status: host_resolution_status(&url, http_port),
                tcp_reachability_status: tcp_reachability_status(&url, http_port),
                service_health_status: service_health_status(&url, http_port),
                repo_auth_readiness: repo_auth_readiness(none_if_empty(
                    remote.status_observed.clone(),
                )),
                status_observed: none_if_empty(remote.status_observed),
                notes: remote.notes.unwrap_or_default(),
            })
        })
        .collect()
}

fn endpoint_host(url: &str) -> Option<String> {
    observe_tcp_endpoint(url, None).host
}

fn host_resolution_status(url: &str, declared_http_port: Option<u16>) -> String {
    observe_tcp_endpoint(url, declared_http_port).resolution_status
}

fn resolved_address_strings(url: &str, declared_port: Option<u16>) -> Vec<String> {
    observe_tcp_endpoint(url, declared_port).resolved_addresses
}

fn tcp_reachability_status(url: &str, declared_http_port: Option<u16>) -> String {
    observe_tcp_endpoint(url, declared_http_port).tcp_reachability_status
}

fn service_health_status(url: &str, declared_http_port: Option<u16>) -> String {
    let tcp = tcp_reachability_status(url, declared_http_port);
    if tcp == "open" {
        "reachable".to_string()
    } else {
        "unreachable".to_string()
    }
}

fn repo_auth_readiness(status_observed: Option<String>) -> String {
    match status_observed.as_deref() {
        Some(status) if status.contains("auth_required") || status.contains("repo_missing") => {
            "auth_or_repo_action_required".to_string()
        }
        Some(status) if status.contains("reachable") => "service_only_observed".to_string(),
        Some(_) => "unknown".to_string(),
        None => "unknown".to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::{
        REPO_ACCESS_STATUS_SCHEMA, current_project_root_from, endpoint_host,
        host_resolution_status, load_repo_access_entries, repo_access_status,
        service_health_status, tcp_reachability_status,
    };
    use std::fs;
    use std::net::TcpListener;
    use std::path::Path;

    #[test]
    fn load_repo_access_entries_dedupes_and_normalizes() {
        let tempdir = tempfile::tempdir().unwrap();
        let repo_a = tempdir.path().join("repo-a");
        let repo_b = tempdir.path().join("repo-b");
        fs::create_dir_all(&repo_a).unwrap();
        fs::create_dir_all(&repo_b).unwrap();
        let profile = tempdir.path().join("repo-access.json");
        fs::write(
            &profile,
            format!(
                r#"{{
  "schema": "cynic.runtime_principals.repo_access.v1",
  "repositories": [
    {{"path": "{}", "owner": "alice", "added_by": "apply", "discovered": true, "added_at": "2026-01-01T00:00:00+00:00", "source": "sync"}},
    {{"path": "{}", "owner": "alice", "added_by": "apply", "discovered": true, "added_at": "2026-01-01T00:00:00+00:00", "source": "sync"}},
    {{"path": "{}", "owner": null, "added_by": "", "discovered": false, "added_at": "", "source": ""}}
  ]
}}"#,
                repo_a.display(),
                repo_a.display(),
                repo_b.display()
            ),
        )
        .unwrap();

        let entries = load_repo_access_entries(&profile);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].owner.as_deref(), Some("alice"));
        assert_eq!(entries[1].source, "sync");
        assert_eq!(entries[1].added_at, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn repo_access_status_reports_known_repos() {
        let tempdir = tempfile::tempdir().unwrap();
        let canonical = tempdir.path().join("canonical");
        let extra = tempdir.path().join("extra");
        fs::create_dir_all(&canonical).unwrap();
        fs::create_dir_all(&extra).unwrap();
        let profile = tempdir.path().join("repo-access.json");
        fs::write(
            &profile,
            format!(
                r#"{{
  "schema": "cynic.runtime_principals.repo_access.v1",
  "repositories": [
    {{"path": "{}", "owner": "alice", "added_by": "apply", "discovered": true, "added_at": "2026-01-01T00:00:00+00:00", "source": "sync"}}
  ]
}}"#,
                extra.display()
            ),
        )
        .unwrap();

        let report = repo_access_status(&profile, Some(&canonical));
        assert_eq!(report.schema, REPO_ACCESS_STATUS_SCHEMA);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.known_repos.len(), 2);
        assert!(
            report
                .known_repos
                .iter()
                .any(|item| item == &canonical.to_string_lossy())
        );
        assert!(
            report
                .known_repos
                .iter()
                .any(|item| item == &extra.to_string_lossy())
        );
        assert!(report.sovereign_remotes.is_empty());
    }

    #[test]
    fn repo_access_status_exposes_sovereign_forgejo_remote_authority() {
        let tempdir = tempfile::tempdir().unwrap();
        let canonical = tempdir.path().join("canonical");
        fs::create_dir_all(canonical.join("infra")).unwrap();
        let profile = tempdir.path().join("repo-access.json");
        fs::write(
            &profile,
            r#"{"schema":"cynic.runtime_principals.repo_access.v1","repositories":[]}"#,
        )
        .unwrap();
        fs::write(
            canonical.join("infra/registry.json"),
            r#"{
  "network_policy":{"reservations":[{"name":"forgejo","dns":["forgejo.synapse.cynic"]}]},
  "nodes":[{"id":"forgejo","services":[
    {"name":"ssh","scheme":"ssh","port":22},
    {"name":"forgejo-backend","scheme":"http","port":3000},
    {"name":"forgejo-api-tls","scheme":"https","port":443}
  ]}]
}"#,
        )
        .unwrap();
        fs::write(
            canonical.join("infra/repo-remotes.json"),
            r#"{
  "schema": "cynic.repo_remotes.v1",
  "remotes": [
    {
      "id": "forgejo",
      "role": "sovereign_candidate",
      "owner": "jeanterre13",
      "repository": "CYNIC",
      "git_user": "forgejo",
      "machine_principal": "cynic-cortex",
      "repository_permission": "write",
      "backend_service_ref":{"node":"forgejo","service":"forgejo-backend"},
      "admin_service_ref":{"node":"forgejo","service":"forgejo-api-tls"},
      "git_service_ref":{"node":"forgejo","service":"ssh"},
      "status_observed": "reachable_http_ui_repo_missing_or_auth_required",
      "notes": ["Forgejo serves on 3000."]
    }
  ]
}"#,
        )
        .unwrap();

        let report = repo_access_status(&profile, Some(&canonical));
        assert_eq!(report.sovereign_remotes.len(), 1);
        let forgejo = &report.sovereign_remotes[0];
        assert_eq!(forgejo.id, "forgejo");
        assert_eq!(forgejo.role, "sovereign_candidate");
        assert_eq!(forgejo.http_port, Some(3000));
        assert_eq!(
            forgejo.desired_git_url.as_deref(),
            Some("ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git")
        );
        assert_eq!(
            forgejo.endpoint_host.as_deref(),
            Some("forgejo.synapse.cynic")
        );
        assert_eq!(
            forgejo.desired_git_host.as_deref(),
            Some("forgejo.synapse.cynic")
        );
        assert!(!forgejo.desired_git_resolution_status.is_empty());
        assert_eq!(forgejo.host_ip, None);
        assert!(!forgejo.host_resolution_status.is_empty());
        assert!(!forgejo.tcp_reachability_status.is_empty());
        assert!(!forgejo.service_health_status.is_empty());
        assert_eq!(forgejo.repo_auth_readiness, "auth_or_repo_action_required");
    }

    #[test]
    fn current_project_root_discovers_from_current_dir() {
        let tempdir = tempfile::tempdir().unwrap();
        let root = tempdir.path().join("root");
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::create_dir_all(root.join("crates")).unwrap();
        fs::create_dir_all(root.join("infra")).unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        fs::write(root.join("contracts/foundation.json"), "{}").unwrap();
        let current = current_project_root_from(Some(root.clone()), None);
        assert_eq!(current.as_deref(), Some(Path::new(&root)));
    }

    #[test]
    fn endpoint_host_extracts_host_from_http_remote() {
        assert_eq!(
            endpoint_host("http://10.10.10.101:3000/jeanterre13/CYNIC.git").as_deref(),
            Some("10.10.10.101")
        );
    }

    #[test]
    fn endpoint_host_extracts_host_from_ssh_remote() {
        assert_eq!(
            endpoint_host("ssh://git@forgejo.synapse.cynic:22/owner/repo.git").as_deref(),
            Some("forgejo.synapse.cynic")
        );
    }

    #[test]
    fn sovereign_remote_health_observes_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://127.0.0.1:{}/repo.git", addr.port());
        assert_eq!(host_resolution_status(&url, Some(addr.port())), "resolved");
        assert_eq!(tcp_reachability_status(&url, Some(addr.port())), "open");
    }

    #[test]
    fn stale_declared_status_cannot_override_failed_runtime_probe() {
        assert_eq!(
            service_health_status("http://127.0.0.1:1/repo.git", Some(1)),
            "unreachable"
        );
    }
}
