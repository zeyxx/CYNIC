#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]
//! PoH atomicité integration test — valide le read-back proof de chaînage
//! atomique de `store_proof_of_history` contre un SurrealDB local réel.
//!
//! Boucle complète CYNIC :
//! 1. Observe : SurrealHttpStorage::init_with (liveness + JWT + observe_schema)
//! 2. Reconcile : store_proof_of_history avec CAS retry loop
//! 3. Bounded action : INSERT avec prev_hash cohérent
//! 4. Read-back proof : SELECT chain + invariant de chaînage

mod common;

use common::setup_test_db;
use common::teardown_test_db;
use cynic_kernel::domain::proof::Provable;
use cynic_kernel::domain::storage::{ProofOfHistory, ProofOfHistoryStorage};

/// Construit un ProofOfHistory depuis un type événement testable.
fn make_poh(event_type: &str, step: u32) -> ProofOfHistory {
    struct TestEvent {
        event_type: String,
        step: u32,
    }
    impl Provable for TestEvent {
        fn source_organ(&self) -> String {
            "test-organ".to_string()
        }
        fn event_type(&self) -> String {
            self.event_type.clone()
        }
        fn proof_payload(&self) -> serde_json::Value {
            serde_json::json!({ "step": self.step })
        }
    }
    let ev = TestEvent {
        event_type: event_type.to_string(),
        step,
    };
    ev.to_proof_of_history()
}

/// Read-back invariant : pour chaque bloc N>0,
/// prev_hash == cryptographic_hash du bloc N-1
fn assert_chain_coherent(chain: &[ProofOfHistory]) {
    assert!(!chain.is_empty(), "chain must persist after appends");
    for i in 1..chain.len() {
        let prev_hash = chain[i]
            .prev_hash
            .as_ref()
            .expect("prev_hash must be set on non-genesis blocks");
        assert_eq!(
            prev_hash, &chain[i - 1].cryptographic_hash,
            "block {} prev_hash must equal previous block's cryptographic_hash",
            i
        );
    }
}

/// 3 appends concurrents → read-back invariant de chaînage atomique.
#[tokio::test]
async fn poh_atomic_concurrent_appends_maintain_chain_coherence() {
    let storage = match setup_test_db("poh_atomic").await {
        Some(s) => s,
        None => {
            eprintln!("[SKIP] SurrealDB unavailable — test deferred");
            return;
        }
    };

    let _ = storage
        .query("DEFINE TABLE proof_of_history SCHEMAFULL;")
        .await;
    let _ = storage
        .query("DEFINE FIELD cryptographic_hash ON proof_of_history AS TYPE string;")
        .await;
    let _ = storage
        .query("DEFINE FIELD prev_hash ON proof_of_history;")
        .await;

    let poh1 = make_poh("event_a", 1);
    let poh2 = make_poh("event_b", 2);
    let poh3 = make_poh("event_c", 3);

    let (r1, r2, r3) = tokio::join!(
        storage.store_proof_of_history(&poh1),
        storage.store_proof_of_history(&poh2),
        storage.store_proof_of_history(&poh3),
    );

    assert!(r1.is_ok(), "writer 1 must succeed: {:?}", r1);
    assert!(r2.is_ok(), "writer 2 must succeed: {:?}", r2);
    assert!(r3.is_ok(), "writer 3 must succeed: {:?}", r3);

    let chain = storage
        .list_proof_of_history(10)
        .await
        .expect("read-back list_proof_of_history");
    assert_eq!(chain.len(), 3, "all three blocks must be persisted");
    // SurrealDB ORDER BY timestamp DESC → reverse pour la chaîne forward
    let chain_forward: Vec<_> = chain.into_iter().rev().collect();
    assert_chain_coherent(&chain_forward);

    teardown_test_db(&storage).await;
}
