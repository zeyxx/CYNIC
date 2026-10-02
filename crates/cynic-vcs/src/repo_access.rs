//! Pure repository access data models — no I/O, no host dependencies.
//!
//! Extracted from `cynic-host/src/repo_access.rs` per Foundation §481 (SoC).
//! The I/O functions (`load_forgejo_desired_state`, `current_repo_access_status`,
//! `load_repo_access_entries`, `load_sovereign_remotes`, etc.) remain in
//! `cynic-host` and import these types from here.

use serde::{Deserialize, Serialize};

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
