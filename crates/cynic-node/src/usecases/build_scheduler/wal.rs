//! Unified WAL emission for scheduler settlement transitions.
//!
//! Every settled build appends exactly one [`SettlementWalEntry`] to the
//! hash-chained WAL. The entry uses the unified `cynic_protocol::hash_chain`
//! primitive (SHA3-512) and follows the evidence-journal pattern from T10.
//!
//! Failure to append fails the settlement loudly — never silent.

use cynic_protocol::hash_chain::{ChainLink, HashChain};
use serde::{Deserialize, Serialize};

/// Schema identifier for scheduler settlement WAL entries.
pub const SETTLEMENT_WAL_SCHEMA: &str = "cynic.scheduler.settlement_wal.v1";

/// Genesis prev-hash for the settlement WAL chain.
pub const SETTLEMENT_WAL_GENESIS: &str = "genesis";

/// Event type for a settled build transition.
pub const SETTLEMENT_WAL_EVENT_SETTLED: &str = "scheduler.build.settled";

/// One immutable WAL entry for a settled build.
///
/// Hash-chained via [`ChainLink`] using the unified SHA3-512 digest law.
/// The `workload_digest` field carries the `final_record_digest` from the
/// [`SettledParcelReceipt`](cynic_workload::SettledParcelReceipt), making
/// the WAL entry independently matchable to the settlement receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementWalEntry {
    pub schema: String,
    /// Monotonic per-node sequence (1-based; 0 = invalid).
    pub sequence: u64,
    /// Node identity that sealed this entry.
    pub node_id: String,
    /// Workload that was settled.
    pub workload_id: String,
    /// Completion status string (`completed`, `blocked`, `failed`, `degraded`).
    pub status: String,
    /// `final_record_digest` from the settlement receipt (prefixed digest).
    pub workload_digest: String,
    /// Evidence references returned at settlement time.
    pub evidence_refs: Vec<String>,
    /// Unix milliseconds at seal time.
    pub settled_at_ms: u64,
    /// Previous entry's hash ("genesis" for the first entry).
    pub prev_hash: String,
    /// This entry's SHA3-512 hash (prefixed, canonical law).
    pub entry_hash: String,
}

impl ChainLink for SettlementWalEntry {
    fn chain_input(&self) -> Vec<u8> {
        // Canonical serialization for hashing (field order frozen).
        let mut out = Vec::new();
        out.extend_from_slice(self.prev_hash.as_bytes());
        out.push(0x00);
        out.extend_from_slice(&self.sequence.to_le_bytes());
        out.push(0x00);
        out.extend_from_slice(self.workload_id.as_bytes());
        out.push(0x00);
        out.extend_from_slice(self.status.as_bytes());
        out.push(0x00);
        out.extend_from_slice(self.workload_digest.as_bytes());
        out.push(0x00);
        out.extend_from_slice(&self.settled_at_ms.to_le_bytes());
        out
    }

    fn entry_hash(&self) -> &str {
        &self.entry_hash
    }

    fn prev_hash(&self) -> &str {
        &self.prev_hash
    }
}

impl SettlementWalEntry {
    /// Seal a new WAL entry given the predecessor's hash.
    ///
    /// Computes the canonical SHA3-512 entry hash covering the link's
    /// full [`ChainLink::chain_input`].
    pub fn seal(
        sequence: u64,
        node_id: String,
        workload_id: String,
        status: String,
        workload_digest: String,
        evidence_refs: Vec<String>,
        settled_at_ms: u64,
        prev_hash: &str,
    ) -> Self {
        let mut entry = Self {
            schema: SETTLEMENT_WAL_SCHEMA.to_string(),
            sequence,
            node_id,
            workload_id,
            status,
            workload_digest,
            evidence_refs,
            settled_at_ms,
            prev_hash: prev_hash.to_string(),
            entry_hash: String::new(),
        };
        entry.entry_hash = HashChain::seal(&entry);
        entry
    }

    /// Verify whether this entry's hash matches its contents and previous link.
    pub fn verify_integrity(&self, expected_prev_hash: &str) -> bool {
        if self.prev_hash != expected_prev_hash {
            return false;
        }
        let expected_hash = HashChain::seal(self);
        self.entry_hash == expected_hash
    }
}

/// Stateful emitter that maintains the settlement WAL hash chain.
///
/// Each call to [`Self::seal_entry`] produces exactly one new entry,
/// chaining from the previous entry's hash (or "genesis" for the first).
#[derive(Debug, Clone)]
pub struct SettlementWalEmitter {
    next_sequence: u64,
    last_hash: String,
}

impl SettlementWalEmitter {
    /// Create a new emitter starting from genesis.
    pub fn new() -> Self {
        Self {
            next_sequence: 1,
            last_hash: SETTLEMENT_WAL_GENESIS.to_string(),
        }
    }

    /// Seal a new WAL entry and advance the chain state.
    pub fn seal_entry(
        &mut self,
        node_id: String,
        workload_id: String,
        status: String,
        workload_digest: String,
        evidence_refs: Vec<String>,
        settled_at_ms: u64,
    ) -> SettlementWalEntry {
        let entry = SettlementWalEntry::seal(
            self.next_sequence,
            node_id,
            workload_id,
            status,
            workload_digest,
            evidence_refs,
            settled_at_ms,
            &self.last_hash,
        );
        self.next_sequence += 1;
        self.last_hash = entry.entry_hash.clone();
        entry
    }

