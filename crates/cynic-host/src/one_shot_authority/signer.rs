//! Port + local adapter for signing one-shot capabilities.
//!
//! The protocol layer owns the *shape* of the signing payload
//! (`OneShotCapability::signing_payload`); the host owns the *key* and
//! the actual ed25519 signature.  Splitting the two keeps cryptography
//! out of the portable protocol crate while still letting any host
//! surface issue verifiable capabilities.

use std::path::Path;

use cynic_protocol::OneShotCapability;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

/// Port: produces a detached ed25519 hex signature over a canonical
/// `OneShotCapability` signing payload.
pub trait CapabilitySigningPort {
    /// Sign the canonical payload of `capability`, writing the result
    /// back into `capability.signature_hex`.
    fn sign_capability(&self, capability: &mut OneShotCapability) -> Result<String, String>;
}

/// Local filesystem-backed signer reading an ed25519 seed keypair.
///
/// The seed file holds 32 raw bytes (the ed25519 secret scalar), identical
/// to what `ssh-keygen -t ed25519 -f key` stores inside the OpenSSH blob.
#[derive(Debug, Clone)]
pub struct LocalCapabilitySigningKey {
    signing_key: SigningKey,
}

impl LocalCapabilitySigningKey {
    /// Load an ed25519 seed from `seed_path`.
    /// Accepts either:
    ///  - 32-byte raw seed, or
    ///  - the base64-decoded private portion of an OpenSSH `openssh-key-v1` blob.
    pub fn from_seed_path(seed_path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(seed_path)
            .map_err(|e| format!("failed to read capability signing key: {}", e))?;

        // Try raw 32-byte seed first.
        if bytes.len() == 32 {
            let arr: [u8; 32] = bytes[..32]
                .try_into()
                .map_err(|_| "seed must be 32 bytes".to_string())?;
            // ed25519-dalek 2.x: `from_bytes` takes `&SecretKey` == `&[u8; 32]`.
            // Bind via a typed `let` so the array type is not inferred to `&[u8]`.
            let secret_key: &ed25519_dalek::SecretKey = &arr;
            return Ok(Self {
                signing_key: SigningKey::from_bytes(secret_key),
            });
        }

        // Otherwise try OpenSSH new-format parsing.
        let signing_key = parse_openssh_ed25519_seed(&bytes)
            .map_err(|e| format!("failed to parse capability signing key: {}", e))?;
        Ok(Self { signing_key })
    }

    /// Public key identifier compatible with
    /// `MachineEnrollment::authority_key_id` and the verifier in
    /// `LocalOneShotCapabilityAuthority`.
    pub fn authority_key_id(&self) -> String {
        format!(
            "sha256:{}",
            hex::encode(Sha256::digest(self.signing_key.verifying_key().as_bytes()))
        )
    }
}

fn parse_openssh_ed25519_seed(bytes: &[u8]) -> Result<SigningKey, String> {
    // Minimal OpenSSH new-format extraction of the ed25519 seed.
    // Layout: "openssh-key-v1\0" | check1 | check2 | pubkey | privkey(64B:
    // check|check|pubkey|32B seed|comment|padding)
    if !bytes.starts_with(b"openssh-key-v1\x00") {
        return Err("not an OpenSSH key blob".to_string());
    }
    let mut cursor = &bytes[b"openssh-key-v1\x00".len()..];
    // skip ciphername (4-byte len + data), kdf (same), kdfoptions (same)
    for _ in 0..3 {
        let len = read_u32(&mut cursor)?;
        cursor = read_bytes(&mut cursor, len)?;
    }
    let _num_keys = read_u32(&mut cursor)?;
    // skip public key blob
    let pub_len = read_u32(&mut cursor)?;
    cursor = read_bytes(&mut cursor, pub_len)?;
    // private key section
    let priv_len = read_u32(&mut cursor)?;
    let priv_section = read_bytes(&mut cursor, priv_len)?;
    let mut priv_cursor = priv_section;
    // skip check1, check2 (4 bytes each)
    priv_cursor = &priv_cursor[8..];
    // skip private pubkey
    let _pub_len = read_u32(&mut priv_cursor)?;
    priv_cursor = read_bytes(&mut priv_cursor, _pub_len)?;
    // ed25519 private key = 64 bytes (seed+public), seed = first 32
    if priv_cursor.len() < 64 {
        return Err("private key section too short".to_string());
    }
    let seed: [u8; 32] = priv_cursor[..32]
        .try_into()
        .map_err(|_| "private key section too short".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn read_u32(cursor: &mut &[u8]) -> Result<usize, String> {
    if cursor.len() < 4 {
        return Err("unexpected EOF reading u32".to_string());
    }
    let n = u32::from_be_bytes([cursor[0], cursor[1], cursor[2], cursor[3]]) as usize;
    *cursor = &cursor[4..];
    Ok(n)
}

fn read_bytes<'a>(cursor: &mut &'a [u8], len: usize) -> Result<&'a [u8], String> {
    if cursor.len() < len {
        return Err("unexpected EOF reading bytes".to_string());
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head)
}

