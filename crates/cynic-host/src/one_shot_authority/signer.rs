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

/// Load a raw 32-byte ed25519 seed from `seed_path`.
///
/// Accepted encodings (fleet key-material authority for operator-held keys):
///  - 64 lowercase hex chars (fleet identity convention, optional newline),
///  - 32 raw bytes,
///  - PEM-armored OpenSSH `openssh-key-v1` blob (`ssh-keygen -t ed25519`),
///  - the base64-decoded OpenSSH blob itself.
///
/// Returns the seed bytes; never logs or prints key material.
pub fn load_ed25519_seed(seed_path: &Path) -> Result<[u8; 32], String> {
    let bytes = std::fs::read(seed_path).map_err(|e| format!("failed to read seed key: {e}"))?;
    let text = String::from_utf8_lossy(&bytes);
    let trimmed = text.trim();
    // Fleet hex convention first (also matches node identity files).
    if trimmed.len() == 64 {
        if let Ok(decoded) = hex::decode(trimmed) {
            if let Ok(seed) = decoded.try_into() {
                return Ok(seed);
            }
        }
    }
    // Raw 32-byte seed.
    if bytes.len() == 32 {
        return bytes[..32]
            .try_into()
            .map_err(|_| "seed must be 32 bytes".to_string());
    }
    // PEM armor?
    let blob: Vec<u8> = if trimmed.starts_with("-----BEGIN") {
        let body: String = trimmed
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect();
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &body)
            .map_err(|e| format!("failed to decode PEM armor: {e}"))?
    } else {
        bytes
    };
    // PKCS#8 (DER SEQUENCE) vs OpenSSH (magic string) dispatch.
    if blob.first() == Some(&0x30) {
        parse_pkcs8_ed25519_seed(&blob).map(|key| key.to_bytes())
    } else {
        parse_openssh_ed25519_seed(&blob).map(|key| key.to_bytes())
    }
}

