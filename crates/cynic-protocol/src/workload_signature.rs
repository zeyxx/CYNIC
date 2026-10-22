//! Workload request signature scheme — the wire contract, pure.
//!
//! One canonical description of how a node authenticates a request to the
//! kernel: which algorithm names exist, which headers carry each part, and
//! which bytes are signed. The node client (`cynic-node`) and the kernel
//! verifier must not drift, so the scheme lives here as data and pure
//! functions — no I/O, no clock, no filesystem.
//!
//! Scope: this module owns the *scheme*, not the cryptography. Signature
//! primitives stay in the adapters that perform them (`ed25519-dalek`,
//! `fips204`); this module decides which headers exist, how a payload is
//! built, and what a verifier must require.
//!
//! Crypto posture: the deployed kernel verifies Ed25519 only. A
//! post-quantum migration is hybrid by construction — Ed25519 stays on the
//! wire byte-for-byte (old servers keep working) and ML-DSA-44 is added as
//! a second signature over the SAME payload bytes. Stateful hash-based
//! signatures (WOTS+/XMSS/LMS) are excluded on purpose: a reconnecting node
//! must never reuse one-time state.

use serde::{Deserialize, Serialize};

/// Canonical scheme identifier for the workload request signature.
pub const WORKLOAD_SIGNATURE_SCHEME: &str = "cynic.workload_request_signature.v1";

/// Header carrying the Ed25519 public key (hex, 32 bytes).
pub const HEADER_PUBLIC_KEY: &str = "X-Cynic-Public-Key";
/// Header carrying the Ed25519 signature (hex, 64 bytes).
pub const HEADER_SIGNATURE: &str = "X-Cynic-Signature";
/// Header carrying the signed unix-seconds timestamp (decimal string).
pub const HEADER_TIMESTAMP: &str = "X-Cynic-Timestamp";
/// Header carrying the post-quantum public key (hex, ML-DSA-44).
pub const HEADER_PQC_PUBLIC_KEY: &str = "X-Cynic-Public-Key-Pqc";
/// Header carrying the post-quantum signature (hex, ML-DSA-44).
pub const HEADER_PQC_SIGNATURE: &str = "X-Cynic-Signature-Pqc";
/// Header naming the post-quantum algorithm in use.
pub const HEADER_PQC_ALGORITHM: &str = "X-Cynic-Algorithm-Pqc";

/// Maximum age of a signed request, in seconds (replay window).
pub const MAX_REQUEST_AGE_SECS: u64 = 300;
/// Maximum tolerated clock drift into the future, in seconds.
pub const MAX_CLOCK_DRIFT_SECS: u64 = 30;

/// Signature algorithms a workload request may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadSignatureAlgorithm {
    /// Classical signature, always present on a signed request.
    Ed25519,
    /// Post-quantum signature (ML-DSA-44), optional under the hybrid policy.
    MlDsa44,
}

impl WorkloadSignatureAlgorithm {
    /// Stable wire name of the algorithm.
    pub fn name(&self) -> &'static str {
        match self {
            WorkloadSignatureAlgorithm::Ed25519 => "ed25519",
            WorkloadSignatureAlgorithm::MlDsa44 => "ml-dsa-44",
        }
    }

    /// Header naming this algorithm, or `None` when the algorithm is carried
    /// by the fixed (unprefixed) classical header pair.
    ///
    /// The classical scheme is the legacy one and keeps its original header
    /// names; the post-quantum scheme is a named variant with suffixed
    /// headers. A verifier that only knows the classical pair therefore
    /// ignores the suffixed headers instead of failing.
    pub fn algorithm_header(&self) -> Option<&'static str> {
        match self {
            WorkloadSignatureAlgorithm::Ed25519 => None,
            WorkloadSignatureAlgorithm::MlDsa44 => Some(HEADER_PQC_ALGORITHM),
        }
    }
}

/// The signature material a request carries, split per algorithm.
///
/// The classical part is always populated for a signed request. The
/// post-quantum part is populated only when the client opts into the
/// hybrid scheme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadSignatureParts {
    /// Classical public key (hex).
    pub public_key_hex: String,
    /// Classical signature (hex).
    pub signature_hex: String,
    /// Signed timestamp (unix seconds, decimal string).
    pub timestamp_secs: String,
    /// Post-quantum public key (hex), absent when the client is classical only.
    #[serde(default)]
    pub pqc_public_key_hex: Option<String>,
    /// Post-quantum signature (hex), absent when the client is classical only.
    #[serde(default)]
    pub pqc_signature_hex: Option<String>,
    /// Post-quantum algorithm name, absent when the client is classical only.
    #[serde(default)]
    pub pqc_algorithm: Option<String>,
}

impl WorkloadSignatureParts {
    /// True when the request carries post-quantum material.
    pub fn has_pqc(&self) -> bool {
        self.pqc_public_key_hex.is_some() && self.pqc_signature_hex.is_some()
    }
}

