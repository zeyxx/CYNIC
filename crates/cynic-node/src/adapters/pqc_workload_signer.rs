//! ML-DSA-44 workload signature — the post-quantum half of the hybrid scheme.
//!
//! Signs the SAME payload bytes as the Ed25519 half
//! (`timestamp + method + "|" + path + "|" + body`), so a hybrid request
//! carries two independent signatures over one message. The classical
//! signature stays byte-identical on the wire: a kernel that has not
//! migrated ignores these headers instead of failing.
//!
//! Key custody matches the classical signer: the key lives in a
//! principal-owned file (absolute path, never group/other accessible), is
//! read at request time, and is never logged or placed in argv.
//!
//! Why ML-DSA-44 and not the alternatives: stateless (a reconnecting node
//! must never reuse one-time state, so WOTS+/XMSS/LMS are excluded), pure
//! integer arithmetic (constant-time without a float FFT, so Falcon-512
//! waits for mature FN-DSA implementations), and the smallest of the
//! FIPS 204 parameter sets at NIST security category 2.

use std::path::{Path, PathBuf};

use fips204::ml_dsa_44;
use fips204::traits::{SerDes, Signer, Verifier};

use cynic_protocol::workload_signature::WorkloadSignatureAlgorithm;

/// Hex-encoded ML-DSA-44 material, ready for the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PqcSignature {
    /// Hex-encoded public key (1312 bytes for ML-DSA-44).
    pub public_key_hex: String,
    /// Hex-encoded signature (2420 bytes for ML-DSA-44).
    pub signature_hex: String,
    /// Algorithm name carried on the wire.
    pub algorithm: WorkloadSignatureAlgorithm,
}

/// Errors from the post-quantum half. Each names the failing step so the
/// operator sees a machine-readable reason, never a generic "sign failed".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PqcSignerError {
    /// The key file is missing, unreadable, or has unsafe permissions.
    KeyFile(String),
    /// The key file does not decode to the expected fixed-size key.
    KeyDecode(String),
    /// Signing failed.
    Sign(String),
    /// Verification failed.
    Verify(String),
}

impl std::fmt::Display for PqcSignerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PqcSignerError::KeyFile(detail) => write!(f, "pqc key file unusable: {detail}"),
            PqcSignerError::KeyDecode(detail) => write!(f, "pqc key malformed: {detail}"),
            PqcSignerError::Sign(detail) => write!(f, "pqc signing failed: {detail}"),
            PqcSignerError::Verify(detail) => write!(f, "pqc verification failed: {detail}"),
        }
    }
}

impl std::error::Error for PqcSignerError {}

/// File-backed ML-DSA-44 workload identity.
#[derive(Debug, Clone)]
pub struct PqcWorkloadSigner {
    signing_key_path: PathBuf,
}

impl PqcWorkloadSigner {
    /// Bind a signer to a key file. The file is validated (absolute path,
    /// not group/other accessible) but NOT read: the key is loaded at
    /// signing time so a missing key surfaces on the request, not at
    /// startup.
    pub fn new(signing_key_path: PathBuf) -> Result<Self, PqcSignerError> {
        validate_private_key_file(&signing_key_path)?;
        Ok(Self { signing_key_path })
    }

    /// True when the key file is present on disk.
    pub fn key_present(&self) -> bool {
        self.signing_key_path.is_file()
    }

    /// Path of the bound key file (never the key material).
    pub fn key_path(&self) -> &Path {
        &self.signing_key_path
    }

    /// Load the ML-DSA-44 private key from disk.
    fn load(&self) -> Result<ml_dsa_44::PrivateKey, PqcSignerError> {
        let raw = std::fs::read(&self.signing_key_path).map_err(|error| {
            PqcSignerError::KeyFile(format!("read {}: {error}", self.signing_key_path.display()))
        })?;
        let raw: [u8; ml_dsa_44::SK_LEN] = raw.try_into().map_err(|bytes: Vec<u8>| {
            PqcSignerError::KeyDecode(format!(
                "{}: {} bytes, expected {}",
                self.signing_key_path.display(),
                bytes.len(),
                ml_dsa_44::SK_LEN
            ))
        })?;
        ml_dsa_44::PrivateKey::try_from_bytes(raw).map_err(|detail| {
            PqcSignerError::KeyDecode(format!("{}: {detail}", self.signing_key_path.display()))
        })
    }

