//! Cold-evidence journal contract (D1 decision, storage-layering L0).
//!
//! The journal is the AUTHORITATIVE home of cold evidence: one JSONL line
//! per sealed event, hash-chained with the canonical SHA3-512 digest law,
//! signed by the node's kernel identity. Projections (SurrealDB tables,
//! side-car indexes) are derived and reconstructible by replay — never
//! authority.
//!
//! Pure contract only: types + chain validation. File I/O stays in
//! adapters (kernel/host), mirroring `coordination_journal.rs`.

use serde::{Deserialize, Serialize};

pub const EVIDENCE_JOURNAL_SCHEMA: &str = "cynic.evidence.journal_entry.v1";
/// Event type discriminator for a sealed cold-evidence entry.
///
/// The previous value was `pattern_gc.model.sealed`, whose only producer was
/// `pattern_gc::poh_anchor`, deleted with the rest of that module's dead half.
/// Nothing can emit that event any more, so a token no code produces is a token
/// that cannot prove the journal handles a real one.
pub const EVIDENCE_EVENT_ENTRY_SEALED: &str = "evidence.entry.sealed";

/// One sealed cold-evidence event.
///
/// `payload_digest` is a prefixed canonical digest (`sha3-512:`) over the
/// evidence content that lives elsewhere (PatternModel file, report file).
/// The journal carries proof OF the evidence, not the evidence itself —
/// keeping lines small and verification cheap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceJournalEntry {
    pub schema: String,
    /// Monotonic per-node sequence (1-based; 0 = invalid).
    pub sequence: u64,
    /// Node identity fingerprint (multi-node merge sorts by this then seq).
    pub node_id: String,
    /// Event type discriminator (`evidence.entry.sealed`, ...).
    pub event_type: String,
    /// Content digest of the referenced evidence (prefixed, canonical law).
    pub payload_digest: String,
    /// SHA3-512 chained digest: canonical(payload_digest|prev_hash|sequence).
    pub entry_hash: String,
    /// Previous entry's hash ("genesis" for the first line of a segment
    /// that explicitly continues nothing; segments carry prev across rolls).
    pub prev_hash: String,
    /// Unix seconds at seal time (monotonicity enforced by validators).
    pub sealed_at_unix_s: i64,
}

impl EvidenceJournalEntry {
    /// Canonical bytes hashed into `entry_hash`: fixed field order so the
    /// chain recomputation is byte-stable across implementations.
    pub fn chain_input(&self) -> Vec<u8> {
        (
            self.payload_digest.as_str(),
            self.prev_hash.as_str(),
            self.sequence,
            self.sealed_at_unix_s,
        )
            .serialize_bytes()
    }
}

/// Serde-free canonical serialization for hashing (field order frozen).
trait SerializeBytes {
    fn serialize_bytes(&self) -> Vec<u8>;
}

impl SerializeBytes for (&str, &str, u64, i64) {
    fn serialize_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.0.as_bytes());
        out.push(0x00);
        out.extend_from_slice(self.1.as_bytes());
        out.push(0x00);
        out.extend_from_slice(&self.2.to_le_bytes());
        out.extend_from_slice(&self.3.to_le_bytes());
        out
    }
}

/// Compute the canonical chained hash for an entry given its predecessor's
/// hash. Uses the constitutional internal digest (SHA3-512, prefixed).
pub fn compute_entry_hash(
    payload_digest: &str,
    prev_hash: &str,
    sequence: u64,
    sealed_at_unix_s: i64,
) -> String {
    let input = (payload_digest, prev_hash, sequence, sealed_at_unix_s).serialize_bytes();
    cynic_protocol::digest::digest_bytes(&input)
}

/// Per-entry structural validation (mirrors `validate_journal_entry`).
pub fn validate_evidence_entry(entry: &EvidenceJournalEntry) -> Vec<&'static str> {
    let mut violations = Vec::new();
    if entry.schema != EVIDENCE_JOURNAL_SCHEMA {
        violations.push("evidence_journal_schema_invalid");
    }
    if entry.sequence == 0 {
        violations.push("evidence_journal_sequence_missing");
    }
    if entry.node_id.trim().is_empty() {
        violations.push("evidence_journal_node_missing");
    }
    if entry.event_type.trim().is_empty() {
        violations.push("evidence_journal_event_type_missing");
    }
    if cynic_protocol::digest::validate_digest(&entry.payload_digest, true).is_err() {
        violations.push("evidence_journal_payload_digest_invalid");
    }
    if cynic_protocol::digest::validate_digest(&entry.entry_hash, false).is_err() {
        violations.push("evidence_journal_entry_hash_not_canonical");
    }
    violations
}

