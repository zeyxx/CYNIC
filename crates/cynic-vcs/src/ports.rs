//! Port traits abstracting L1 host substrate dependencies.
//!
//! These traits allow `cynic-vcs` (L2) to define the interfaces it needs
//! without depending on `cynic-host` (L1). The concrete implementations
//! (`LocalRepoAccessPort`, `LocalCredentialPort`, `LocalEndpointProbePort`)
//! live in `cynic-host` and are injected at runtime.

use std::path::Path;

use crate::repo_access::RepoAccessStatusReport;

/// Port for repository access status observation.
///
/// Abstracts `crate::repo_access::current_repo_access_status` and
/// `crate::repo_access::load_forgejo_desired_state` from `cynic-host`.
pub trait RepoAccessPort: Send + Sync {
    /// Returns the current repository access status report.
    fn current_repo_access_status(&self) -> RepoAccessStatusReport;

    /// Returns the canonical project root path, if discoverable.
    fn current_project_root(&self) -> Option<std::path::PathBuf>;

    /// Loads the Forgejo desired state from the registry authority.
    fn load_forgejo_desired_state(
        &self,
        canonical_project_root: &Path,
    ) -> Result<crate::forgejo::ForgejoDesiredState, String>;
}

/// Port for credential resolution.
///
/// Abstracts `crate::credential::CredentialResolver` and
/// `crate::credential::SecretFileResolver` from `cynic-host`.
pub trait CredentialPort: Send + Sync {
    /// Resolves a credential by reference path.
    fn resolve(&self, credential_ref: &str) -> Result<Credential, String>;

    /// Prompts for and saves a credential if not found.
    fn prompt_and_save(
        &self,
        credential_ref: &str,
        credential_type: CredentialType,
    ) -> Result<Credential, String>;
}

/// Credential types mirrored from `cynic-host::credential`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialType {
    SshKey,
    Token,
}

/// Credential handle mirrored from `cynic-host::credential::Credential`.
#[derive(Debug, Clone)]
pub enum Credential {
    SshKey(std::path::PathBuf),
    Token(std::path::PathBuf),
}

/// Port for TCP endpoint observation.
///
/// Abstracts `crate::endpoint_observation::observe_tcp_endpoint` from `cynic-host`.
pub trait EndpointProbePort: Send + Sync {
    /// Observes a TCP endpoint and returns its resolution and reachability status.
    fn observe_tcp_endpoint(
        &self,
        endpoint: &str,
        declared_port: Option<u16>,
    ) -> EndpointObservation;
}

/// Endpoint observation result mirrored from `cynic-host::endpoint_observation`.
#[derive(Debug, Clone)]
pub struct EndpointObservation {
    pub host: String,
    pub resolution_status: String,
    pub resolved_addresses: Vec<String>,
    pub tcp_reachability_status: String,
}