    /// The last entry's hash (or "genesis" if no entries sealed yet).
    pub fn last_hash(&self) -> &str {
        &self.last_hash
    }

    /// The next sequence number to be assigned.
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }
}

impl Default for SettlementWalEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TDD Step 1: Failing tests ──────────────────────────────────────

    /// A settled build must produce exactly one WAL entry with a matching
    /// workload digest. The entry must verify against the genesis prev-hash.
    #[test]
    fn settled_build_produces_wal_entry_with_matching_digest() {
        let mut emitter = SettlementWalEmitter::new();

        let entry = emitter.seal_entry(
            "node-1".to_string(),
            "workload-abc".to_string(),
            "completed".to_string(),
            "sha3-512:deadbeef".to_string(),
            vec!["scheduler-settlement:cafe".to_string()],
            1_700_000_000_000,
        );

        // Exactly one entry: sequence 1, correct workload_id and digest.
        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.workload_id, "workload-abc");
        assert_eq!(entry.workload_digest, "sha3-512:deadbeef");
        assert_eq!(entry.prev_hash, SETTLEMENT_WAL_GENESIS);
        assert!(
            entry.entry_hash.starts_with("sha3-512:"),
            "entry hash must use canonical SHA3-512 prefix"
        );
        assert!(
            entry.verify_integrity(SETTLEMENT_WAL_GENESIS),
            "entry must verify against genesis"
        );
    }

    /// Two consecutive settlements must form a valid hash chain:
    /// the second entry's prev_hash must equal the first entry's entry_hash.
    #[test]
    fn consecutive_entries_form_valid_hash_chain() {
        let mut emitter = SettlementWalEmitter::new();

        let entry1 = emitter.seal_entry(
            "node-1".to_string(),
            "workload-1".to_string(),
            "completed".to_string(),
            "sha3-512:aaa".to_string(),
            vec![],
            1_000,
        );
        let entry2 = emitter.seal_entry(
            "node-1".to_string(),
            "workload-2".to_string(),
            "failed".to_string(),
            "sha3-512:bbb".to_string(),
            vec![],
            2_000,
        );

        assert_eq!(entry1.sequence, 1);
        assert_eq!(entry2.sequence, 2);
        assert_eq!(entry2.prev_hash, entry1.entry_hash);

        // The full chain must verify from genesis.
        assert!(HashChain::verify(SETTLEMENT_WAL_GENESIS, &[entry1, entry2]).is_ok());
    }

    /// Tampering with any field after sealing must break verification.
    #[test]
    fn tampered_entry_fails_verification() {
        let mut entry = SettlementWalEntry::seal(
            1,
            "node-1".to_string(),
            "workload-123".to_string(),
            "completed".to_string(),
            "sha3-512:abc".to_string(),
            vec![],
            1_700_000_000_000,
            SETTLEMENT_WAL_GENESIS,
        );

        // Tamper with the workload digest after sealing.
        entry.workload_digest = "sha3-512:tampered".to_string();
        assert!(
            !entry.verify_integrity(SETTLEMENT_WAL_GENESIS),
            "tampered entry must fail verification"
        );
    }

    /// Tampering with prev_hash must break verification.
    #[test]
    fn tampered_prev_hash_fails_verification() {
        let mut entry = SettlementWalEntry::seal(
            1,
            "node-1".to_string(),
            "workload-123".to_string(),
            "completed".to_string(),
            "sha3-512:abc".to_string(),
            vec![],
            1_700_000_000_000,
            SETTLEMENT_WAL_GENESIS,
        );

        entry.prev_hash = "sha3-512:wrong".to_string();
        assert!(
            !entry.verify_integrity(SETTLEMENT_WAL_GENESIS),
            "tampered prev_hash must fail verification"
        );
    }

    /// The emitter must track sequence and last_hash correctly.
    #[test]
    fn emitter_tracks_sequence_and_last_hash() {
        let mut emitter = SettlementWalEmitter::new();
        assert_eq!(emitter.next_sequence(), 1);
        assert_eq!(emitter.last_hash(), SETTLEMENT_WAL_GENESIS);

        let entry = emitter.seal_entry(
            "node-1".to_string(),
            "wl-1".to_string(),
            "completed".to_string(),
            "sha3-512:x".to_string(),
            vec![],
            100,
        );
        assert_eq!(emitter.next_sequence(), 2);
        assert_eq!(emitter.last_hash(), entry.entry_hash);
    }

    /// JSON roundtrip must preserve all fields (for persistence).
    #[test]
    fn json_roundtrip_preserves_all_fields() {
        let entry = SettlementWalEntry::seal(
            42,
            "node-42".to_string(),
            "workload-42".to_string(),
            "degraded".to_string(),
            "sha3-512:42".to_string(),
            vec!["ref:1".to_string(), "ref:2".to_string()],
            42_000,
            "sha3-512:prev",
        );

        let json = serde_json::to_string(&entry).expect("serialize");
        let parsed: SettlementWalEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, entry);
        assert!(parsed.verify_integrity("sha3-512:prev"));
    }

    /// The entry schema must match the frozen constant.
    #[test]
    fn entry_schema_matches_frozen_constant() {
        let entry = SettlementWalEntry::seal(
            1,
            "n".to_string(),
            "w".to_string(),
            "completed".to_string(),
            "sha3-512:d".to_string(),
            vec![],
            0,
            SETTLEMENT_WAL_GENESIS,
        );
        assert_eq!(entry.schema, SETTLEMENT_WAL_SCHEMA);
    }
}
