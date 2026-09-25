//! Evidence journal → SurrealDB projection replay (D1, layer L0).
//!
//! The journal is authority; this module defines the derived projection:
//! an idempotent replay of journal entries into a queryable table.
//!
//! Idempotency law: replaying the same entry twice produces the same
//! single row (keyed by `event_id` = node_id + sequence). Rebuilding the
//! projection from scratch is ALWAYS safe — delete + full replay yields
//! identical state. The projection is never authority: its corruption is
//! repaired by re-running the replay, not by editing rows.

use serde::{Deserialize, Serialize};

use crate::evidence_journal::EvidenceJournalEntry;

pub const EVIDENCE_PROJECTION_SCHEMA: &str = "cynic.evidence.projection_row.v1";

/// One projected row — what the SurrealDB table stores per journal entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceProjectionRow {
    pub schema: String,
    /// Stable identity: `{node_id}:{sequence}` — replay key.
    pub event_id: String,
    pub node_id: String,
    pub sequence: u64,
    pub event_type: String,
    /// Prefixed canonical digest of the anchored evidence content.
    pub payload_digest: String,
    pub entry_hash: String,
    pub sealed_at_unix_s: i64,
}

/// Compute the deterministic projection row for a journal entry.
/// Pure: same entry always yields the same row.
pub fn project_entry(entry: &EvidenceJournalEntry) -> EvidenceProjectionRow {
    EvidenceProjectionRow {
        schema: EVIDENCE_PROJECTION_SCHEMA.to_string(),
        event_id: format!("{}:{}", entry.node_id, entry.sequence),
        node_id: entry.node_id.clone(),
        sequence: entry.sequence,
        event_type: entry.event_type.clone(),
        payload_digest: entry.payload_digest.clone(),
        entry_hash: entry.entry_hash.clone(),
        sealed_at_unix_s: entry.sealed_at_unix_s,
    }
}

/// Project a whole verified chain, preserving order. Callers must run
/// `validate_evidence_chain` FIRST and refuse to project a broken chain —
/// a corrupted journal must surface as verification failure, not as
/// silently projected corrupt rows.
pub fn project_chain(entries: &[EvidenceJournalEntry]) -> Vec<EvidenceProjectionRow> {
    entries.iter().map(project_entry).collect()
}

/// Replay plan: which entries to upsert and which stale rows to remove.
///
/// Derived projections can drift from the journal when segments roll or
/// entries are re-verified; the diff keeps the repair story explicit and
/// bounded instead of "delete everything" on every sync.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplayPlan {
    /// Rows to upsert (idempotent by event_id).
    pub upserts: Vec<EvidenceProjectionRow>,
    /// event_ids present in the projection but absent from the journal —
    /// evidence of drift or of a pruned segment; removal is the caller's
    /// decision (the journal remains authority either way).
    pub stale_event_ids: Vec<String>,
}

/// Diff a set of journal entries against existing projection keys.
pub fn replay_plan(entries: &[EvidenceJournalEntry], existing_event_ids: &[String]) -> ReplayPlan {
    let wanted: Vec<EvidenceProjectionRow> = entries.iter().map(project_entry).collect();
    let wanted_ids: std::collections::BTreeSet<&str> =
        wanted.iter().map(|r| r.event_id.as_str()).collect();

    let upserts = wanted
        .iter()
        .filter(|row| !existing_event_ids.contains(&row.event_id))
        .cloned()
        .collect();

    let stale_event_ids: Vec<String> = existing_event_ids
        .iter()
        .filter(|id| !wanted_ids.contains(id.as_str()))
        .cloned()
        .collect();

    ReplayPlan {
        upserts,
        stale_event_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_journal::{EVIDENCE_EVENT_MODEL_SEALED, compute_entry_hash};

    fn make_entry(node: &str, seq: u64, prev: &str, ts: i64) -> EvidenceJournalEntry {
        let payload = format!("sha3-512:{node}-{seq}");
        EvidenceJournalEntry {
            schema: crate::evidence_journal::EVIDENCE_JOURNAL_SCHEMA.to_string(),
            sequence: seq,
            node_id: node.to_string(),
            event_type: EVIDENCE_EVENT_MODEL_SEALED.to_string(),
            entry_hash: compute_entry_hash(&payload, prev, seq, ts),
            payload_digest: payload,
            prev_hash: prev.to_string(),
            sealed_at_unix_s: ts,
        }
    }

    #[test]
    fn projection_is_deterministic_and_keyed() {
        let e = make_entry("node-a", 3, "genesis", 1000);
        let r1 = project_entry(&e);
        let r2 = project_entry(&e);
        assert_eq!(r1, r2);
        assert_eq!(r1.event_id, "node-a:3");
        assert_eq!(r1.schema, EVIDENCE_PROJECTION_SCHEMA);
        assert_eq!(r1.payload_digest, e.payload_digest);
    }

    #[test]
    fn chain_projection_preserves_order_and_count() {
        let chain = vec![
            make_entry("n", 1, "genesis", 100),
            make_entry("n", 2, "genesis", 200),
            make_entry("n", 3, "genesis", 300),
        ];
        // Note: hashes won't chain here (prev="genesis" for all), but
        // projection doesn't validate — validation is a separate, mandatory
        // step before projection.
        let rows = project_chain(&chain);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].event_id, "n:1");
        assert_eq!(rows[2].event_id, "n:3");
    }

    #[test]
    fn replay_plan_detects_upserts_and_stale_rows() {
        let chain = vec![
            make_entry("n", 1, "genesis", 100),
            make_entry("n", 2, "genesis", 200),
        ];
        // Projection already has n:1 AND a stale n:9.
        let existing = vec!["n:1".to_string(), "n:9".to_string()];
        let plan = replay_plan(&chain, &existing);
        assert_eq!(plan.upserts.len(), 1);
        assert_eq!(plan.upserts[0].event_id, "n:2");
        assert_eq!(plan.stale_event_ids, vec!["n:9".to_string()]);
    }

    #[test]
    fn replay_plan_empty_when_projection_fresh() {
        let chain = vec![make_entry("n", 1, "genesis", 100)];
        let plan = replay_plan(&chain, &[]);
        assert_eq!(plan.upserts.len(), 1);
        assert!(plan.stale_event_ids.is_empty());
    }

    #[test]
    fn json_roundtrip_of_row() {
        let row = project_entry(&make_entry("n", 7, "genesis", 500));
        let json = serde_json::to_string(&row).unwrap();
        let back: EvidenceProjectionRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back, row);
    }
}
