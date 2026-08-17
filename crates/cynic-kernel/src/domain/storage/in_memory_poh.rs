//! InMemory PoH storage for atomic-chain tests without a live SurrealDB.
//! Reproduces the CAS retry-loop semantics of
//! `SurrealHttpStorage::store_proof_of_history` so the read-back proof of
//! chain coherence can be exercised in unit tests.

use super::{ProofOfHistory, StorageError};
use std::sync::Mutex;

use crate::domain::proof::chain_hash;

#[derive(Debug, Default)]
pub struct InMemoryPoHStorage {
    /// Chain entries, guarded by a Mutex to serialize appends.
    pub chain: Mutex<Vec<PoHEntry>>,
}

#[derive(Debug, Clone)]
pub struct PoHEntry {
    pub record_id: String,
    pub chained_hash: String,
    pub prev_hash: String,
    pub payload_hash: String,
}

impl InMemoryPoHStorage {
    pub fn new() -> Self {
        Self {
            chain: Mutex::new(Vec::new()),
        }
    }

    /// Last cryptographic_hash on the chain tip, or "genesis" if empty.
    fn tip_hash(&self) -> String {
        let chain = self.chain.lock().unwrap();
        chain
            .last()
            .map(|e| e.chained_hash.clone())
            .unwrap_or_else(|| "genesis".to_string())
    }

    /// CAS append: read tip, compute chained_hash, insert with prev_hash
    /// coherence check. Retries up to MAX_RETRIES if the tip changed
    /// (simulating SurrealDB UNIQUE constraint conflict on concurrent writers).
    pub async fn append(&self, poh: &ProofOfHistory) -> Result<String, StorageError> {
        const MAX_RETRIES: u32 = 5;
        for _attempt in 0..MAX_RETRIES {
            let prev_hash = self.tip_hash();
            let chained_hash = chain_hash(&poh.cryptographic_hash, &prev_hash);

            let mut chain = self.chain.lock().unwrap();
            // CAS: verify the tip hasn't moved since we read it.
            // Under true concurrency, another writer may have inserted
            // between our tip_hash() read and this lock acquisition.
            let current_tip = chain.last().map(|e| e.chained_hash.clone());
            let expected_tip = if current_tip.is_none() {
                "genesis".to_string()
            } else {
                current_tip.unwrap()
            };

            if expected_tip != prev_hash {
                // Stale tip — retry with fresh state
                continue;
            }

            let record_id = format!("poh:{}", chained_hash);
            let entry = PoHEntry {
                record_id: record_id.clone(),
                chained_hash: chained_hash.clone(),
                prev_hash,
                payload_hash: poh.cryptographic_hash.clone(),
            };
            chain.push(entry);
            return Ok(record_id);
        }
        Err(StorageError::QueryFailed(
            "PoH append failed after retries (concurrent writer)".to_string(),
        ))
    }

    /// Read-back: return the full chain for invariant verification.
    pub async fn chain_read_back(&self) -> Vec<PoHEntry> {
        self.chain.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::proof::Provable;
    use crate::domain::storage::ProofOfHistory;

    /// Test helper: build a PoH from an event_type + payload.
    fn make_poh(event_type: &str, payload_str: &str) -> ProofOfHistory {
        struct TestProvable {
            event_type: String,
            payload: crate::json::JsonValue,
        }
        impl Provable for TestProvable {
            fn source_organ(&self) -> String {
                "test-organ".to_string()
            }
            fn event_type(&self) -> String {
                self.event_type.clone()
            }
            fn proof_payload(&self) -> crate::json::JsonValue {
                self.payload.clone()
            }
        }
        let payload: crate::json::JsonValue =
            serde_json::from_str(payload_str).unwrap_or(serde_json::Value::Null);
        let prov = TestProvable {
            event_type: event_type.to_string(),
            payload,
        };
        prov.to_proof_of_history()
    }

    /// Read-back invariant: for every entry N>0, prev_hash == chained_hash of N-1.
    /// This is the core atomicity proof — a broken chain means the CAS failed.
    fn assert_chain_coherent(chain: &[PoHEntry]) {
        assert!(!chain.is_empty(), "chain must be non-empty after appends");
        if chain.len() == 1 {
            assert_eq!(
                chain[0].prev_hash, "genesis",
                "first block must have genesis prev_hash"
            );
            return;
        }
        for i in 1..chain.len() {
            assert_eq!(
                chain[i].prev_hash,
                chain[i - 1].chained_hash,
                "block {} prev_hash must equal previous block's chained_hash (chain broken)",
                i
            );
        }
    }

    #[tokio::test]
    async fn poh_sequential_appends_produce_coherent_chain() {
        let storage = InMemoryPoHStorage::new();

        let poh1 = make_poh("event_a", r#"{"step":1}"#);
        let poh2 = make_poh("event_b", r#"{"step":2}"#);

        let id1 = storage.append(&poh1).await.expect("first append");
        let id2 = storage.append(&poh2).await.expect("second append");

        assert!(!id1.is_empty() && !id2.is_empty());

        let chain = storage.chain_read_back().await;
        assert_eq!(chain.len(), 2);
        assert_chain_coherent(&chain);
    }

    #[tokio::test]
    async fn poh_concurrent_appends_maintain_chain_coherence() {
        let storage = InMemoryPoHStorage::new();

        let poh1 = make_poh("event_a", r#"{"step":1}"#);
        let poh2 = make_poh("event_b", r#"{"step":2}"#);
        let poh3 = make_poh("event_c", r#"{"step":3}"#);

        // Three concurrent writers appending to the same chain tip.
        let (r1, r2, r3) = tokio::join!(
            storage.append(&poh1),
            storage.append(&poh2),
            storage.append(&poh3),
        );

        assert!(r1.is_ok(), "writer 1 must succeed");
        assert!(r2.is_ok(), "writer 2 must succeed");
        assert!(r3.is_ok(), "writer 3 must succeed");

        let chain = storage.chain_read_back().await;
        assert_eq!(chain.len(), 3, "all three blocks must be persisted");
        assert_chain_coherent(&chain);
    }
}
