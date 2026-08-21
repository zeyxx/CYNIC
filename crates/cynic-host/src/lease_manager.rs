//! Cryptographic lease and monotonic fencing manager for the CYNIC Trust-Chain vertical.
//!
//! Provides the power-atomicity guarantee:
//! 1. Authority requires an active, unexpired cryptographic lease.
//! 2. If the lease expires by a single millisecond, authority drops instantly.
//! 3. All storage writes require a strictly monotonic fencing token.
//! 4. Role transitions (Takeovers) are recorded as signed, sequenced records.

use std::sync::Arc;

use cynic_protocol::runtime_takeover::{CryptographicLease, RoleTransitionRecord};

use crate::crypto::{CryptoError, CryptoProvider, Ed25519Provider};

/// Manager for cryptographic leases and monotonic fencing tokens.
pub struct LeaseManager {
    crypto: Arc<dyn CryptoProvider>,
    authority_public_key: Option<Vec<u8>>,
    current_lease: Option<CryptographicLease>,
    fencing_token_watermark: u64,
    transition_history: Vec<RoleTransitionRecord>,
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self {
            crypto: Arc::new(Ed25519Provider),
            authority_public_key: None,
            current_lease: None,
            fencing_token_watermark: 0,
            transition_history: Vec::new(),
        }
    }
}

impl LeaseManager {
    pub fn new(crypto: Arc<dyn CryptoProvider>, authority_public_key: Option<Vec<u8>>) -> Self {
        Self {
            crypto,
            authority_public_key,
            current_lease: None,
            fencing_token_watermark: 0,
            transition_history: Vec::new(),
        }
    }

    /// Sets the trusted authority public key.
    pub fn set_authority_public_key(&mut self, key: Vec<u8>) {
        self.authority_public_key = Some(key);
    }

    /// Returns the currently held lease, if any.
    pub fn current_lease(&self) -> Option<&CryptographicLease> {
        self.current_lease.as_ref()
    }

    /// Returns the highest known fencing token watermark.
    pub fn fencing_token_watermark(&self) -> u64 {
        self.fencing_token_watermark
    }

    /// Returns the audit history of all accepted role transitions.
    pub fn transition_history(&self) -> &[RoleTransitionRecord] {
        &self.transition_history
    }

    /// Checks whether this node currently possesses active, unexpired authority.
    ///
    /// If the lease TTL has elapsed, authority drops instantly without waiting for network gossip.
    pub fn has_active_authority(&self, node_id: &str, now_unix_ms: u64) -> bool {
        match &self.current_lease {
            Some(lease) => lease.holder_node_id == node_id && lease.is_active_at(now_unix_ms),
            None => false,
        }
    }

