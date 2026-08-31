//! Evidence projection storage port + implementations.
//!
//! The queryable projection of the cold-evidence journal (D1 hybrid):
//! the journal is authority, this table is a derived view rebuilt by
//! replay. All writes are idempotent upserts keyed by `event_id`
//! (`node_id:sequence`) so replaying the same journal twice is safe.

use crate::domain::storage::{ProofOfHistory, StorageError};
use cynic_protocol::evidence_projection::EvidenceProjectionRow;
use sha3::Digest;

pub const EVIDENCE_PROJECTION_TABLE: &str = "evidence_projection";

/// Build the ProofOfHistory anchor for one projected row (kernel-chain
/// binding). Shared by writers so the payload shape has one source.
pub fn projection_anchor(row: &EvidenceProjectionRow) -> ProofOfHistory {
    let mut hasher = sha3::Sha3_512::new();
    sha3::Digest::update(&mut hasher, b"evidence_projected|");
    sha3::Digest::update(&mut hasher, row.event_id.as_bytes());
    sha3::Digest::update(&mut hasher, b"|");
    sha3::Digest::update(&mut hasher, row.entry_hash.as_bytes());
    ProofOfHistory {
        id: String::new(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        source_organ: "cynic-kernel".to_string(),
        event_type: "evidence.projection.replayed".to_string(),
        payload: serde_json::json!({
            "event_id": row.event_id,
            "entry_hash": row.entry_hash,
            "payload_digest": row.payload_digest,
            "table": EVIDENCE_PROJECTION_TABLE,
        }),
        cryptographic_hash: hex::encode(sha3::Digest::finalize(hasher)),
        prev_hash: None,
        cryptographic_signature: None,
    }
}

/// Upsert one projected row into SurrealDB. Idempotent by event_id:
/// replaying the same row twice leaves identical state.
#[allow(dead_code)]
pub(super) async fn upsert_projection_row(
    storage: &crate::storage::SurrealHttpStorage,
    row: &EvidenceProjectionRow,
) -> Result<(), StorageError> {
    let id_key = crate::storage::sanitize_id(&row.event_id).map_err(|e| {
        StorageError::QueryFailed(format!("invalid event_id {}: {e}", row.event_id))
    })?;
    let sql = format!(
        "UPSERT evidence_projection:`{id_key}` SET \
            schema = '{schema}', \
            event_id = '{event_id}', \
            node_id = '{node_id}', \
            sequence = {sequence}, \
            event_type = '{event_type}', \
            payload_digest = '{payload_digest}', \
            entry_hash = '{entry_hash}', \
            sealed_at_unix_s = {sealed_at};",
        schema = crate::storage::escape_surreal(&row.schema),
        event_id = crate::storage::escape_surreal(&row.event_id),
        node_id = crate::storage::escape_surreal(&row.node_id),
        sequence = row.sequence,
        event_type = crate::storage::escape_surreal(&row.event_type),
        payload_digest = crate::storage::escape_surreal(&row.payload_digest),
        entry_hash = crate::storage::escape_surreal(&row.entry_hash),
        sealed_at = row.sealed_at_unix_s,
    );
    storage
        .query_one(&sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("upsert evidence_projection: {e}")))?;
    Ok(())
}

/// List existing event_ids in the projection (for `replay_plan` diffing).
#[allow(dead_code)]
pub(super) async fn list_projected_event_ids(
    storage: &crate::storage::SurrealHttpStorage,
) -> Result<Vec<String>, StorageError> {
    let rows = storage
        .query_one("SELECT event_id FROM evidence_projection;")
        .await
        .map_err(|e| StorageError::QueryFailed(format!("list evidence_projection ids: {e}")))?;
    Ok(rows
        .iter()
        .filter_map(|r| r.get("event_id").and_then(|v| v.as_str()).map(String::from))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row(event_id: &str) -> EvidenceProjectionRow {
        EvidenceProjectionRow {
            schema: "cynic.evidence.projection_row.v1".to_string(),
            event_id: event_id.to_string(),
            node_id: "node-a".to_string(),
            sequence: 1,
            event_type: "pattern_gc.model.sealed".to_string(),
            payload_digest: format!("sha3-512:{event_id}"),
            entry_hash: format!("sha3-512:hash-{event_id}"),
            sealed_at_unix_s: 1_700_000_000,
        }
    }

    #[test]
    fn anchor_binds_event_id_to_entry_hash() {
        let row = sample_row("node-a:7");
        let anchor = projection_anchor(&row);
        assert_eq!(anchor.event_type, "evidence.projection.replayed");
        assert_eq!(anchor.payload["event_id"], "node-a:7");
        assert_eq!(anchor.payload["entry_hash"], "sha3-512:hash-node-a:7");
        assert_eq!(anchor.cryptographic_hash.len(), 128); // SHA3-512 hex

        // Different rows produce different anchors.
        let other = projection_anchor(&sample_row("node-a:8"));
        assert_ne!(anchor.cryptographic_hash, other.cryptographic_hash);
    }

    #[test]
    fn table_name_is_stable() {
        assert_eq!(EVIDENCE_PROJECTION_TABLE, "evidence_projection");
    }
}
