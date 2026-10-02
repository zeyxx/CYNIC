//! Concrete implementations of the `cynic-vcs` port traits.
//!
//! These adapters bridge the L2 port traits to the L1 host substrate
//! (`crate::repo_access`, `crate::credential`, `crate::endpoint_observation`).
//! They are injected into `cynic-vcs` runners at runtime.

use std::path::Path;

use cynic_vcs::ports::{
    Credential, CredentialPort, CredentialType, EndpointObservation, EndpointProbePort,
    RepoAccessPort,
};
use cynic_vcs::repo_access::RepoAccessStatusReport;

use crate::credential::{
    Credential as HostCredential, CredentialResolver, CredentialType as HostCredentialType,
};
use crate::endpoint_observation::EndpointObservation as HostEndpointObservation;

/// `RepoAccessPort` implementation backed by `crate::repo_access`.
pub struct LocalRepoAccessPort;

impl RepoAccessPort for LocalRepoAccessPort {
    fn current_repo_access_status(&self) -> RepoAccessStatusReport {
        crate::repo_access::current_repo_access_status()
    }

    fn current_project_root(&self) -> Option<std::path::PathBuf> {
        crate::repo_access::current_project_root()
    }

    fn load_forgejo_desired_state(
        &self,
        canonical_project_root: &Path,
    ) -> Result<cynic_vcs::forgejo::ForgejoDesiredState, String> {
        crate::repo_access::load_forgejo_desired_state(canonical_project_root)
    }
}

/// `CredentialPort` implementation backed by `crate::credential::SecretFileResolver`.
pub struct LocalCredentialPort {
    resolver: crate::credential::SecretFileResolver,
}

impl LocalCredentialPort {
    pub fn new() -> Self {
        Self {
            resolver: crate::credential::SecretFileResolver::new(
                crate::credential::SecretFileResolver::default_path(),
            ),
        }
    }

    pub fn with_resolver(resolver: crate::credential::SecretFileResolver) -> Self {
        Self { resolver }
    }
}

impl Default for LocalCredentialPort {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialPort for LocalCredentialPort {
    fn resolve(&self, credential_ref: &str) -> Result<Credential, String> {
        self.resolver
            .resolve(credential_ref)
            .map(|cred| match cred {
                HostCredential::SshKey(path) => Credential::SshKey(path),
                HostCredential::Token(path) => Credential::Token(path),
            })
    }

    fn prompt_and_save(
        &self,
        credential_ref: &str,
        credential_type: CredentialType,
    ) -> Result<Credential, String> {
        let host_type = match credential_type {
            CredentialType::SshKey => HostCredentialType::SshKey,
            CredentialType::Token => HostCredentialType::GitHubPat,
        };
        self.resolver
            .prompt_and_save(credential_ref, host_type)
            .map(|cred| match cred {
                HostCredential::SshKey(path) => Credential::SshKey(path),
                HostCredential::Token(path) => Credential::Token(path),
            })
    }
}

/// `EndpointProbePort` implementation backed by `crate::endpoint_observation`.
pub struct LocalEndpointProbePort;

impl EndpointProbePort for LocalEndpointProbePort {
    fn observe_tcp_endpoint(
        &self,
        endpoint: &str,
        declared_port: Option<u16>,
    ) -> EndpointObservation {
        let obs: HostEndpointObservation =
            crate::endpoint_observation::observe_tcp_endpoint(endpoint, declared_port);
        EndpointObservation {
            host: obs.host.unwrap_or_default(),
            resolution_status: obs.resolution_status,
            resolved_addresses: obs.resolved_addresses,
            tcp_reachability_status: obs.tcp_reachability_status,
        }
    }
}