    /// Issues a new signed CryptographicLease (used by the authority plane / Kernel).
    pub fn issue_lease(
        &self,
        lease_id: &str,
        holder_node_id: &str,
        role: &str,
        fencing_token: u64,
        ttl_ms: u64,
        authority_id: &str,
        authority_private_key: &[u8],
        now_unix_ms: u64,
    ) -> Result<CryptographicLease, CryptoError> {
        let dummy = CryptographicLease::new(
            lease_id.to_string(),
            holder_node_id.to_string(),
            role.to_string(),
            fencing_token,
            now_unix_ms,
            ttl_ms,
            authority_id.to_string(),
            String::new(),
        );

        let signing_bytes = dummy.signing_bytes();
        let signature = self.crypto.sign(&signing_bytes, authority_private_key)?;
        let signature_hex = hex::encode(signature);

        let lease = CryptographicLease::new(
            lease_id.to_string(),
            holder_node_id.to_string(),
            role.to_string(),
            fencing_token,
            now_unix_ms,
            ttl_ms,
            authority_id.to_string(),
            signature_hex,
        );

        let blockers = lease.validate(now_unix_ms);
        if !blockers.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "issued lease is invalid: {blockers:?}"
            )));
        }

        Ok(lease)
    }

    /// Adopts or renews a cryptographic lease, verifying authority and strictly monotonic fencing.
    pub fn acquire_or_renew_lease(
        &mut self,
        lease: CryptographicLease,
        now_unix_ms: u64,
    ) -> Result<(), CryptoError> {
        let blockers = lease.validate(now_unix_ms);
        if !blockers.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "cannot adopt invalid lease: {blockers:?}"
            )));
        }

        // Verify authority signature if key is configured
        if let Some(pubkey) = &self.authority_public_key {
            let signature = hex::decode(&lease.signature_hex)
                .map_err(|e| CryptoError::InvalidKey(format!("invalid hex signature: {e}")))?;
            let signing_bytes = lease.signing_bytes();
            let valid = self.crypto.verify(&signing_bytes, &signature, pubkey)?;
            if !valid {
                return Err(CryptoError::OperationFailed(
                    "lease signature verification failed".to_string(),
                ));
            }
        }

        // Strict monotonicity check for fencing tokens
        if lease.fencing_token <= self.fencing_token_watermark {
            return Err(CryptoError::OperationFailed(format!(
                "fencing token {} is not strictly greater than watermark {}",
                lease.fencing_token, self.fencing_token_watermark
            )));
        }

        // Record the role transition
        let from_node_id = self
            .current_lease
            .as_ref()
            .map(|l| l.holder_node_id.clone());
        let transition = RoleTransitionRecord::new(
            format!("tx-{}", lease.fencing_token),
            from_node_id,
            lease.holder_node_id.clone(),
            lease.role.clone(),
            lease.fencing_token,
            self.fencing_token_watermark,
            now_unix_ms,
            lease.signature_hex.clone(),
        );

        self.fencing_token_watermark = lease.fencing_token;
        self.current_lease = Some(lease);
        self.transition_history.push(transition);

        Ok(())
    }

    /// Verifies that a storage mutation with a proposed fencing token is permitted.
    pub fn assert_fenced_write(
        &self,
        node_id: &str,
        proposed_fencing_token: u64,
        now_unix_ms: u64,
    ) -> Result<(), CryptoError> {
        if !self.has_active_authority(node_id, now_unix_ms) {
            return Err(CryptoError::OperationFailed(
                "fenced write rejected: node does not possess active cryptographic authority"
                    .to_string(),
            ));
        }

        if proposed_fencing_token < self.fencing_token_watermark {
            return Err(CryptoError::OperationFailed(format!(
                "fenced write rejected: token {proposed_fencing_token} is lower than watermark {}",
                self.fencing_token_watermark
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lease_lifecycle_and_instant_expiry() {
        let provider = Ed25519Provider;
        let (auth_secret, auth_public) = provider.generate_keypair().expect("keys");

        let mut manager = LeaseManager::new(Arc::new(provider), Some(auth_public));

        let now = 1_000_000;
        let lease = manager
            .issue_lease(
                "lease-alpha",
                "node-cortex",
                "cortex",
                100,
                5_000, // 5000ms TTL
                "kernel-root",
                &auth_secret,
                now,
            )
            .expect("issue lease");

        // Acquire lease
        manager
            .acquire_or_renew_lease(lease, now)
            .expect("acquire lease");
        assert_eq!(manager.fencing_token_watermark(), 100);

        // Active at now + 100ms
        assert!(manager.has_active_authority("node-cortex", now + 100));

        // Active at now + 4999ms
        assert!(manager.has_active_authority("node-cortex", now + 4999));

        // Expired at now + 5000ms -> authority drops instantly
        assert!(!manager.has_active_authority("node-cortex", now + 5000));

        // Another node cannot claim authority
        assert!(!manager.has_active_authority("node-other", now + 100));
    }

    #[test]
    fn non_monotonic_fencing_token_is_rejected() {
        let provider = Ed25519Provider;
        let (auth_secret, auth_public) = provider.generate_keypair().expect("keys");

        let mut manager = LeaseManager::new(Arc::new(provider), Some(auth_public));

        let now = 1_000_000;
        let lease1 = manager
            .issue_lease(
                "lease-1",
                "node-a",
                "primary",
                50,
                10_000,
                "kernel-root",
                &auth_secret,
                now,
            )
            .expect("issue 1");
        manager
            .acquire_or_renew_lease(lease1, now)
            .expect("acquire 1");

        // Attempting to acquire lease with token 40 (lower than 50)
        let lease_lower = manager
            .issue_lease(
                "lease-2",
                "node-b",
                "primary",
                40,
                10_000,
                "kernel-root",
                &auth_secret,
                now + 100,
            )
            .expect("issue lower");

        let result = manager.acquire_or_renew_lease(lease_lower, now + 100);
        assert!(result.is_err());
        assert_eq!(manager.fencing_token_watermark(), 50);
    }

    #[test]
    fn fenced_writes_are_guarded() {
        let provider = Ed25519Provider;
        let (auth_secret, auth_public) = provider.generate_keypair().expect("keys");

        let mut manager = LeaseManager::new(Arc::new(provider), Some(auth_public));

        let now = 1_000_000;
        let lease = manager
            .issue_lease(
                "lease-1",
                "node-a",
                "primary",
                10,
                5_000,
                "kernel-root",
                &auth_secret,
                now,
            )
            .expect("issue");
        manager.acquire_or_renew_lease(lease, now).expect("acquire");

        // Permitted write with token 10
        assert!(manager.assert_fenced_write("node-a", 10, now + 100).is_ok());

        // Permitted write with higher token 11
        assert!(manager.assert_fenced_write("node-a", 11, now + 100).is_ok());

        // Stale token 9 rejected
        assert!(manager.assert_fenced_write("node-a", 9, now + 100).is_err());

        // Write during expired lease rejected
        assert!(
            manager
                .assert_fenced_write("node-a", 10, now + 6000)
                .is_err()
        );
    }

    #[test]
    fn transition_history_is_recorded_and_sequenced() {
        let provider = Ed25519Provider;
        let (auth_secret, auth_public) = provider.generate_keypair().expect("keys");

        let mut manager = LeaseManager::new(Arc::new(provider), Some(auth_public));

        let now = 1_000_000;
        let lease1 = manager
            .issue_lease("l1", "node-a", "primary", 1, 5000, "k", &auth_secret, now)
            .expect("l1");
        manager.acquire_or_renew_lease(lease1, now).expect("acq 1");

        let lease2 = manager
            .issue_lease(
                "l2",
                "node-b",
                "primary",
                2,
                5000,
                "k",
                &auth_secret,
                now + 1000,
            )
            .expect("l2");
        manager
            .acquire_or_renew_lease(lease2, now + 1000)
            .expect("acq 2");

        let history = manager.transition_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].to_node_id, "node-a");
        assert_eq!(history[0].fencing_token, 1);
        assert_eq!(history[1].from_node_id, Some("node-a".to_string()));
        assert_eq!(history[1].to_node_id, "node-b");
        assert_eq!(history[1].fencing_token, 2);
    }
}