    /// Sign a payload and return the hex-encoded public key and signature.
    ///
    /// The public key travels with the signature so a verifier can check the
    /// signature without a separate key-distribution channel; the kernel
    /// still binds that key to a workload principal before trusting it.
    pub fn sign(&self, payload: &[u8]) -> Result<PqcSignature, PqcSignerError> {
        let private_key = self.load()?;
        let signature = private_key
            .try_sign(payload, &[])
            .map_err(|detail| PqcSignerError::Sign(detail.to_string()))?;
        let public_key = private_key.get_public_key();
        Ok(PqcSignature {
            public_key_hex: hex::encode(public_key.into_bytes()),
            signature_hex: hex::encode(signature),
            algorithm: WorkloadSignatureAlgorithm::MlDsa44,
        })
    }
}

/// Verify a detached ML-DSA-44 signature over `payload`.
///
/// `public_key_hex` and `signature_hex` are hex. A malformed key or
/// signature is an error, never a silent "invalid" — the two failures
/// mean different things operationally.
pub fn verify_pqc_signature(
    public_key_hex: &str,
    signature_hex: &str,
    payload: &[u8],
) -> Result<bool, PqcSignerError> {
    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|error| PqcSignerError::KeyDecode(format!("public key is not hex: {error}")))?;
    let public_key_bytes: [u8; ml_dsa_44::PK_LEN] =
        public_key_bytes.try_into().map_err(|bytes: Vec<u8>| {
            PqcSignerError::KeyDecode(format!(
                "public key is {} bytes, expected {}",
                bytes.len(),
                ml_dsa_44::PK_LEN
            ))
        })?;
    let public_key = ml_dsa_44::PublicKey::try_from_bytes(public_key_bytes)
        .map_err(|detail| PqcSignerError::KeyDecode(format!("public key rejected: {detail}")))?;
    let signature_bytes = hex::decode(signature_hex)
        .map_err(|error| PqcSignerError::Verify(format!("signature is not hex: {error}")))?;
    let signature: [u8; ml_dsa_44::SIG_LEN] =
        signature_bytes.try_into().map_err(|bytes: Vec<u8>| {
            PqcSignerError::Verify(format!(
                "signature is {} bytes, expected {}",
                bytes.len(),
                ml_dsa_44::SIG_LEN
            ))
        })?;
    Ok(public_key.verify(payload, &signature, &[]))
}

/// Generate a fresh ML-DSA-44 keypair as hex strings.
///
/// Returns `(public_key_hex, private_key_hex)`. The hex form is the WIRE
/// form (public key travels in a header); the on-disk key file holds the
/// RAW serialized key bytes, which is what [`PqcWorkloadSigner`] reads.
/// The private key never leaves this process in hex form unless a caller
/// explicitly persists it.
pub fn generate_pqc_keypair() -> Result<(String, String), PqcSignerError> {
    let (public_key, private_key) =
        ml_dsa_44::try_keygen().map_err(|detail| PqcSignerError::KeyDecode(detail.to_string()))?;
    Ok((
        hex::encode(public_key.into_bytes()),
        hex::encode(private_key.into_bytes()),
    ))
}

/// Write a fresh ML-DSA-44 keypair to disk in the format the signer reads.
///
/// The private key is written RAW (2560 bytes), the public key is written
/// as hex text next to it (`.pub`) so an operator can enroll it without
/// decoding anything. Both files are owner-only.
pub fn generate_pqc_keypair_files(
    private_key_path: &Path,
) -> Result<(String, PathBuf), PqcSignerError> {
    use std::os::unix::fs::PermissionsExt;
    let (public_hex, private_hex) = generate_pqc_keypair()?;
    let private_bytes = hex::decode(&private_hex)
        .map_err(|error| PqcSignerError::KeyDecode(format!("private key is not hex: {error}")))?;
    std::fs::write(private_key_path, &private_bytes).map_err(|error| {
        PqcSignerError::KeyFile(format!("write {}: {error}", private_key_path.display()))
    })?;
    std::fs::set_permissions(private_key_path, std::fs::Permissions::from_mode(0o600)).map_err(
        |error| PqcSignerError::KeyFile(format!("chmod {}: {error}", private_key_path.display())),
    )?;
    let public_path = private_key_path.with_extension("pub");
    std::fs::write(&public_path, format!("{public_hex}\n")).map_err(|error| {
        PqcSignerError::KeyFile(format!("write {}: {error}", public_path.display()))
    })?;
    std::fs::set_permissions(&public_path, std::fs::Permissions::from_mode(0o644)).map_err(
        |error| PqcSignerError::KeyFile(format!("chmod {}: {error}", public_path.display())),
    )?;
    Ok((public_hex, public_path))
}

