//! Node identity and enrollment workflow for the CYNIC Trust-Chain vertical.
//!
//! Provides the end-to-end cryptographic root-of-trust pipeline:
//! 1. Generate / load local Ed25519 node signing key.
//! 2. Generate self-signed Certificate Signing Request (CSR) proving possession.
//! 3. Verify CSR authenticity.
//! 4. Issue Kernel-signed Node Enrollment Certificate.
//! 5. Verify and install certificate to the system config directory's
//! `node-identity.cert`.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use cynic_node_protocol::{
    MachineEnrollmentAuthority, NODE_ENROLLMENT_CERTIFICATE_SCHEMA, NodeCsr,
    NodeEnrollmentCertificate, NodeEnrollmentRequest, SignedNodeEnrollmentGrant,
    admit_node_enrollment, build_node_enrollment_grant, node_enrollment_grant_signing_bytes,
    validate_signed_node_enrollment_grant,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::crypto::{CryptoError, CryptoProvider, Ed25519Provider, SecretStore};

/// Cryptographic identity material for a node.
#[derive(Debug, Clone)]
pub struct NodeIdentityMaterial {
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub public_key_hex: String,
}

/// Orchestrator for the cryptographic identity enrollment workflow.
pub struct NodeIdentityWorkflow {
    crypto: Arc<dyn CryptoProvider>,
}

impl Default for NodeIdentityWorkflow {
    fn default() -> Self {
        Self {
            crypto: Arc::new(Ed25519Provider),
        }
    }
}

impl NodeIdentityWorkflow {
    pub fn new(crypto: Arc<dyn CryptoProvider>) -> Self {
        Self { crypto }
    }

    /// Generates a new Ed25519 keypair and persists the private key with strict 0600 permissions.
    pub fn generate_keypair(&self, key_path: &Path) -> Result<NodeIdentityMaterial, CryptoError> {
        let (private_key, public_key) = self.crypto.generate_keypair()?;
        SecretStore::save_secret(key_path, &private_key)?;
        let public_key_hex = hex::encode(&public_key);
        Ok(NodeIdentityMaterial {
            private_key,
            public_key,
            public_key_hex,
        })
    }

    /// Loads an existing private key from disk or generates a new keypair if absent.
    pub fn load_or_generate_keypair(
        &self,
        key_path: &Path,
    ) -> Result<NodeIdentityMaterial, CryptoError> {
        if key_path.exists() {
            let private_key = SecretStore::load_secret(key_path)?;
            if private_key.len() != 32 {
                return Err(CryptoError::InvalidKey(
                    "stored private key is not 32 bytes".to_string(),
                ));
            }
            let bytes: [u8; 32] = private_key
                .clone()
                .try_into()
                .map_err(|_| CryptoError::InvalidKey("invalid key length".to_string()))?;
            let signing_key = SigningKey::from_bytes(&bytes);
            let verifying_key = VerifyingKey::from(&signing_key);
            let public_key = verifying_key.to_bytes().to_vec();
            let public_key_hex = hex::encode(&public_key);
            Ok(NodeIdentityMaterial {
                private_key,
                public_key,
                public_key_hex,
            })
        } else {
            self.generate_keypair(key_path)
        }
    }

