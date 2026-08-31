//! Pattern-model seal → PoH anchoring (option b, D1 follow-through).
//!
//! When a PatternModel is sealed, an event `pattern_gc.model.sealed` enters
//! the kernel proof-of-history chain carrying the model's digest. The
//! journal (L0) stays the authoritative cold store; the PoH row is the
//! cryptographic anchor binding the journal's local chain to the kernel's
//! global chain.
//!
//! Pure assembly here: the caller supplies a `ProofOfHistoryStorage` port.

use crate::digest::{SHA3_PREFIX, digest_bytes};

/// Anchor payload for one sealed pattern model.
#[derive(Debug, Clone)]
pub struct PatternModelSealed<'a> {
    pub model_id: &'a str,
    pub report_digest: &'a str,
    /// Journal segment + sequence where the evidence line lives.
    pub journal_segment: &'a str,
    pub journal_sequence: u64,
}

impl PatternModelSealed<'_> {
    /// Canonical anchor digest (constitutional SHA3-512 law): binds model
    /// identity to its journal location so neither can drift silently.
    pub fn anchor_digest(&self) -> String {
        let input = format!(
            "{}|{}|{}|{}",
            self.model_id, self.report_digest, self.journal_segment, self.journal_sequence
        );
        let full = digest_bytes(input.as_bytes());
        // Trim the prefix for compactness in payload; law compliance is
        // carried by the digest string itself when stored elsewhere.
        full.trim_start_matches(SHA3_PREFIX).to_string()
    }

    /// Build the append-ready journal entry fields for this seal.
    pub fn journal_event(&self) -> (&'static str, String) {
        (
            "pattern_gc.model.sealed",
            format!("sha3-512:{}", self.anchor_digest()),
        )
    }
}

/// Kernel-side Provable impl lives in domain/proof.rs consumers; this
/// function assembles the ProofOfHistory-compatible payload JSON that
/// `Provable::proof_payload()` expects for `pattern_gc.model.sealed`.
pub fn anchor_payload_json(model: &PatternModelSealed<'_>) -> serde_json::Value {
    serde_json::json!({
        "model_id": model.model_id,
        "report_digest": model.report_digest,
        "journal_segment": model.journal_segment,
        "journal_sequence": model.journal_sequence,
        "anchor_digest": model.anchor_digest(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_digest_is_deterministic_and_position_sensitive() {
        let a = PatternModelSealed {
            model_id: "m1",
            report_digest: "rep1",
            journal_segment: "evidence-202608.journal",
            journal_sequence: 7,
        };
        let b = PatternModelSealed {
            journal_sequence: 8,
            ..a.clone()
        };
        assert_eq!(a.anchor_digest(), a.anchor_digest());
        assert_ne!(a.anchor_digest(), b.anchor_digest());

        let moved = PatternModelSealed {
            journal_segment: "evidence-202609.journal",
            ..a.clone()
        };
        assert_ne!(a.anchor_digest(), moved.anchor_digest());
    }

    #[test]
    fn journal_event_pairs_type_with_prefixed_digest() {
        let a = PatternModelSealed {
            model_id: "m1",
            report_digest: "rep1",
            journal_segment: "seg",
            journal_sequence: 1,
        };
        let (event_type, digest) = a.journal_event();
        assert_eq!(event_type, "pattern_gc.model.sealed");
        assert!(digest.starts_with("sha3-512:"));
    }

    #[test]
    fn anchor_payload_carries_all_binding_fields() {
        let a = PatternModelSealed {
            model_id: "m1",
            report_digest: "rep1",
            journal_segment: "seg",
            journal_sequence: 42,
        };
        let payload = anchor_payload_json(&a);
        assert_eq!(payload["model_id"], "m1");
        assert_eq!(payload["journal_sequence"], 42);
        assert!(payload["anchor_digest"].as_str().unwrap().len() == 128);
    }
}