/// Full-chain validation over a segment: per-entry checks + hash-chain
/// continuity + monotonic sequences/timestamps + genesis/prev rules.
pub fn validate_evidence_chain(entries: &[EvidenceJournalEntry]) -> Vec<&'static str> {
    let mut violations = Vec::new();
    let mut prev_hash = String::from("genesis");
    let mut prev_sequence: u64 = 0;
    let mut prev_ts: i64 = i64::MIN;

    for entry in entries {
        violations.extend(validate_evidence_entry(entry));
        let expected = compute_entry_hash(
            &entry.payload_digest,
            &prev_hash,
            entry.sequence,
            entry.sealed_at_unix_s,
        );
        if entry.entry_hash != expected {
            violations.push("evidence_journal_chain_broken");
        }
        if entry.sequence <= prev_sequence {
            violations.push("evidence_journal_sequence_not_monotonic");
        }
        if entry.sealed_at_unix_s < prev_ts {
            violations.push("evidence_journal_timestamp_regressed");
        }
        if entry.prev_hash != prev_hash && prev_sequence != 0 {
            violations.push("evidence_journal_prev_hash_mismatch");
        }
        // The chain continues from the RECOMPUTED hash, not the stored one:
        // tampering with any entry then cascades through every later link
        // instead of isolating the damage to a single line.
        prev_hash = expected;
        prev_sequence = entry.sequence;
        prev_ts = entry.sealed_at_unix_s;
    }
    violations.sort_unstable();
    violations.dedup();
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(seq: u64, prev: &str, ts: i64) -> EvidenceJournalEntry {
        let payload = format!("sha3-512:{seq:04}");
        let hash = compute_entry_hash(&payload, prev, seq, ts);
        EvidenceJournalEntry {
            schema: EVIDENCE_JOURNAL_SCHEMA.to_string(),
            sequence: seq,
            node_id: "node-lab".to_string(),
            event_type: EVIDENCE_EVENT_ENTRY_SEALED.to_string(),
            payload_digest: payload,
            entry_hash: hash,
            prev_hash: prev.to_string(),
            sealed_at_unix_s: ts,
        }
    }

    #[test]
    fn valid_chain_passes_all_checks() {
        let e1 = make_entry(1, "genesis", 1000);
        let e2 = make_entry(2, &e1.entry_hash.clone(), 2000);
        let e3 = make_entry(3, &e2.entry_hash.clone(), 3000);
        assert!(validate_evidence_chain(&[e1, e2, e3]).is_empty());
    }

    #[test]
    fn single_byte_corruption_breaks_the_chain() {
        // The core property: tampering with any historical field makes every
        // subsequent hash mismatch — post-mortem detection without a server.
        let e1 = make_entry(1, "genesis", 1000);
        let e2 = make_entry(2, &e1.entry_hash.clone(), 2000);
        let mut corrupted = e1.clone();
        corrupted.sealed_at_unix_s = 1500; // backdate attempt
        corrupted.prev_hash = "genesis".to_string();
        corrupted.entry_hash = compute_entry_hash(
            &corrupted.payload_digest,
            &corrupted.prev_hash,
            corrupted.sequence,
            corrupted.sealed_at_unix_s,
        );
        // Even re-signed consistently, the NEXT link breaks:
        let violations = validate_evidence_chain(&[corrupted, e2]);
        assert!(violations.contains(&"evidence_journal_prev_hash_mismatch"));
        assert!(violations.contains(&"evidence_journal_chain_broken"));
    }

    #[test]
    fn unprefixed_payload_digest_is_rejected() {
        let mut e = make_entry(1, "genesis", 1000);
        e.payload_digest = "deadbeef".to_string(); // unprefixed = invalid by law
        let violations = validate_evidence_chain(&[e]);
        assert!(violations.contains(&"evidence_journal_payload_digest_invalid"));
    }

    #[test]
    fn non_canonical_entry_hash_is_rejected() {
        let mut e = make_entry(1, "genesis", 1000);
        e.entry_hash = format!("sha256:{}", "ab".repeat(32)); // legacy algo not allowed as chain hash
        let violations = validate_evidence_chain(&[e]);
        assert!(violations.contains(&"evidence_journal_entry_hash_not_canonical"));
    }

    #[test]
    fn sequence_and_time_monotonicity_enforced() {
        let e1 = make_entry(1, "genesis", 2000);
        let e2 = make_entry(1, &e1.entry_hash.clone(), 1000); // same seq, older ts
        let violations = validate_evidence_chain(&[e1, e2]);
        assert!(violations.contains(&"evidence_journal_sequence_not_monotonic"));
        assert!(violations.contains(&"evidence_journal_timestamp_regressed"));
    }

    #[test]
    fn schema_drift_is_caught() {
        let mut e = make_entry(1, "genesis", 1000);
        e.schema = "cynic.evidence.journal_entry.v9".to_string();
        assert!(validate_evidence_chain(&[e]).contains(&"evidence_journal_schema_invalid"));
    }

    #[test]
    fn jsonl_roundtrip_preserves_validation() {
        let e1 = make_entry(1, "genesis", 1000);
        let e2 = make_entry(2, &e1.entry_hash.clone(), 2000);
        let line = serde_json::to_string(&e1).unwrap();
        let parsed: EvidenceJournalEntry = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed, e1);
        assert!(validate_evidence_chain(&[parsed, e2]).is_empty());
    }
}
