//! Peer validator for the authenticated gossip mesh in the CYNIC Trust-Chain vertical.
//!
//! Validates incoming fleet heartbeats against cryptographically enrolled peer certificates,
//! guaranteeing that unauthenticated or tampered health signals are rejected before
//! affecting the fleet EmergenceState.

use std::collections::HashMap;
use std::sync::Arc;

use cynic_protocol::fleet_heartbeat::{
    FleetHeartbeat, HeartbeatValidationResult, SignedFleetHeartbeat,
};
use cynic_protocol::node_identity::NodeEnrollmentCertificate;

use crate::crypto::{CryptoError, CryptoProvider, Ed25519Provider};

/// Validator for peer heartbeats backed by Kernel-signed enrollment certificates.
/// Supports multiple federated root authorities to eliminate any Single Point of Failure (SPOF).
pub struct PeerValidator {
    crypto: Arc<dyn CryptoProvider>,
    trusted_root_authorities: HashMap<String, Vec<u8>>,
    peer_certificates: HashMap<String, NodeEnrollmentCertificate>,
}

impl Default for PeerValidator {
    fn default() -> Self {
        Self {
            crypto: Arc::new(Ed25519Provider),
            trusted_root_authorities: HashMap::new(),
            peer_certificates: HashMap::new(),
        }
    }
}

impl PeerValidator {
    pub fn new(crypto: Arc<dyn CryptoProvider>) -> Self {
        Self {
            crypto,
            trusted_root_authorities: HashMap::new(),
            peer_certificates: HashMap::new(),
        }
    }

    /// Registers a trusted root/kernel authority public key.
    pub fn register_root_authority(
        &mut self,
        authority_id: &str,
        public_key: &[u8],
    ) -> Result<(), CryptoError> {
        if authority_id.trim().is_empty() {
            return Err(CryptoError::InvalidKey(
                "authority_id cannot be empty".into(),
            ));
        }
        if public_key.len() != 32 {
            return Err(CryptoError::InvalidKey(
                "root public key must be 32 bytes".into(),
            ));
        }
        self.trusted_root_authorities
            .insert(authority_id.to_string(), public_key.to_vec());
        Ok(())
    }

