//! Proof of History persistence in SurrealDB.

use super::SurrealHttpStorage;
use crate::domain::storage::{ProofOfHistory, StorageError};
use serde_json::Value;
use sha3::Digest;

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

pub(super) async fn store_proof_of_history(
    storage: &SurrealHttpStorage,
    poh: &ProofOfHistory,
) -> Result<String, StorageError> {
    // 1. Fetch the last hash for temporal chaining
    let last_query =
        "SELECT cryptographic_hash FROM proof_of_history ORDER BY timestamp DESC LIMIT 1;";
    let last_rows = storage.query_one(last_query).await?;
    let prev_hash = last_rows
        .first()
        .and_then(|row| row.get("cryptographic_hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("genesis")
        .to_string();

    // 2. Compute the true chained hash: SHA3-512(payload_hash + prev_hash)
    // The incoming `poh.cryptographic_hash` is the payload's isolated hash.
    let mut hasher = sha3::Sha3_512::new();
    sha3::Digest::update(&mut hasher, poh.cryptographic_hash.as_bytes());
    sha3::Digest::update(&mut hasher, b"|");
    sha3::Digest::update(&mut hasher, prev_hash.as_bytes());
    let chained_hash = hex::encode(sha3::Digest::finalize(hasher));

    // 3. Seal the event with the organism's Ed25519 identity
    use ed25519_dalek::Signer;
    let signature = crate::domain::proof::kernel_identity().sign(chained_hash.as_bytes());
    let sig_hex = format!(
        "'{}'",
        crate::storage::escape_surreal(&hex::encode(signature.to_bytes()))
    );

    let payload_str = serde_json::to_string(&poh.payload).unwrap_or_else(|_| "null".to_string());

    let query = format!(
        "INSERT INTO proof_of_history {{ timestamp: '{}', source_organ: '{}', event_type: '{}', payload: {}, prev_hash: '{}', cryptographic_hash: '{}', cryptographic_signature: {} }} RETURN id;",
        poh.timestamp,
        crate::storage::escape_surreal(&poh.source_organ),
        crate::storage::escape_surreal(&poh.event_type),
        payload_str,
        crate::storage::escape_surreal(&prev_hash),
        crate::storage::escape_surreal(&chained_hash),
        sig_hex
    );

    let rows = storage.query_one(&query).await?;
    rows.first().and_then(extract_record_id).ok_or_else(|| {
        StorageError::QueryFailed("proof of history insert returned no id".to_string())
    })
}

pub(super) async fn list_proof_of_history(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<ProofOfHistory>, StorageError> {
    let query = format!("SELECT * FROM proof_of_history ORDER BY timestamp DESC LIMIT {limit};");
    let rows = storage.query_one(&query).await?;
    Ok(rows.iter().filter_map(row_to_poh).collect())
}