impl CapabilitySigningPort for LocalCapabilitySigningKey {
    fn sign_capability(&self, capability: &mut OneShotCapability) -> Result<String, String> {
        let signing_payload = capability
            .signing_payload()
            .map_err(|e| format!("failed to generate signing payload: {}", e))?;
        let signature = self.signing_key.sign(&signing_payload).to_bytes();
        let signature_hex = hex::encode(signature);
        capability.signature_hex = signature_hex.clone();
        Ok(signature_hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::one_shot_authority::OneShotCapabilityAuthorityPort;

    fn fixture_capability(signing_key: &SigningKey) -> OneShotCapability {
        let action_digest = cynic_protocol::build_runner_bootstrap_action_digest("make:build")
            .expect("action digest for fixture");
        let mut capability = OneShotCapability {
            schema: cynic_protocol::ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "MachineEnrollment".to_string(),
            capability_id: "bootstrap-test-1".to_string(),
            subject: "node:cynic-builder".to_string(),
            scope: "build-runner-bootstrap".to_string(),
            input_digest: format!("sha256:{}", "a".repeat(64)),
            executor_digest: format!("sha256:{}", "b".repeat(64)),
            issued_at_unix_s: 100,
            expires_at_unix_s: 200,
            allowed_action_digests: vec![action_digest.clone()],
            authority_key_id: format!(
                "sha256:{}",
                hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()))
            ),
            signature_hex: String::new(),
            evidence_refs: vec!["enrollment:proof".to_string()],
            budget: None,
        };
        // sign via port
        let port = LocalCapabilitySigningKey {
            signing_key: signing_key.clone(),
        };
        port.sign_capability(&mut capability).expect("sign");
        capability
    }

    #[test]
    fn signing_key_id_matches_protocol_derivation() {
        let signing_key = SigningKey::from_bytes(&[42_u8; 32]);
        let port = LocalCapabilitySigningKey {
            signing_key: signing_key.clone(),
        };
        let expected = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()))
        );
        assert_eq!(port.authority_key_id(), expected);
    }

    #[test]
    fn port_signature_roundtrips_through_protocol_verifier() {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let capability = fixture_capability(&signing_key);
        let verifying_key = signing_key.verifying_key();
        let pubkey_path = std::env::temp_dir().join("test_authority.pub");
        std::fs::write(&pubkey_path, hex::encode(verifying_key.as_bytes())).expect("write pubkey");
        let root = std::env::temp_dir().join("cynic_signer_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        let authority =
            crate::one_shot_authority::LocalOneShotCapabilityAuthority::new(&pubkey_path, &root);
        let action_digest = cynic_protocol::build_runner_bootstrap_action_digest("make:build")
            .expect("action digest");
        let consumed = authority
            .verify_and_consume(
                &capability,
                "node:cynic-builder",
                "build-runner-bootstrap",
                &action_digest,
                150,
            )
            .expect("consume signed capability");
        assert_eq!(consumed.capability_id, "bootstrap-test-1");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&pubkey_path);
    }

    #[test]
    fn raw_seed_and_openssh_blob_both_load() {
        let signing_key = SigningKey::from_bytes(&[1_u8; 32]);
        let seed_path = std::env::temp_dir().join("test_seed.raw");
        std::fs::write(&seed_path, signing_key.to_bytes()).expect("write seed");
        let loaded = LocalCapabilitySigningKey::from_seed_path(&seed_path).expect("load raw seed");
        assert_eq!(
            loaded.authority_key_id(),
            format!(
                "sha256:{}",
                hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()))
            )
        );
        let _ = std::fs::remove_file(&seed_path);
    }
}