    /// Registers a peer certificate using the configured federated root authorities (Zero-SPOF).
    pub fn register_peer_certificate_with_roots(
        &mut self,
        cert: NodeEnrollmentCertificate,
        now_unix_s: u64,
    ) -> Result<(), CryptoError> {
        let root_pubkey = self
            .trusted_root_authorities
            .get(&cert.kernel_authority_id)
            .ok_or_else(|| {
                CryptoError::OperationFailed(format!(
                    "untrusted root authority '{}' for peer certificate",
                    cert.kernel_authority_id
                ))
            })?
            .clone();
        self.register_peer_certificate(cert, &root_pubkey, now_unix_s)
    }
    /// Registers a Kernel-signed enrollment certificate for a peer node.
    pub fn register_peer_certificate(
        &mut self,
        cert: NodeEnrollmentCertificate,
        kernel_public_key: &[u8],
        now_unix_s: u64,
    ) -> Result<(), CryptoError> {
        let blockers = cert.validate(now_unix_s);
        if !blockers.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "peer certificate validation failed: {blockers:?}"
            )));
        }

        let signature = hex::decode(&cert.kernel_signature_hex)
            .map_err(|e| CryptoError::InvalidKey(format!("invalid hex signature: {e}")))?;

        let signing_bytes = cert.signing_bytes();
        let valid = self
            .crypto
            .verify(&signing_bytes, &signature, kernel_public_key)?;
        if !valid {
            return Err(CryptoError::OperationFailed(
                "peer certificate Kernel signature verification failed".to_string(),
            ));
        }

        self.peer_certificates.insert(cert.node_id.clone(), cert);
        Ok(())
    }

    /// Signs an outgoing heartbeat using the local node's private key.
    pub fn sign_heartbeat(
        &self,
        heartbeat: &FleetHeartbeat,
        private_key: &[u8],
        certificate_id: &str,
    ) -> Result<SignedFleetHeartbeat, CryptoError> {
        let dummy = SignedFleetHeartbeat::new(
            heartbeat.clone(),
            heartbeat.node_id.clone(),
            certificate_id.to_string(),
            String::new(),
        );

        let signing_bytes = dummy.signing_bytes();
        let signature = self.crypto.sign(&signing_bytes, private_key)?;
        let signature_hex = hex::encode(signature);

        let signed = SignedFleetHeartbeat::new(
            heartbeat.clone(),
            heartbeat.node_id.clone(),
            certificate_id.to_string(),
            signature_hex,
        );

        Ok(signed)
    }

    /// Validates an incoming signed heartbeat against registered peer certificates.
    pub fn validate_heartbeat(
        &self,
        signed: &SignedFleetHeartbeat,
        now_unix_s: u64,
        max_clock_skew_s: u64,
    ) -> HeartbeatValidationResult {
        let blockers = signed.validate();
        if !blockers.is_empty() {
            return HeartbeatValidationResult::MalformedPayload(format!("{blockers:?}"));
        }

        if signed.signer_node_id != signed.heartbeat.node_id {
            return HeartbeatValidationResult::NodeIdMismatch;
        }

        // Check clock skew
        let time_diff = if now_unix_s >= signed.heartbeat.emitted_at_unix_s {
            now_unix_s - signed.heartbeat.emitted_at_unix_s
        } else {
            signed.heartbeat.emitted_at_unix_s - now_unix_s
        };

        if time_diff > max_clock_skew_s {
            return HeartbeatValidationResult::ClockSkewExceeded;
        }

        // Lookup peer certificate
        let cert = match self.peer_certificates.get(&signed.signer_node_id) {
            Some(c) => c,
            None => return HeartbeatValidationResult::UntrustedSigner,
        };

        // Check certificate expiration
        if !cert.is_valid_at(now_unix_s) {
            return HeartbeatValidationResult::ExpiredCertificate;
        }

        // Verify Ed25519 signature
        let public_key = match hex::decode(&cert.subject_public_key_hex) {
            Ok(k) => k,
            Err(_) => {
                return HeartbeatValidationResult::MalformedPayload("invalid public key hex".into());
            }
        };

        let signature = match hex::decode(&signed.signature_hex) {
            Ok(s) => s,
            Err(_) => {
                return HeartbeatValidationResult::MalformedPayload("invalid signature hex".into());
            }
        };

        let signing_bytes = signed.signing_bytes();
        match self.crypto.verify(&signing_bytes, &signature, &public_key) {
            Ok(true) => HeartbeatValidationResult::Valid,
            Ok(false) => HeartbeatValidationResult::InvalidSignature,
            Err(e) => HeartbeatValidationResult::MalformedPayload(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::fleet_discovery::FleetNodeStatus;
    use cynic_protocol::node_identity::NODE_ENROLLMENT_CERTIFICATE_SCHEMA;
    use sha2::{Digest, Sha256};

    fn make_cert(
        node_id: &str,
        public_key: &[u8],
        kernel_secret: &[u8],
        now_unix_s: u64,
        ttl_s: u64,
        provider: &dyn CryptoProvider,
    ) -> NodeEnrollmentCertificate {
        make_cert_for_authority(
            node_id,
            "kernel-alpha",
            public_key,
            kernel_secret,
            now_unix_s,
            ttl_s,
            provider,
        )
    }

    /// Issues a certificate under a named root authority.
    ///
    /// The authority id must be a parameter, not a constant patched afterwards:
    /// it is covered by `signing_bytes()` and by the `certificate_id` digest, so
    /// mutating it post-signature leaves the id computed over the previous bytes.
    /// Multi-root tests need a real issuer name, since
    /// `register_peer_certificate_with_roots` resolves the trusted key by this id.
    fn make_cert_for_authority(
        node_id: &str,
        kernel_authority_id: &str,
        public_key: &[u8],
        kernel_secret: &[u8],
        now_unix_s: u64,
        ttl_s: u64,
        provider: &dyn CryptoProvider,
    ) -> NodeEnrollmentCertificate {
        let mut cert = NodeEnrollmentCertificate {
            schema: NODE_ENROLLMENT_CERTIFICATE_SCHEMA.to_string(),
            certificate_id: String::new(),
            node_id: node_id.to_string(),
            role: "worker".to_string(),
            subject_public_key_hex: hex::encode(public_key),
            kernel_authority_id: kernel_authority_id.to_string(),
            issued_at_unix_s: now_unix_s,
            expires_at_unix_s: now_unix_s + ttl_s,
            kernel_signature_hex: String::new(),
        };
        let signing_bytes = cert.signing_bytes();
        let digest = Sha256::digest(&signing_bytes);
        cert.certificate_id = format!("sha256:{}", hex::encode(digest));
        let sig = provider.sign(&signing_bytes, kernel_secret).expect("sign");
        cert.kernel_signature_hex = hex::encode(sig);
        cert
    }

    #[test]
    fn enrolled_peer_heartbeat_is_validated() {
        let provider = Ed25519Provider;
        let mut validator = PeerValidator::default();

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("kernel keys");
        let (peer_secret, peer_public) = provider.generate_keypair().expect("peer keys");

        let now = 1700000000;
        let cert = make_cert(
            "node-worker-1",
            &peer_public,
            &kernel_secret,
            now,
            86400,
            &provider,
        );
        validator
            .register_peer_certificate(cert.clone(), &kernel_public, now)
            .expect("register cert");

        let hb = FleetHeartbeat {
            node_id: "node-worker-1".to_string(),
            emitted_at_unix_s: now + 5,
            self_status: FleetNodeStatus::Alive,
            uptime_s: 3600,
            unknowns: vec![],
        };

        let signed = validator
            .sign_heartbeat(&hb, &peer_secret, &cert.certificate_id)
            .expect("sign hb");

        let result = validator.validate_heartbeat(&signed, now + 10, 30);
        assert_eq!(result, HeartbeatValidationResult::Valid);
    }

    #[test]
    fn tampered_heartbeat_signature_is_rejected() {
        let provider = Ed25519Provider;
        let mut validator = PeerValidator::default();

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("kernel keys");
        let (peer_secret, peer_public) = provider.generate_keypair().expect("peer keys");

        let now = 1700000000;
        let cert = make_cert(
            "node-worker-1",
            &peer_public,
            &kernel_secret,
            now,
            86400,
            &provider,
        );
        validator
            .register_peer_certificate(cert.clone(), &kernel_public, now)
            .expect("register cert");

        let hb = FleetHeartbeat {
            node_id: "node-worker-1".to_string(),
            emitted_at_unix_s: now + 5,
            self_status: FleetNodeStatus::Alive,
            uptime_s: 3600,
            unknowns: vec![],
        };

        let mut signed = validator
            .sign_heartbeat(&hb, &peer_secret, &cert.certificate_id)
            .expect("sign hb");

        // Tamper with heartbeat status after signing
        signed.heartbeat.self_status = FleetNodeStatus::Degraded;

        let result = validator.validate_heartbeat(&signed, now + 10, 30);
        assert_eq!(result, HeartbeatValidationResult::InvalidSignature);
    }

    #[test]
    fn unregistered_peer_is_untrusted() {
        let provider = Ed25519Provider;
        let validator = PeerValidator::default();
        let (peer_secret, _) = provider.generate_keypair().expect("peer keys");

        let now = 1700000000;
        let hb = FleetHeartbeat {
            node_id: "node-unknown".to_string(),
            emitted_at_unix_s: now,
            self_status: FleetNodeStatus::Alive,
            uptime_s: 100,
            unknowns: vec![],
        };

        let signed = validator
            .sign_heartbeat(
                &hb,
                &peer_secret,
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .expect("sign hb");

        let result = validator.validate_heartbeat(&signed, now, 30);
        assert_eq!(result, HeartbeatValidationResult::UntrustedSigner);
    }

    #[test]
    fn clock_skew_beyond_threshold_is_rejected() {
        let provider = Ed25519Provider;
        let mut validator = PeerValidator::default();

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("kernel keys");
        let (peer_secret, peer_public) = provider.generate_keypair().expect("peer keys");

        let now = 1700000000;
        let cert = make_cert(
            "node-worker-1",
            &peer_public,
            &kernel_secret,
            now,
            86400,
            &provider,
        );
        validator
            .register_peer_certificate(cert.clone(), &kernel_public, now)
            .expect("register cert");

        let hb = FleetHeartbeat {
            node_id: "node-worker-1".to_string(),
            emitted_at_unix_s: now - 100, // 100s ago
            self_status: FleetNodeStatus::Alive,
            uptime_s: 3600,
            unknowns: vec![],
        };

        let signed = validator
            .sign_heartbeat(&hb, &peer_secret, &cert.certificate_id)
            .expect("sign hb");

        let result = validator.validate_heartbeat(&signed, now, 30); // max skew 30s
        assert_eq!(result, HeartbeatValidationResult::ClockSkewExceeded);
    }

    #[test]
    fn multi_root_federation_zero_spof_validation() {
        let provider = Ed25519Provider;
        let mut validator = PeerValidator::default();

        let (kernel_a_secret, kernel_a_pub) = provider.generate_keypair().expect("kernel a");
        let (kernel_b_secret, kernel_b_pub) =
            provider.generate_keypair().expect("kernel b (standby)");

        validator
            .register_root_authority("kernel-primary", &kernel_a_pub)
            .expect("reg a");
        validator
            .register_root_authority("kernel-standby", &kernel_b_pub)
            .expect("reg b");

        let (peer_1_secret, peer_1_pub) = provider.generate_keypair().expect("p1");
        let (peer_2_secret, peer_2_pub) = provider.generate_keypair().expect("p2");

        let now = 1700000000;
        // Peer 1 enrolled by the primary root, peer 2 by the standby root. Both
        // issuers are named at construction: certificate_id is a digest over
        // signing_bytes(), which covers kernel_authority_id, so patching the
        // authority after signing left cert2 with an id computed over the
        // pre-mutation bytes — a stale identity the registry would still accept.
        let cert1 = make_cert_for_authority(
            "node-1",
            "kernel-primary",
            &peer_1_pub,
            &kernel_a_secret,
            now,
            86400,
            &provider,
        );
        let cert2 = make_cert_for_authority(
            "node-2",
            "kernel-standby",
            &peer_2_pub,
            &kernel_b_secret,
            now,
            86400,
            &provider,
        );

        // Both enroll cleanly through multi-root registry
        validator
            .register_peer_certificate_with_roots(cert1.clone(), now)
            .expect("reg c1");
        validator
            .register_peer_certificate_with_roots(cert2.clone(), now)
            .expect("reg c2");

        // Both heartbeats are valid and verifiable without communicating with any central server
        let hb1 = FleetHeartbeat {
            node_id: "node-1".to_string(),
            emitted_at_unix_s: now,
            self_status: FleetNodeStatus::Alive,
            uptime_s: 10,
            unknowns: vec![],
        };
        let signed1 = validator
            .sign_heartbeat(&hb1, &peer_1_secret, &cert1.certificate_id)
            .expect("sign 1");
        assert_eq!(
            validator.validate_heartbeat(&signed1, now, 30),
            HeartbeatValidationResult::Valid
        );

        let hb2 = FleetHeartbeat {
            node_id: "node-2".to_string(),
            emitted_at_unix_s: now,
            self_status: FleetNodeStatus::Alive,
            uptime_s: 10,
            unknowns: vec![],
        };
        let signed2 = validator
            .sign_heartbeat(&hb2, &peer_2_secret, &cert2.certificate_id)
            .expect("sign 2");
        assert_eq!(
            validator.validate_heartbeat(&signed2, now, 30),
            HeartbeatValidationResult::Valid
        );
    }
}