/// What a verifier requires from an incoming request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadSignaturePolicy {
    /// Ed25519 is sufficient; a post-quantum signature is accepted and
    /// ignored. The policy of a kernel that has not migrated yet.
    ClassicalOnly,
    /// Both signatures are required and both must verify.
    RequireHybrid,
}

/// The verification outcome, with the reason spelled out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadSignatureDecision {
    /// The request is authenticated under the policy.
    Accept,
    /// The request carries no usable classical signature.
    MissingClassicalSignature,
    /// The policy requires a post-quantum signature and none is present.
    MissingPqcSignature,
    /// The request carries a post-quantum signature the policy cannot read
    /// (unknown algorithm name).
    UnsupportedPqcAlgorithm,
    /// The timestamp is outside the replay window.
    TimestampOutsideWindow,
    /// A signature did not verify against the payload.
    SignatureInvalid,
    /// The request carried a post-quantum signature that failed to verify
    /// while the classical one succeeded.
    PqcSignatureInvalid,
}

impl WorkloadSignatureDecision {
    /// True when the request may proceed.
    pub fn is_accept(&self) -> bool {
        matches!(self, WorkloadSignatureDecision::Accept)
    }
}

/// What a verifier observed about an incoming request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadSignatureObservation {
    /// The signature material extracted from the request.
    pub parts: WorkloadSignatureParts,
    /// The policy the verifier applies.
    pub policy: WorkloadSignaturePolicy,
    /// True when the classical signature verified.
    pub classical_verified: bool,
    /// True when a post-quantum signature was present and verified.
    pub pqc_verified: bool,
    /// The decision, with its reason.
    pub decision: WorkloadSignatureDecision,
}

