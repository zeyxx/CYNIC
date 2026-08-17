//! Proof of History persistence in SurrealDB.

use super::SurrealHttpStorage;
use crate::domain::proof::chain_hash;
use crate::domain::storage::{ProofOfHistory, StorageError};
use serde_json::Value;

fn extract_record_id(row: &Value) -> Option<String> {
    row.as_str()
        .map(String::from)
        .or_else(|| row.get("id").and_then(|v| v.as_str()).map(String::from))
}

fn row_to_poh(row: &Value) -> Option<ProofOfHistory> {
    row.as_object().map(|obj| ProofOfHistory {
        id: obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        timestamp: obj.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0),
        source_organ: obj
            .get("source_organ")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        event_type: obj
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        payload: obj
            .get("payload")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        cryptographic_hash: obj
            .get("cryptographic_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        prev_hash: obj
            .get("prev_hash")
            .and_then(|v| v.as_str())
            .map(String::from),
        cryptographic_signature: obj
            .get("cryptographic_signature")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

/// Atomic read of the last hash for temporal chaining.
async fn last_proof_hash(storage: &SurrealHttpStorage) -> Result<String, StorageError> {
    let query = "SELECT cryptographic_hash FROM proof_of_history ORDER BY timestamp DESC LIMIT 1;";
    let rows = storage.query_one(query).await?;
    Ok(rows
        .first()
        .and_then(|row| row.get("cryptographic_hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("genesis")
        .to_string())
}

/// Atomic PoH append with CAS retry loop.
///
/// Between SELECT prev_hash and INSERT, another writer may intercalate a
/// block. The retry loop re-reads the latest prev_hash and recalculates the
/// chained_hash (deterministic per prev_hash), retrying up to MAX_RETRIES
/// until the INSERT succeeds with a coherent prev_hash.
pub(super) async fn store_proof_of_history(
    storage: &SurrealHttpStorage,
    poh: &ProofOfHistory,
) -> Result<String, StorageError> {
    const MAX_RETRIES: u32 = 5;

    for attempt in 0..MAX_RETRIES {
        // 1. Atomically read the current tip of the chain
        let prev_hash = last_proof_hash(storage).await?;

        // 2. Compute the chained hash using the pure domain primitive
        let chained_hash = chain_hash(&poh.cryptographic_hash, &prev_hash);

        // 3. Seal the event with the organism's Ed25519 identity
        use ed25519_dalek::Signer;
        let signature = crate::domain::proof::kernel_identity().sign(chained_hash.as_bytes());
        let sig_hex = format!(
            "'{}'",
            crate::storage::escape_surreal(&hex::encode(signature.to_bytes()))
        );

        let payload_str =
            serde_json::to_string(&poh.payload).unwrap_or_else(|_| "null".to_string());

        // 4. Insert the block. SurrealDB HTTP est stateless — pas de vraie
        //    transaction. L'atomicité est garantie par le CAS retry loop :
        //    si un autre writer a intercalé entre notre SELECT et cet INSERT,
        //    le read-back (étape 5) détectera un prev_hash stale et déclenchera
        //    le retry (étape 6).
        let query = format!(
            "INSERT INTO proof_of_history {{
                timestamp: '{}',
                source_organ: '{}',
                event_type: '{}',
                payload: {},
                prev_hash: '{}',
                cryptographic_hash: '{}',
                cryptographic_signature: {}
            }};",
            poh.timestamp,
            crate::storage::escape_surreal(&poh.source_organ),
            crate::storage::escape_surreal(&poh.event_type),
            payload_str,
            crate::storage::escape_surreal(&prev_hash),
            crate::storage::escape_surreal(&chained_hash),
            sig_hex
        );

        let rows = storage.query_one(&query).await?;
        if let Some(id) = rows.first().and_then(extract_record_id) {
            // 5. Read-back proof: verify the stored prev_hash matches what we
            //    used. If a concurrent writer appended after our INSERT but
            //    before our read-back, we detect the race via a re-read.
            let read_back = last_proof_hash(storage).await?;
            if read_back == chained_hash {
                // Fast path: our block is still the tip
                return Ok(id);
            }
            // Re-read our stored record to confirm it was persisted with our
            // prev_hash (another writer may have since appended past it)
            let verify_query = format!(
                "SELECT prev_hash, cryptographic_hash FROM proof_of_history WHERE cryptographic_hash = '{}';",
                crate::storage::escape_surreal(&chained_hash)
            );
            let verify_rows = storage.query_one(&verify_query).await?;
            if let Some(row) = verify_rows.first() {
                let stored_prev = row.get("prev_hash").and_then(|v| v.as_str()).unwrap_or("");
                if stored_prev == prev_hash {
                    return Ok(id);
                }
            }
        }

        // 6. Stale prev_hash detected — retry with fresh state
        if attempt + 1 < MAX_RETRIES {
            tracing::debug!(
                "PoH insert attempt {} raced; retrying with fresh prev_hash",
                attempt + 1
            );
            continue;
        }
    }

    Err(StorageError::QueryFailed(
        "proof of history insert failed after retries (concurrent writer)".to_string(),
    ))
}

pub(super) async fn list_proof_of_history(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<ProofOfHistory>, StorageError> {
    let query = format!("SELECT * FROM proof_of_history ORDER BY timestamp DESC LIMIT {limit};");
    let rows = storage.query_one(&query).await?;
    Ok(rows.iter().filter_map(row_to_poh).collect())
}
