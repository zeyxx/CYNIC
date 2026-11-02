//! Port traits abstracting L1 host substrate dependencies.
//!
//! These traits allow `cynic-vcs` (L2) to define the interfaces it needs
//! without depending on `cynic-host` (L1). The concrete implementations
//! (`LocalRepoAccessPort`, `LocalCredentialPort`, `LocalEndpointProbePort`)
//! live in `cynic-host` and are injected at runtime.

use std::path::Path;

use crate::repo_access::RepoAccessStatusReport;
use crate::{SourceMaterializationObservation, SourceMaterializationRequest, SourceRepositoryBootstrapObservation, SourceRepositoryBootstrapRequest};
use cynic_protocol::{LeaseGuard, LeaseVerification, RuntimeProofBundle};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMaterializationPortError {
    pub code: String,
    pub message: String,
}

impl SourceMaterializationPortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait SourceSnapshotAcquisitionPort: Send + Sync {
    fn acquire(
        &self,
        snapshot_digest: &str,
        snapshot_format: &str,
    ) -> Result<(), SourceMaterializationPortError>;
}

pub trait SourceMaterializationPort: Send + Sync {
    fn observe(
        &self,
        request: &SourceMaterializationRequest,
    ) -> Result<SourceMaterializationObservation, SourceMaterializationPortError>;

    fn materialize(
        &self,
        request: &SourceMaterializationRequest,
    ) -> Result<(), SourceMaterializationPortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRepositoryProofCommitError {
    pub code: String,
    pub message: String,
}

impl SourceRepositoryProofCommitError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait SourceRepositoryProofCommitPort {
    async fn claim(
        &self,
        agent_id: &str,
        target: &str,
    ) -> Result<LeaseGuard, SourceRepositoryProofCommitError>;

    async fn verify(
        &self,
        guard: &LeaseGuard,
    ) -> Result<LeaseVerification, SourceRepositoryProofCommitError>;

    async fn finalize(
        &self,
        guard: &LeaseGuard,
        bundle: &RuntimeProofBundle,
    ) -> Result<String, SourceRepositoryProofCommitError>;

    async fn release(&self, guard: &LeaseGuard) -> Result<(), SourceRepositoryProofCommitError>;
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRepositoryLayout {
    pub repository_root: std::path::PathBuf,
    pub required_principal: String,
}

impl SourceRepositoryLayout {
    pub fn new(repository_root: impl Into<std::path::PathBuf>, required_principal: impl Into<String>) -> Self {
        Self {
            repository_root: repository_root.into(),
            required_principal: required_principal.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRepositoryBootstrapPortError {
    pub code: String,
    pub message: String,
}

impl SourceRepositoryBootstrapPortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait SourceRepositoryBootstrapPort: Send + Sync {
    fn observe(
        &self,
        layout: &SourceRepositoryLayout,
        request: &SourceRepositoryBootstrapRequest,
    ) -> Result<SourceRepositoryBootstrapObservation, SourceRepositoryBootstrapPortError>;

    fn apply(
        &self,
        layout: &SourceRepositoryLayout,
        request: &SourceRepositoryBootstrapRequest,
    ) -> Result<(), SourceRepositoryBootstrapPortError>;
}