fn validate_private_key_file(path: &Path) -> Result<(), PqcSignerError> {
    if !path.is_absolute() {
        return Err(PqcSignerError::KeyFile(
            "pqc signing key must be an absolute path".to_string(),
        ));
    }
    if !path.is_file() {
        return Err(PqcSignerError::KeyFile(format!(
            "{} is not an existing file",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = std::fs::metadata(path)
            .map_err(|error| PqcSignerError::KeyFile(format!("inspect key: {error}")))?
            .mode()
            & 0o777;
        if mode & 0o027 != 0 {
            return Err(PqcSignerError::KeyFile(
                "pqc signing key must not be group-writable or accessible by others".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_key_path(contents: &[u8]) -> PathBuf {
        let mut path = std::env::temp_dir();
        // Unique per call: the tests run in parallel, so a name derived from
        // the content length alone would make them overwrite each other's
        // key file and sign with another test's key.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        path.push(format!(
            "cynic-pqc-key-{}-{seq}-{}",
            std::process::id(),
            contents.len()
        ));
        std::fs::write(&path, contents).expect("write key");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod key");
        path
    }

    fn generated_key() -> (PathBuf, String) {
        let (public_hex, private_hex) = generate_pqc_keypair().expect("keygen");
        // The key file holds RAW bytes, not hex: the on-disk format is the
        // serialized key itself, so a 2560-byte key is 2560 bytes on disk.
        let path = temp_key_path(&hex::decode(private_hex).expect("private key is hex"));
        (path, public_hex)
    }

    #[test]
    fn signs_and_verifies_the_same_payload() {
        let (path, public_hex) = generated_key();
        let signer = PqcWorkloadSigner::new(path.clone()).expect("signer");
        let payload = b"1700000000GET|/events/ws|";
        let signature = signer.sign(payload).expect("sign");

        assert_eq!(signature.public_key_hex, public_hex);
        assert_eq!(
            verify_pqc_signature(&signature.public_key_hex, &signature.signature_hex, payload),
            Ok(true)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_tampered_payload_does_not_verify() {
        let (path, _) = generated_key();
        let signer = PqcWorkloadSigner::new(path.clone()).expect("signer");
        let signature = signer.sign(b"1700000000GET|/events/ws|").expect("sign");

        assert_eq!(
            verify_pqc_signature(
                &signature.public_key_hex,
                &signature.signature_hex,
                b"1700000000GET|/other|"
            ),
            Ok(false)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_tampered_signature_does_not_verify() {
        let (path, _) = generated_key();
        let signer = PqcWorkloadSigner::new(path.clone()).expect("signer");
        let payload = b"payload";
        let signature = signer.sign(payload).expect("sign");
        let mut tampered = signature.signature_hex.clone();
        tampered.replace_range(0..2, "00");

        assert_eq!(
            verify_pqc_signature(&signature.public_key_hex, &tampered, payload),
            Ok(false)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn malformed_public_key_is_an_error_not_a_false() {
        let error = verify_pqc_signature("not-hex", "aabb", b"payload")
            .expect_err("malformed key must not verify");
        assert!(matches!(error, PqcSignerError::KeyDecode(_)));
    }

    #[test]
    fn malformed_signature_is_an_error_not_a_false() {
        let (public_hex, _) = generate_pqc_keypair().expect("keygen");
        let error = verify_pqc_signature(&public_hex, "not-hex", b"payload")
            .expect_err("malformed signature must not verify");
        assert!(matches!(error, PqcSignerError::Verify(_)));
    }

    #[test]
    fn a_world_writable_key_file_is_refused() {
        let (path, _) = generated_key();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).expect("chmod key");
        let error = PqcWorkloadSigner::new(path.clone()).expect_err("unsafe key must be refused");
        assert!(matches!(error, PqcSignerError::KeyFile(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_relative_key_path_is_refused() {
        let error = PqcWorkloadSigner::new(PathBuf::from("relative/key")).expect_err("relative");
        assert!(matches!(error, PqcSignerError::KeyFile(_)));
    }

    #[test]
    fn a_missing_key_file_is_refused_at_construction() {
        let error = PqcWorkloadSigner::new(PathBuf::from("/nonexistent/cynic-pqc-key"))
            .expect_err("missing key");
        assert!(matches!(error, PqcSignerError::KeyFile(_)));
    }

    #[test]
    fn a_malformed_key_file_is_refused_at_signing() {
        let path = temp_key_path(b"not-a-key");
        let signer = PqcWorkloadSigner::new(path.clone()).expect("signer");
        let error = signer
            .sign(b"payload")
            .expect_err("malformed key must not sign");
        assert!(matches!(error, PqcSignerError::KeyDecode(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_signature_carries_the_ml_dsa_44_algorithm_name() {
        let (path, _) = generated_key();
        let signer = PqcWorkloadSigner::new(path.clone()).expect("signer");
        let signature = signer.sign(b"payload").expect("sign");
        assert_eq!(signature.algorithm, WorkloadSignatureAlgorithm::MlDsa44);
        assert_eq!(signature.algorithm.name(), "ml-dsa-44");
        let _ = std::fs::remove_file(path);
    }
}
