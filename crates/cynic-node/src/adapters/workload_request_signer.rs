//! Shared workload request signer — one implementation for every adapter.
//!
//! Both the HTTP node-control adapter and the WebSocket federation client
//! authenticate with the same kernel wire scheme: `X-Cynic-Public-Key`,
//! `X-Cynic-Signature`, `X-Cynic-Timestamp` over
//! `timestamp + method + "|" + path + "|" + body`, signed by the node's
//! workload Ed25519 key (hex, 32 bytes, absolute file, never group/other
//! accessible). This module owns that scheme so the two adapters cannot drift.
//!
//! Key custody: the key is loaded from a principal-owned file at request time,
//! never logged, never placed in argv. Reports carry presence only.
//!
//! Crypto agility: `RequestSignatureAlgorithm` names the active scheme. The
//! deployed kernel verifies Ed25519 only, so Ed25519 is the sole variant
//! today. A post-quantum migration (recommended: hybrid Ed25519 + ML-DSA-44,
//! never stateful WOTS+/XMSS for a reconnecting node, Falcon-512 only after
//! FN-DSA implementations mature) requires a bilateral protocol change and a
//! new enum variant — never a silent payload change.

use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};

/// Active request-signature scheme. Ed25519-only until the kernel speaks PQC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestSignatureAlgorithm {
    Ed25519,
}

impl RequestSignatureAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            RequestSignatureAlgorithm::Ed25519 => "ed25519",
        }
    }
}

/// Signed request headers bound to one workload key file.
#[derive(Debug, Clone)]
pub struct RequestSignature {
    pub algorithm: RequestSignatureAlgorithm,
    pub public_key_hex: String,
    pub signature_hex: String,
    pub timestamp_secs: String,
}

/// File-backed workload identity shared by node adapters.
#[derive(Debug, Clone)]
pub struct WorkloadRequestSigner {
    signing_key_path: PathBuf,
    algorithm: RequestSignatureAlgorithm,
}

impl WorkloadRequestSigner {
    pub fn new(signing_key_path: PathBuf) -> Result<Self, String> {
        validate_private_key_file(&signing_key_path)?;
        Ok(Self {
            signing_key_path,
            algorithm: RequestSignatureAlgorithm::Ed25519,
        })
    }

    pub fn algorithm(&self) -> RequestSignatureAlgorithm {
        self.algorithm
    }

    pub fn key_present(&self) -> bool {
        self.signing_key_path.is_file()
    }

    pub fn load(&self) -> Result<SigningKey, String> {
        let raw = std::fs::read_to_string(&self.signing_key_path).map_err(|error| {
            format!(
                "read workload identity {}: {error}",
                self.signing_key_path.display()
            )
        })?;
        let bytes = hex::decode(raw.trim())
            .map_err(|_| "workload signing key must be hex encoded".to_string())?;
        let secret: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "workload signing key must contain 32 bytes".to_string())?;
        Ok(SigningKey::from_bytes(&secret))
    }

    /// Sign one request. Timestamp must be unix seconds as decimal string.
    pub fn sign_request(
        &self,
        timestamp_secs: &str,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<RequestSignature, String> {
        let signing_key = self.load()?;
        let public_key_hex = hex::encode(signing_key.verifying_key().as_bytes());
        let payload = signed_payload(timestamp_secs, method, path, body);
        let signature_hex = hex::encode(signing_key.sign(&payload).to_bytes());
        Ok(RequestSignature {
            algorithm: self.algorithm,
            public_key_hex,
            signature_hex,
            timestamp_secs: timestamp_secs.to_string(),
        })
    }
}

/// Pure wire format: `timestamp + method + "|" + path + "|" + body`.
pub(crate) fn signed_payload(timestamp: &str, method: &str, path: &str, body: &[u8]) -> Vec<u8> {
    let mut payload = timestamp.as_bytes().to_vec();
    payload.extend_from_slice(method.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(path.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(body);
    payload
}

fn validate_private_key_file(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || !path.is_file() {
        return Err("workload signing key must be an existing absolute file".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = std::fs::metadata(path)
            .map_err(|error| format!("inspect workload identity: {error}"))?
            .mode()
            & 0o777;
        if mode & 0o027 != 0 {
            return Err(
                "workload signing key must not be group-writable or accessible by others"
                    .to_string(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_payload_matches_kernel_wire_format() {
        assert_eq!(
            signed_payload("12", "GET", "/node-control/node-a/desired-state", &[]),
            b"12GET|/node-control/node-a/desired-state|"
        );
    }

    #[test]
    fn ws_handshake_payload_format() {
        assert_eq!(
            signed_payload("1700000000", "GET", "/events/ws", &[]),
            b"1700000000GET|/events/ws|"
        );
    }
}