/// Build the signed payload: `timestamp + method + "|" + path + "|" + body`.
///
/// This is the byte sequence every algorithm signs. Pure and identical for
/// every adapter and every algorithm, so a hybrid request signs exactly the
/// same bytes twice.
pub fn workload_signed_payload(
    timestamp_secs: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Vec<u8> {
    let mut payload = timestamp_secs.as_bytes().to_vec();
    payload.extend_from_slice(method.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(path.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(body);
    payload
}

/// Decide whether a timestamp is inside the replay window.
///
/// `now_unix_secs` is injected: the caller owns the clock. A timestamp in
/// the future is tolerated up to the drift bound; beyond it, the request is
/// refused because the sender's clock cannot be trusted.
pub fn timestamp_within_window(
    timestamp_secs: u64,
    now_unix_secs: u64,
    max_age_secs: u64,
    max_drift_secs: u64,
) -> bool {
    if timestamp_secs > now_unix_secs {
        return now_unix_secs + max_drift_secs >= timestamp_secs;
    }
    now_unix_secs - timestamp_secs <= max_age_secs
}

/// Pure predicate: what does the policy require of this request material?
///
/// Separates "is the material complete" from "did the signatures verify",
/// so a verifier can report a precise reason instead of a generic failure.
pub fn decide_workload_signature_requirements(
    parts: &WorkloadSignatureParts,
    policy: WorkloadSignaturePolicy,
) -> WorkloadSignatureDecision {
    if parts.public_key_hex.is_empty() || parts.signature_hex.is_empty() {
        return WorkloadSignatureDecision::MissingClassicalSignature;
    }
    match policy {
        WorkloadSignaturePolicy::ClassicalOnly => WorkloadSignatureDecision::Accept,
        WorkloadSignaturePolicy::RequireHybrid => {
            if !parts.has_pqc() {
                return WorkloadSignatureDecision::MissingPqcSignature;
            }
            match parts.pqc_algorithm.as_deref() {
                Some(name) if name == WorkloadSignatureAlgorithm::MlDsa44.name() => {
                    WorkloadSignatureDecision::Accept
                }
                _ => WorkloadSignatureDecision::UnsupportedPqcAlgorithm,
            }
        }
    }
}

/// Combine the requirement decision with the cryptographic outcomes into the
/// final decision.
///
/// The classical signature is mandatory under every policy. Under
/// `RequireHybrid` the post-quantum signature must also be present and
/// valid; a valid classical signature never rescues a broken
/// post-quantum one, because a hybrid request asserts both.
pub fn decide_workload_signature(
    parts: &WorkloadSignatureParts,
    policy: WorkloadSignaturePolicy,
    classical_verified: bool,
    pqc_verified: bool,
) -> WorkloadSignatureDecision {
    let requirement = decide_workload_signature_requirements(parts, policy);
    if !requirement.is_accept() {
        return requirement;
    }
    if !classical_verified {
        return WorkloadSignatureDecision::SignatureInvalid;
    }
    if matches!(policy, WorkloadSignaturePolicy::RequireHybrid) && !pqc_verified {
        return WorkloadSignatureDecision::PqcSignatureInvalid;
    }
    WorkloadSignatureDecision::Accept
}

/// Header names carrying a given algorithm's material.
pub fn workload_signature_headers(
    algorithm: WorkloadSignatureAlgorithm,
) -> (&'static str, &'static str) {
    match algorithm {
        WorkloadSignatureAlgorithm::Ed25519 => (HEADER_PUBLIC_KEY, HEADER_SIGNATURE),
        WorkloadSignatureAlgorithm::MlDsa44 => (HEADER_PQC_PUBLIC_KEY, HEADER_PQC_SIGNATURE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classical_parts() -> WorkloadSignatureParts {
        WorkloadSignatureParts {
            public_key_hex: "aabb".to_string(),
            signature_hex: "ccdd".to_string(),
            timestamp_secs: "1700000000".to_string(),
            pqc_public_key_hex: None,
            pqc_signature_hex: None,
            pqc_algorithm: None,
        }
    }

    fn hybrid_parts() -> WorkloadSignatureParts {
        WorkloadSignatureParts {
            pqc_public_key_hex: Some("eeff".to_string()),
            pqc_signature_hex: Some("1122".to_string()),
            pqc_algorithm: Some("ml-dsa-44".to_string()),
            ..classical_parts()
        }
    }

    #[test]
    fn payload_matches_kernel_wire_format() {
        assert_eq!(
            workload_signed_payload("12", "GET", "/node-control/node-a/desired-state", &[]),
            b"12GET|/node-control/node-a/desired-state|"
        );
    }

    #[test]
    fn ws_handshake_payload_format() {
        assert_eq!(
            workload_signed_payload("1700000000", "GET", "/events/ws", &[]),
            b"1700000000GET|/events/ws|"
        );
    }

    #[test]
    fn classical_only_policy_accepts_classical_material() {
        let decision = decide_workload_signature(
            &classical_parts(),
            WorkloadSignaturePolicy::ClassicalOnly,
            true,
            false,
        );
        assert_eq!(decision, WorkloadSignatureDecision::Accept);
    }

    #[test]
    fn hybrid_policy_refuses_classical_only_material() {
        let decision = decide_workload_signature(
            &classical_parts(),
            WorkloadSignaturePolicy::RequireHybrid,
            true,
            false,
        );
        assert_eq!(decision, WorkloadSignatureDecision::MissingPqcSignature);
    }

    #[test]
    fn hybrid_policy_accepts_verified_hybrid_material() {
        let decision = decide_workload_signature(
            &hybrid_parts(),
            WorkloadSignaturePolicy::RequireHybrid,
            true,
            true,
        );
        assert_eq!(decision, WorkloadSignatureDecision::Accept);
    }

    #[test]
    fn hybrid_policy_refuses_broken_pqc_even_with_valid_classical() {
        let decision = decide_workload_signature(
            &hybrid_parts(),
            WorkloadSignaturePolicy::RequireHybrid,
            true,
            false,
        );
        assert_eq!(decision, WorkloadSignatureDecision::PqcSignatureInvalid);
    }

    #[test]
    fn classical_only_policy_ignores_present_pqc_material() {
        let decision = decide_workload_signature(
            &hybrid_parts(),
            WorkloadSignaturePolicy::ClassicalOnly,
            true,
            false,
        );
        assert_eq!(decision, WorkloadSignatureDecision::Accept);
    }

    #[test]
    fn missing_classical_signature_is_refused() {
        let mut parts = classical_parts();
        parts.signature_hex.clear();
        let decision =
            decide_workload_signature(&parts, WorkloadSignaturePolicy::ClassicalOnly, false, false);
        assert_eq!(
            decision,
            WorkloadSignatureDecision::MissingClassicalSignature
        );
    }

    #[test]
    fn unknown_pqc_algorithm_is_refused_under_hybrid_policy() {
        let mut parts = hybrid_parts();
        parts.pqc_algorithm = Some("falcon-512".to_string());
        let decision =
            decide_workload_signature(&parts, WorkloadSignaturePolicy::RequireHybrid, true, true);
        assert_eq!(decision, WorkloadSignatureDecision::UnsupportedPqcAlgorithm);
    }

    #[test]
    fn timestamp_inside_window_is_accepted() {
        assert!(timestamp_within_window(1_000, 1_010, 300, 30));
    }

    #[test]
    fn timestamp_older_than_window_is_refused() {
        assert!(!timestamp_within_window(1_000, 1_400, 300, 30));
    }

    #[test]
    fn timestamp_beyond_drift_is_refused() {
        assert!(!timestamp_within_window(1_100, 1_000, 300, 30));
    }

    #[test]
    fn timestamp_within_drift_is_accepted() {
        assert!(timestamp_within_window(1_020, 1_000, 300, 30));
    }

    #[test]
    fn algorithm_names_are_stable() {
        assert_eq!(WorkloadSignatureAlgorithm::Ed25519.name(), "ed25519");
        assert_eq!(WorkloadSignatureAlgorithm::MlDsa44.name(), "ml-dsa-44");
    }

    #[test]
    fn post_quantum_algorithm_names_its_header() {
        assert_eq!(
            WorkloadSignatureAlgorithm::MlDsa44.algorithm_header(),
            Some(HEADER_PQC_ALGORITHM)
        );
        assert_eq!(WorkloadSignatureAlgorithm::Ed25519.algorithm_header(), None);
    }
}
