//! Node identity and enrollment workflow for the CYNIC Trust-Chain vertical.
//!
//! Provides the end-to-end cryptographic root-of-trust pipeline:
//! 1. Generate / load local Ed25519 node signing key.
//! 2. Generate self-signed Certificate Signing Request (CSR) proving possession.
//! 3. Verify CSR authenticity.
//! 4. Issue Kernel-signed Node Enrollment Certificate.
//! 5. Verify and install certificate to `/etc/cynic/node-identity.cert`.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use cynic_node_protocol::{NODE_ENROLLMENT_CERTIFICATE_SCHEMA, NodeCsr, NodeEnrollmentCertificate};
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
}