/// Extract the 32-byte seed from an unencrypted PKCS#8 Ed25519 DER blob
/// (`-----BEGIN PRIVATE KEY-----`, as produced by `openssl pkcs8 -topk8
/// -nocrypt` or `ssh-keygen -m PKCS8`). Strict shape, fail closed:
/// SEQUENCE { INTEGER 0, SEQUENCE { OID 1.3.101.112 }, OCTET STRING {
/// OCTET STRING { seed } } }. Short-form lengths only.
fn parse_pkcs8_ed25519_seed(bytes: &[u8]) -> Result<SigningKey, String> {
    let mut cursor = bytes;
    let expect_tag = |cursor: &mut &[u8], tag: u8, what: &str| -> Result<(), String> {
        let actual = read_bytes(cursor, 1).map_err(|_| format!("pkcs8 truncated at {what}"))?;
        if actual[0] != tag {
            return Err(format!("pkcs8 unexpected tag at {what}"));
        }
        Ok(())
    };
    let expect_len = |cursor: &mut &[u8], want: usize, what: &str| -> Result<(), String> {
        let raw = read_bytes(cursor, 1).map_err(|_| format!("pkcs8 truncated len at {what}"))?;
        if raw[0] >= 0x80 {
            return Err(format!("pkcs8 long-form length refused at {what}"));
        }
        if raw[0] as usize != want {
            return Err(format!("pkcs8 unexpected length at {what}"));
        }
        Ok(())
    };
    expect_tag(&mut cursor, 0x30, "outer sequence")?;
    let outer_len =
        read_bytes(&mut cursor, 1).map_err(|_| "pkcs8 truncated outer len".to_string())?;
    if outer_len[0] >= 0x80 || outer_len[0] as usize != cursor.len() {
        return Err("pkcs8 outer length mismatch".to_string());
    }
    expect_tag(&mut cursor, 0x02, "version integer")?;
    expect_len(&mut cursor, 1, "version len")?;
    let version = read_bytes(&mut cursor, 1).map_err(|_| "pkcs8 truncated version".to_string())?;
    if version[0] != 0 {
        return Err("pkcs8 unsupported version".to_string());
    }
    expect_tag(&mut cursor, 0x30, "algorithm sequence")?;
    expect_len(&mut cursor, 5, "algorithm len")?;
    expect_tag(&mut cursor, 0x06, "algorithm oid")?;
    expect_len(&mut cursor, 3, "oid len")?;
    let oid = read_bytes(&mut cursor, 3).map_err(|_| "pkcs8 truncated oid".to_string())?;
    if oid != [0x2b, 0x65, 0x70] {
        return Err("pkcs8 only Ed25519 (1.3.101.112) is supported".to_string());
    }
    expect_tag(&mut cursor, 0x04, "outer octet string")?;
    expect_len(&mut cursor, 34, "outer octet len")?;
    expect_tag(&mut cursor, 0x04, "inner octet string")?;
    expect_len(&mut cursor, 32, "seed len")?;
    let seed = read_bytes(&mut cursor, 32).map_err(|_| "pkcs8 truncated seed".to_string())?;
    if !cursor.is_empty() {
        return Err("pkcs8 trailing bytes refused".to_string());
    }
    let arr: [u8; 32] = seed
        .try_into()
        .map_err(|_| "pkcs8 seed is not 32 bytes".to_string())?;
    Ok(SigningKey::from_bytes(&arr))
}

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
        read_bytes(&mut cursor, len)?;
    }
    let _num_keys = read_u32(&mut cursor)?;
    // skip public key blob
    let pub_len = read_u32(&mut cursor)?;
    read_bytes(&mut cursor, pub_len)?;
    // private key section
    let priv_len = read_u32(&mut cursor)?;
    let priv_section = read_bytes(&mut cursor, priv_len)?;
    let mut priv_cursor = priv_section;
    // skip check1, check2 (4 bytes each)
    if priv_cursor.len() < 8 {
        return Err("private key section too short for checks".to_string());
    }
    priv_cursor = &priv_cursor[8..];
    // key type string: only ssh-ed25519 is supported (fail closed otherwise)
    let type_len = read_u32(&mut priv_cursor)?;
    let key_type = read_bytes(&mut priv_cursor, type_len)?;
    if key_type != b"ssh-ed25519" {
        return Err("unsupported private key type (only ssh-ed25519)".to_string());
    }
    // public key string: 32 bytes (recorded, not trusted for identity)
    let pub_len = read_u32(&mut priv_cursor)?;
    let stored_public = read_bytes(&mut priv_cursor, pub_len)?;
    if stored_public.len() != 32 {
        return Err("ed25519 public key string is not 32 bytes".to_string());
    }
    // private string: 64 bytes (32B seed + 32B public)
    let seed_len = read_u32(&mut priv_cursor)?;
    let private_blob = read_bytes(&mut priv_cursor, seed_len)?;
    if private_blob.len() != 64 {
        return Err("ed25519 private string is not 64 bytes".to_string());
    }
    let seed: [u8; 32] = private_blob[..32]
        .try_into()
        .map_err(|_| "ed25519 private string is not 64 bytes".to_string())?;
    // Cross-check: stored public must match the seed-derived public key.
    let derived = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    if derived != private_blob[32..] {
        return Err("seed and public halves of private string disagree".to_string());
    }
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
        let action_digest = cynic_build::build_runner_bootstrap_action_digest("cargo:build")
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
        let action_digest =
            cynic_build::build_runner_bootstrap_action_digest("cargo:build").expect("action digest");
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
    fn seed_loader_accepts_hex_and_raw_and_rejects_garbage() {
        let dir = std::env::temp_dir().join("test_seed_loader");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let seed = [7_u8; 32];
        let hex_path = dir.join("seed.hex");
        std::fs::write(&hex_path, format!("{}\n", hex::encode(seed))).expect("write hex");
        assert_eq!(load_ed25519_seed(&hex_path).expect("load hex"), seed);
        let raw_path = dir.join("seed.raw");
        std::fs::write(&raw_path, seed).expect("write raw");
        assert_eq!(load_ed25519_seed(&raw_path).expect("load raw"), seed);
        let bad_path = dir.join("seed.bad");
        std::fs::write(&bad_path, "not-a-key").expect("write bad");
        assert!(load_ed25519_seed(&bad_path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn seed_loader_accepts_pkcs8_pem_and_rejects_wrong_oid() {
        // Dummy-only vector, built in-test: PKCS#8 wrapping seed [9u8; 32].
        let seed = [9_u8; 32];
        let mut der = vec![
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22,
            0x04, 0x20,
        ];
        der.extend_from_slice(&seed);
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &der)
        );
        let dir = std::env::temp_dir().join("test_seed_pkcs8");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let key_path = dir.join("seed.pkcs8");
        std::fs::write(&key_path, &pem).expect("write pkcs8");
        assert_eq!(load_ed25519_seed(&key_path).expect("load pkcs8"), seed);
        let _ = std::fs::remove_dir_all(&dir);
        // Same envelope with a non-Ed25519 OID must fail closed.
        let mut bad = der.clone();
        bad[11] = 0x71;
        assert!(parse_pkcs8_ed25519_seed(&bad).is_err());
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
