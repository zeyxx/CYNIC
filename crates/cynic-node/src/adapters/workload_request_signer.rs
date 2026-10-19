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

use cynic_protocol::workload_signature::{
    HEADER_PQC_ALGORITHM, HEADER_PQC_PUBLIC_KEY, HEADER_PQC_SIGNATURE, HEADER_PUBLIC_KEY,
    HEADER_SIGNATURE, HEADER_TIMESTAMP, WorkloadSignatureAlgorithm, workload_signed_payload,
};

use crate::adapters::pqc_workload_signer::PqcWorkloadSigner;

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
    /// Post-quantum half of a hybrid request. `None` when the node has no
    /// post-quantum key configured — the classical signature alone then
    /// travels, exactly as before the migration.
    pub pqc: Option<PqcSignatureMaterial>,
}

/// Post-quantum signature material carried alongside the classical one.
#[derive(Debug, Clone)]
pub struct PqcSignatureMaterial {
    pub public_key_hex: String,
    pub signature_hex: String,
    pub algorithm: WorkloadSignatureAlgorithm,
}

/// File-backed workload identity shared by node adapters.
#[derive(Debug, Clone)]
pub struct WorkloadRequestSigner {
    signing_key_path: PathBuf,
    algorithm: RequestSignatureAlgorithm,
    /// Post-quantum signer. `None` when the node has no post-quantum key
    /// configured: the request then carries the classical signature alone,
    /// exactly as before the migration.
    pqc_signer: Option<PqcWorkloadSigner>,
}

impl WorkloadRequestSigner {
    pub fn new(signing_key_path: PathBuf) -> Result<Self, String> {
        validate_private_key_file(&signing_key_path)?;
        Ok(Self {
            signing_key_path,
            algorithm: RequestSignatureAlgorithm::Ed25519,
            pqc_signer: None,
        })
    }

    /// Attach a post-quantum signer, turning every request into a hybrid one.
    ///
    /// The classical signature stays byte-identical on the wire, so a kernel
    /// that has not migrated keeps working: it simply ignores the
    /// post-quantum headers.
    pub fn with_pqc_signer(mut self, pqc_signer: PqcWorkloadSigner) -> Self {
        self.pqc_signer = Some(pqc_signer);
        self
    }

    /// True when this signer produces hybrid (classical + post-quantum)
    /// signatures.
    pub fn hybrid(&self) -> bool {
        self.pqc_signer.is_some()
    }

    /// True when a post-quantum key file is present on disk.
    pub fn pqc_key_present(&self) -> bool {
        self.pqc_signer
            .as_ref()
            .map(|signer| signer.key_present())
            .unwrap_or(false)
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
    ///
    /// When a post-quantum signer is attached, the SAME payload bytes are
    /// signed twice — once with Ed25519, once with ML-DSA-44 — so the two
    /// signatures attest to one message, not two.
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
        let pqc = match &self.pqc_signer {
            None => None,
            Some(pqc_signer) => {
                let signature = pqc_signer
                    .sign(&payload)
                    .map_err(|error| format!("post-quantum workload signature: {error}"))?;
                Some(PqcSignatureMaterial {
                    public_key_hex: signature.public_key_hex,
                    signature_hex: signature.signature_hex,
                    algorithm: signature.algorithm,
                })
            }
        };
        Ok(RequestSignature {
            algorithm: self.algorithm,
            public_key_hex,
            signature_hex,
            timestamp_secs: timestamp_secs.to_string(),
            pqc,
        })
    }
}

/// Pure wire format: `timestamp + method + "|" + path + "|" + body`.
///
/// Delegates to the canonical contract in `cynic-protocol` so the node and
/// the kernel cannot drift on what exactly gets signed.
pub(crate) fn signed_payload(timestamp: &str, method: &str, path: &str, body: &[u8]) -> Vec<u8> {
    workload_signed_payload(timestamp, method, path, body)
}

/// Apply the workload signature headers to a request builder.
///
/// One application point for every adapter: the classical headers and the
/// optional post-quantum ones are emitted together, so a hybrid request
/// cannot exist on one transport and be missing on another. The header
/// names come from the canonical contract, not from a local literal.
pub(crate) fn apply_signature_headers<B>(builder: B, signature: &RequestSignature) -> B
where
    B: WorkloadHeaderBuilder,
{
    let builder = builder
        .header(HEADER_PUBLIC_KEY, signature.public_key_hex.clone())
        .header(HEADER_SIGNATURE, signature.signature_hex.clone())
        .header(HEADER_TIMESTAMP, signature.timestamp_secs.clone());
    match &signature.pqc {
        None => builder,
        Some(pqc) => builder
            .header(HEADER_PQC_PUBLIC_KEY, pqc.public_key_hex.clone())
            .header(HEADER_PQC_SIGNATURE, pqc.signature_hex.clone())
            .header(HEADER_PQC_ALGORITHM, pqc.algorithm.name().to_string()),
    }
}

