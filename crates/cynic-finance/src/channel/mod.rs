pub mod types;
pub mod htlc;
pub mod channel;
pub mod finance_adapters;

pub use channel::AgentChannel;
pub use htlc::TaskHtlc;
pub use types::{
    AGENT_CHANNEL_SCHEMA, ChannelPhase, ChannelSettlementProof, SETTLEMENT_PROOF_SCHEMA,
    TASK_HTLC_SCHEMA,
};
pub use finance_adapters::ChannelSettlementLedger;

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use crate::channel::channel::AgentChannel;
    use crate::channel::htlc::TaskHtlc;

    #[test]
    fn htlc_lifecycle_success() {
        let preimage = "build_proof_commit_05ecc253457a445d";
        let hash_lock = hex::encode(Sha256::digest(preimage.as_bytes()));

        let mut htlc = TaskHtlc::new("task-01", hash_lock, 100, 5000);
        assert!(!htlc.is_fulfilled());

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

        channel
            .settle_task_htlc("htlc-build-01", preimage, 5000, "rev_hash_2")
            .expect("settle ok");

        assert_eq!(channel.initiator_balance, 750);
        assert_eq!(channel.worker_balance, 250);
        assert_eq!(channel.sequence, 3);

        let proof = channel
            .generate_settlement_proof()
            .expect("settlement proof generated");
        assert_eq!(proof.final_sequence, 3);
        assert_eq!(proof.worker_balance, 250);
    }
}
