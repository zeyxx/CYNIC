use sha2::{Digest, Sha256};

use crate::channel::AgentChannel;
use crate::htlc::TaskHtlc;

#[test]
fn htlc_lifecycle_success() {
    let preimage = "build_proof_commit_05ecc253457a445d";
    let hash_lock = hex::encode(Sha256::digest(preimage.as_bytes()));

    let mut htlc = TaskHtlc::new("task-01", hash_lock, 100, 5000);
    assert!(!htlc.is_fulfilled());

    // Fulfill with correct preimage before deadline
    assert!(htlc.fulfill(preimage, 2500).is_ok());
    assert!(htlc.is_fulfilled());
    assert_eq!(htlc.revealed_preimage.as_deref(), Some(preimage));
}

#[test]
fn channel_execution_and_settlement() {
    let mut channel = AgentChannel::open(
        "chan-hermes-builder-01",
        "agent:hermes",
        "worker:builder",
        1000,
        "rev_hash_0",
    );

    let preimage = "cargo_check_ok_evidence_sha256";
    let hash_lock = hex::encode(Sha256::digest(preimage.as_bytes()));

    let htlc = TaskHtlc::new("htlc-build-01", hash_lock, 250, 10_000);
    channel.lock_task_htlc(htlc, "rev_hash_1").expect("lock ok");

    assert_eq!(channel.initiator_balance, 750);
    assert_eq!(channel.worker_balance, 0);
    assert_eq!(channel.sequence, 2);

    // Worker provides proof before deadline
    channel
        .settle_task_htlc("htlc-build-01", preimage, 5000, "rev_hash_2")
        .expect("settle ok");

    assert_eq!(channel.initiator_balance, 750);
    assert_eq!(channel.worker_balance, 250);
    assert_eq!(channel.sequence, 3);

    // Generate Layer 1 settlement proof
    let proof = channel
        .generate_settlement_proof()
        .expect("settlement proof generated");
    assert_eq!(proof.final_sequence, 3);
    assert_eq!(proof.worker_balance, 250);
}