/// Minimal request-builder shape shared by the HTTP and WebSocket clients.
///
/// Both `reqwest::RequestBuilder` and `http::request::Builder` offer a
/// `header` method returning the same builder type; this trait names that
/// shape once so the header application above is written once.
pub(crate) trait WorkloadHeaderBuilder {
    /// Attach one header, replacing any existing value.
    fn header(self, name: &str, value: String) -> Self;
}

impl WorkloadHeaderBuilder for reqwest::RequestBuilder {
    fn header(self, name: &str, value: String) -> Self {
        // Fully qualified: an unqualified `self.header` would resolve to
        // this trait method and recurse forever.
        reqwest::RequestBuilder::header(self, name, value)
    }
}

impl WorkloadHeaderBuilder for http::request::Builder {
    fn header(self, name: &str, value: String) -> Self {
        http::request::Builder::header(self, name, value)
    }
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
    use crate::adapters::pqc_workload_signer::{
        PqcWorkloadSigner, generate_pqc_keypair, verify_pqc_signature,
    };
    use std::os::unix::fs::PermissionsExt;

    /// The classical verification trait, needed to check the Ed25519 half
    /// of a hybrid request independently.
    use ed25519_dalek::Verifier;

    /// Write a 32-byte Ed25519 key (hex text, the classical on-disk format)
    /// and return its absolute path.
    fn ed25519_key_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        path.push(format!("cynic-ed25519-key-{}-{seq}", std::process::id()));
        let secret = [7u8; 32];
        std::fs::write(&path, hex::encode(secret)).expect("write key");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod key");
        path
    }

    /// Write an ML-DSA-44 key (raw bytes) and return its absolute path.
    fn pqc_key_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        path.push(format!("cynic-pqc-key-{}-{seq}", std::process::id()));
        let (_, private_hex) = generate_pqc_keypair().expect("keygen");
        std::fs::write(&path, hex::decode(private_hex).expect("private key is hex"))
            .expect("write key");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod key");
        path
    }

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

    #[test]
    fn a_classical_only_signer_carries_no_post_quantum_half() {
        let key_path = ed25519_key_path();
        let signer = WorkloadRequestSigner::new(key_path.clone()).expect("signer");
        assert!(!signer.hybrid());
        assert!(!signer.pqc_key_present());

        let signature = signer
            .sign_request("1700000000", "GET", "/events/ws", &[])
            .expect("sign");
        assert!(signature.pqc.is_none());
        let _ = std::fs::remove_file(key_path);
    }

    #[test]
    fn a_hybrid_signer_signs_the_same_payload_with_both_algorithms() {
        let key_path = ed25519_key_path();
        let pqc_path = pqc_key_path();
        let signer = WorkloadRequestSigner::new(key_path.clone())
            .expect("signer")
            .with_pqc_signer(PqcWorkloadSigner::new(pqc_path.clone()).expect("pqc signer"));
        assert!(signer.hybrid());

        let signature = signer
            .sign_request("1700000000", "GET", "/events/ws", &[])
            .expect("sign");
        let pqc = signature
            .pqc
            .expect("hybrid request carries a post-quantum half");

        // Both signatures must verify over the SAME payload bytes: that is
        // what makes the pair one hybrid assertion, not two unrelated ones.
        let payload = signed_payload("1700000000", "GET", "/events/ws", &[]);
        assert_eq!(
            verify_pqc_signature(&pqc.public_key_hex, &pqc.signature_hex, &payload),
            Ok(true)
        );
        assert_eq!(pqc.algorithm, WorkloadSignatureAlgorithm::MlDsa44);

        let _ = std::fs::remove_file(key_path);
        let _ = std::fs::remove_file(pqc_path);
    }

    #[test]
    fn a_hybrid_request_still_verifies_classically_on_its_own() {
        let key_path = ed25519_key_path();
        let pqc_path = pqc_key_path();
        let signer = WorkloadRequestSigner::new(key_path.clone())
            .expect("signer")
            .with_pqc_signer(PqcWorkloadSigner::new(pqc_path.clone()).expect("pqc signer"));

        let signature = signer
            .sign_request("1700000000", "GET", "/events/ws", &[])
            .expect("sign");
        let payload = signed_payload("1700000000", "GET", "/events/ws", &[]);

        let key_bytes = hex::decode(&signature.public_key_hex).expect("public key is hex");
        let verifying_key =
            ed25519_dalek::VerifyingKey::from_bytes(&key_bytes.try_into().expect("32 bytes"))
                .expect("verifying key");
        let sig_bytes = hex::decode(&signature.signature_hex).expect("signature is hex");
        let ed_sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().expect("64 bytes"));
        assert!(verifying_key.verify(&payload, &ed_sig).is_ok());

        let _ = std::fs::remove_file(key_path);
        let _ = std::fs::remove_file(pqc_path);
    }
}
