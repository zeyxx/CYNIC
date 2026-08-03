use std::time::Duration;

use cynic_protocol::{CredentialLeaseDesiredState, CredentialLeaseObservation};

use crate::adapters::ed25519_credential_lease_verifier::Ed25519CredentialLeaseVerifier;
use crate::adapters::proxmox_control::ProxmoxControlAdapter;
use crate::ports::credential_broker_port::CredentialBrokerPort;
use crate::ports::credential_lease_verifier_port::CredentialLeaseVerifierPort;

/// Node-owned Proxmox credential broker with an explicit, injected proof
/// verifier.  The provider adapter remains responsible for material access;
/// this wrapper is the operational boundary that upgrades a local-file
/// observation into a signed lease observation.
#[derive(Debug)]
pub struct VerifiedCredentialBroker<V> {
    provider: ProxmoxControlAdapter,
    verifier: V,
}

impl<V> VerifiedCredentialBroker<V>
where
    V: CredentialLeaseVerifierPort,
{
    pub fn new(provider: ProxmoxControlAdapter, verifier: V) -> Self {
        Self { provider, verifier }
    }
}

impl VerifiedCredentialBroker<Ed25519CredentialLeaseVerifier> {
    pub fn from_env(
        timeout: Duration,
        authority_public_key_path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, String> {
        Ok(Self::new(
            ProxmoxControlAdapter::from_env(timeout)?,
            Ed25519CredentialLeaseVerifier::new(authority_public_key_path),
        ))
    }
}

impl<V> CredentialBrokerPort for VerifiedCredentialBroker<V>
where
    V: CredentialLeaseVerifierPort,
{
    fn observe(
        &self,
        desired: &CredentialLeaseDesiredState,
        _now_unix_seconds: u64,
    ) -> CredentialLeaseObservation {
        ProxmoxControlAdapter::observe_credential_lease_verified(desired, &self.verifier)
    }

    fn issue(&self, desired: &CredentialLeaseDesiredState) -> Result<(), String> {
        self.provider.issue_credential(desired)
    }

    fn renew(&self, desired: &CredentialLeaseDesiredState) -> Result<(), String> {
        self.provider.renew_credential(desired)
    }

    fn revoke(&self, desired: &CredentialLeaseDesiredState, generation: u64) -> Result<(), String> {
        self.provider.revoke_credential(desired, generation)
    }
}