    /// Creates a self-signed CSR to prove possession of the private key.
    pub fn create_csr(
        &self,
        node_id: &str,
        requested_role: &str,
        material: &NodeIdentityMaterial,
        now_unix_s: u64,
    ) -> Result<NodeCsr, CryptoError> {
        let dummy_csr = NodeCsr::new(
            node_id.to_string(),
            requested_role.to_string(),
            material.public_key_hex.clone(),
            now_unix_s,
            String::new(),
        );
        let signing_bytes = dummy_csr.signing_bytes();
        let signature = self.crypto.sign(&signing_bytes, &material.private_key)?;
        let csr_signature_hex = hex::encode(signature);

        let csr = NodeCsr::new(
            node_id.to_string(),
            requested_role.to_string(),
            material.public_key_hex.clone(),
            now_unix_s,
            csr_signature_hex,
        );

        let blockers = csr.validate();
        if !blockers.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "created CSR is structurally invalid: {blockers:?}"
            )));
        }

        Ok(csr)
    }

    /// Verifies the signature on a node CSR using its enclosed public key.
    pub fn verify_csr(&self, csr: &NodeCsr) -> Result<bool, CryptoError> {
        let blockers = csr.validate();
        if !blockers.is_empty() {
            return Ok(false);
        }
        let public_key = hex::decode(&csr.public_key_hex)
            .map_err(|e| CryptoError::InvalidKey(format!("invalid hex public key: {e}")))?;
        let signature = hex::decode(&csr.csr_signature_hex)
            .map_err(|e| CryptoError::InvalidKey(format!("invalid hex signature: {e}")))?;

        let signing_bytes = csr.signing_bytes();
        self.crypto.verify(&signing_bytes, &signature, &public_key)
    }

    /// Issues a Kernel-signed enrollment certificate for an admitted node CSR.
    pub fn issue_certificate(
        &self,
        csr: &NodeCsr,
        kernel_authority_id: &str,
        kernel_private_key: &[u8],
        ttl_s: u64,
        now_unix_s: u64,
    ) -> Result<NodeEnrollmentCertificate, CryptoError> {
        if !self.verify_csr(csr)? {
            return Err(CryptoError::OperationFailed(
                "cannot issue certificate for invalid or unverified CSR".to_string(),
            ));
        }

        let expires_at_unix_s = now_unix_s.saturating_add(ttl_s);
        let mut dummy_cert = NodeEnrollmentCertificate {
            schema: NODE_ENROLLMENT_CERTIFICATE_SCHEMA.to_string(),
            certificate_id: String::new(),
            node_id: csr.node_id.clone(),
            role: csr.requested_role.clone(),
            subject_public_key_hex: csr.public_key_hex.clone(),
            attested_capabilities: csr.hardware_capabilities.clone(),
            kernel_authority_id: kernel_authority_id.to_string(),
            issued_at_unix_s: now_unix_s,
            expires_at_unix_s,
            kernel_signature_hex: String::new(),
        };

        let signing_bytes = dummy_cert.signing_bytes();
        let cert_id_digest = Sha256::digest(&signing_bytes);
        dummy_cert.certificate_id = format!("sha256:{}", hex::encode(cert_id_digest));

        let signature = self.crypto.sign(&signing_bytes, kernel_private_key)?;
        dummy_cert.kernel_signature_hex = hex::encode(signature);

        let blockers = dummy_cert.validate(now_unix_s);
        if !blockers.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "issued certificate is structurally invalid: {blockers:?}"
            )));
        }

        Ok(dummy_cert)
    }

    /// Verifies the Kernel signature and validity window of a NodeEnrollmentCertificate.
    pub fn verify_certificate(
        &self,
        cert: &NodeEnrollmentCertificate,
        kernel_public_key: &[u8],
        now_unix_s: u64,
    ) -> Result<bool, CryptoError> {
        let blockers = cert.validate(now_unix_s);
        if !blockers.is_empty() {
            return Ok(false);
        }

        let signature = hex::decode(&cert.kernel_signature_hex)
            .map_err(|e| CryptoError::InvalidKey(format!("invalid hex signature: {e}")))?;

        let signing_bytes = cert.signing_bytes();
        self.crypto
            .verify(&signing_bytes, &signature, kernel_public_key)
    }

    /// Installs a verified enrollment certificate to disk as JSON.
    pub fn install_certificate(
        &self,
        cert: &NodeEnrollmentCertificate,
        cert_path: &Path,
    ) -> Result<(), CryptoError> {
        if let Some(parent) = cert_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json_bytes = serde_json::to_vec_pretty(cert)?;
        fs::write(cert_path, json_bytes)?;
        Ok(())
    }

    /// Loads an enrollment certificate from disk.
    pub fn load_certificate(
        &self,
        cert_path: &Path,
    ) -> Result<NodeEnrollmentCertificate, CryptoError> {
        let bytes = fs::read(cert_path)?;
        let cert: NodeEnrollmentCertificate = serde_json::from_slice(&bytes)?;
        Ok(cert)
    }

    /// Issues a signed node enrollment grant for an admitted request.
    ///
    /// Runs the pure admission decision first; a blocked admission returns
    /// its blockers without touching any key. The authority signing key is
    /// loaded read-only from already-decoded bytes and is never generated
    /// here. Pass an empty `presented_token_digest` with
    /// `require_token = false` only for operator-mediated issuance, where
    /// the authority holder vouches out of band (reported, never silent).
    pub fn issue_signed_enrollment_grant(
        &self,
        request: &NodeEnrollmentRequest,
        authority: &MachineEnrollmentAuthority,
        presented_token_digest: &str,
        require_token: bool,
        authority_signing_key: &[u8],
        now_unix_s: u64,
    ) -> Result<SignedNodeEnrollmentGrant, CryptoError> {
        let admission = admit_node_enrollment(
            request,
            authority,
            presented_token_digest,
            now_unix_s,
            require_token,
        );
        if !admission.blockers.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "enrollment admission blocked: {}",
                admission.blockers.join(", ")
            )));
        }
        let grant =
            build_node_enrollment_grant(request, &admission, &authority.authority_id, now_unix_s);
        let key_bytes: [u8; 32] = authority_signing_key.try_into().map_err(|_| {
            CryptoError::InvalidKey("authority signing key is not 32 bytes".to_string())
        })?;
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let authority_key_id = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()))
        );
        let mut signed = SignedNodeEnrollmentGrant {
            schema: cynic_node_protocol::SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
            authority_key_id,
            signature_namespace: cynic_node_protocol::NODE_ENROLLMENT_SIGNATURE_NAMESPACE
                .to_string(),
            grant,
            signature_hex: String::new(),
        };
        let signing_bytes = node_enrollment_grant_signing_bytes(&signed).map_err(|error| {
            CryptoError::OperationFailed(format!("grant signing bytes failed: {error}"))
        })?;
        let signature = self.crypto.sign(&signing_bytes, authority_signing_key)?;
        signed.signature_hex = hex::encode(signature);
        let violations = validate_signed_node_enrollment_grant(&signed);
        if !violations.is_empty() {
            return Err(CryptoError::OperationFailed(format!(
                "issued grant is structurally invalid: {}",
                violations.join(", ")
            )));
        }
        Ok(signed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn full_identity_enrollment_cycle_succeeds() {
        let dir = tempdir().expect("tempdir");
        let node_key_path = dir.path().join("node.key");
        let cert_path = dir.path().join("node.cert");

        let workflow = NodeIdentityWorkflow::default();
        let provider = Ed25519Provider;

        // 1. Generate node keypair
        let node_material = workflow
            .generate_keypair(&node_key_path)
            .expect("generate node key");
        assert!(node_key_path.exists());

        // 2. Generate Kernel authority keypair
        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("kernel keypair");

        // 3. Node creates CSR
        let now = 1700000000;
        let csr = workflow
            .create_csr("node-cortex", "cortex", &node_material, now)
            .expect("create csr");
        assert!(workflow.verify_csr(&csr).expect("verify csr"));

        // 4. Kernel issues certificate
        let cert = workflow
            .issue_certificate(&csr, "kernel-authority-01", &kernel_secret, 86400, now)
            .expect("issue cert");

        // 5. Verify certificate
        assert!(
            workflow
                .verify_certificate(&cert, &kernel_public, now + 100)
                .expect("verify cert")
        );

        // 6. Install and reload
        workflow
            .install_certificate(&cert, &cert_path)
            .expect("install cert");
        assert!(cert_path.exists());

        let reloaded = workflow.load_certificate(&cert_path).expect("load cert");
        assert_eq!(reloaded, cert);
    }

    #[test]
    fn tampered_csr_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let node_key_path = dir.path().join("node.key");

        let workflow = NodeIdentityWorkflow::default();
        let node_material = workflow
            .generate_keypair(&node_key_path)
            .expect("generate node key");

        let mut csr = workflow
            .create_csr("node-alpha", "builder", &node_material, 1700000000)
            .expect("csr");

        // Tamper with node_id
        csr.node_id = "node-malicious".to_string();
        assert!(!workflow.verify_csr(&csr).expect("verify csr"));
    }

    #[test]
    fn expired_or_tampered_certificate_is_rejected() {
        let workflow = NodeIdentityWorkflow::default();
        let provider = Ed25519Provider;

        let (node_secret, node_public) = provider.generate_keypair().expect("node keys");
        let node_material = NodeIdentityMaterial {
            private_key: node_secret,
            public_key: node_public.clone(),
            public_key_hex: hex::encode(&node_public),
        };

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("kernel keys");

        let now = 1700000000;
        let csr = workflow
            .create_csr("node-beta", "builder", &node_material, now)
            .expect("csr");

        let cert = workflow
            .issue_certificate(
                &csr,
                "kernel-auth",
                &kernel_secret,
                3600, // 1 hour TTL
                now,
            )
            .expect("issue cert");

        // Valid at now + 10s
        assert!(
            workflow
                .verify_certificate(&cert, &kernel_public, now + 10)
                .expect("verify")
        );

        // Expired at now + 3601s
        assert!(
            !workflow
                .verify_certificate(&cert, &kernel_public, now + 3601)
                .expect("verify")
        );

        // Wrong kernel public key
        let (_, wrong_kernel_public) = provider.generate_keypair().expect("other keys");
        assert!(
            !workflow
                .verify_certificate(&cert, &wrong_kernel_public, now + 10)
                .expect("verify")
        );
    }

    #[test]
    fn certificate_attests_hardware_capabilities_from_csr() {
        let workflow = NodeIdentityWorkflow::default();
        let provider = Ed25519Provider;

        let (node_secret, node_public) = provider.generate_keypair().expect("node keys");
        let node_material = NodeIdentityMaterial {
            private_key: node_secret,
            public_key: node_public.clone(),
            public_key_hex: hex::encode(&node_public),
        };

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("kernel keys");
        let now = 1700000000;

        let mut csr = workflow
            .create_csr("node-gpu-1", "inference_worker", &node_material, now)
            .expect("csr");
        csr.hardware_capabilities = vec!["vram_24gb".to_string(), "avx512".to_string()];
        // Resign CSR with capabilities
        let signing_bytes = csr.signing_bytes();
        let sig = provider
            .sign(&signing_bytes, &node_material.private_key)
            .expect("sign");
        csr.csr_signature_hex = hex::encode(sig);

        let cert = workflow
            .issue_certificate(&csr, "kernel-auth", &kernel_secret, 3600, now)
            .expect("issue cert");

        assert_eq!(
            cert.attested_capabilities,
            vec!["vram_24gb".to_string(), "avx512".to_string()]
        );
        assert!(
            workflow
                .verify_certificate(&cert, &kernel_public, now + 10)
                .expect("verify")
        );
    }

    fn enrollment_request_for(public_key: &[u8], now: u64) -> NodeEnrollmentRequest {
        use sha2::{Digest, Sha256};
        let public_key_hex = hex::encode(public_key);
        let digest = format!("sha256:{}", hex::encode(Sha256::digest(public_key)));
        NodeEnrollmentRequest {
            schema: "cynic.node_enrollment_request.v1".to_string(),
            request_id: "enroll:test-request".to_string(),
            seed_id: "seed:test".to_string(),
            node_id: "node-test-1".to_string(),
            role: "build_runner".to_string(),
            machine_identity_digest: digest,
            machine_public_key_hex: public_key_hex,
            nonce: "nonce-test".to_string(),
            issued_at_unix_s: now,
            expires_at_unix_s: now + 3_600,
            requested_capabilities: vec!["node_control".to_string(), "build_runner".to_string()],
        }
    }

    fn enrollment_authority_for(seed: &str, now: u64) -> MachineEnrollmentAuthority {
        MachineEnrollmentAuthority {
            authority_id: "authority:test".to_string(),
            seed_id: seed.to_string(),
            token_digest: String::new(),
            expires_at_unix_s: now + 3_600,
            renewal_authority_expires_at_unix_s: now + 86_400,
            max_grant_ttl_s: 1_800,
            allowed_roles: vec!["build_runner".to_string()],
            allowed_capabilities: vec!["node_control".to_string(), "build_runner".to_string()],
        }
    }

    #[test]
    fn issued_grant_verifies_against_authority_key() {
        let workflow = NodeIdentityWorkflow::default();
        let provider = Ed25519Provider;
        let (authority_secret, authority_public) = provider.generate_keypair().expect("keys");
        let (node_secret, node_public) = provider.generate_keypair().expect("node keys");
        let _ = node_secret;
        let now = 1_700_000_000;
        let request = enrollment_request_for(&node_public, now);
        let authority = enrollment_authority_for(&request.seed_id, now);

        let signed = workflow
            .issue_signed_enrollment_grant(&request, &authority, "", false, &authority_secret, now)
            .expect("issue grant");

        assert_eq!(signed.grant.node_id, "node-test-1");
        assert_eq!(signed.grant.role, "build_runner");
        assert_eq!(signed.grant.expires_at_unix_s, now + 1_800);
        let signing_bytes =
            node_enrollment_grant_signing_bytes(&signed).expect("signing bytes");
        let signature = hex::decode(&signed.signature_hex).expect("signature hex");
        assert!(
            provider
                .verify(&signing_bytes, &signature, &authority_public)
                .expect("verify")
        );
    }

    #[test]
    fn blocked_admission_returns_blockers_without_signing() {
        let workflow = NodeIdentityWorkflow::default();
        let provider = Ed25519Provider;
        let (authority_secret, _) = provider.generate_keypair().expect("keys");
        let (_, node_public) = provider.generate_keypair().expect("node keys");
        let now = 1_700_000_000;
        let mut request = enrollment_request_for(&node_public, now);
        request.role = "forbidden_role".to_string();
        let authority = enrollment_authority_for(&request.seed_id, now);

        let error = workflow
            .issue_signed_enrollment_grant(&request, &authority, "", false, &authority_secret, now)
            .expect_err("must block");
        let message = format!("{error:?}");
        assert!(
            message.contains("node_enrollment_role_not_allowed"),
            "{message}"
        );
    }
}
