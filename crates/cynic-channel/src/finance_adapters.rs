//! Finance adapters for cynic-channel.
//!
//! Implements cynic-finance integration for AgentChannel settlements.
//! Converts channel settlements into ResourceTransaction entries.

use crate::types::ChannelSettlementProof;
use cynic_finance::ports::MeterError;
use cynic_finance::types::{
    ResourceLedger, ResourceTransaction, ResourceType, TransactionDirection,
};
use cynic_finance::ResourceUnit;

/// Channel settlement ledger — records channel settlements as ResourceTransaction.
///
/// Each settlement produces two transactions:
/// - Debit from initiator (budget consumed)
/// - Credit to worker (budget earned)
///
/// This bridges the off-chain channel state with the on-chain ledger.
pub struct ChannelSettlementLedger {
    ledger: ResourceLedger,
}

impl ChannelSettlementLedger {
    /// Creates a new empty settlement ledger.
    pub fn new(channel_id: impl Into<String>) -> Self {
        Self {
            ledger: ResourceLedger::new(
                format!("{}/settlements", channel_id.into()),
                ResourceType::Token,
            ),
        }
    }

    /// Records a channel settlement as two ResourceTransaction entries.
    ///
    /// Returns the transaction IDs of the debit and credit entries.
    pub fn record_settlement(
        &mut self,
        proof: &ChannelSettlementProof,
        timestamp: u64,
    ) -> Result<(String, String), MeterError> {
        let total = proof.initiator_balance + proof.worker_balance;

        // Debit from initiator (remaining balance returned)
        let debit_tx = ResourceTransaction::new(
            format!("{}/debit/{}", proof.channel_id, proof.final_sequence),
            timestamp,
            "initiator",
            ResourceUnit::TokenCyd(proof.initiator_balance),
            TransactionDirection::Debit,
            "channel_settlement",
            Default::default(),
        );

        // Credit to worker (earned balance)
        let credit_tx = ResourceTransaction::new(
            format!("{}/credit/{}", proof.channel_id, proof.final_sequence),
            timestamp,
            "worker",
            ResourceUnit::TokenCyd(proof.worker_balance),
            TransactionDirection::Credit,
            "channel_settlement",
            Default::default(),
        );

        let debit_id = debit_tx.transaction_id.clone();
        let credit_id = credit_tx.transaction_id.clone();

        self.ledger.entries.push(debit_tx);
        self.ledger.entries.push(credit_tx);

        // Update balance
        self.ledger.balance = ResourceUnit::TokenCyd(total);

        Ok((debit_id, credit_id))
    }

    /// Returns the current ledger balance.
    pub fn balance(&self) -> &ResourceUnit {
        &self.ledger.balance
    }

    /// Returns all recorded transactions.
    pub fn transactions(&self) -> &[ResourceTransaction] {
        &self.ledger.entries
    }

    /// Returns the number of transactions.
    pub fn len(&self) -> usize {
        self.ledger.entries.len()
    }

    /// Returns true if no transactions recorded.
    pub fn is_empty(&self) -> bool {
        self.ledger.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SETTLEMENT_PROOF_SCHEMA;

    fn sample_proof() -> ChannelSettlementProof {
        ChannelSettlementProof {
            schema: SETTLEMENT_PROOF_SCHEMA.to_string(),
            channel_id: "ch1".to_string(),
            final_sequence: 5,
            initiator_balance: 800,
            worker_balance: 200,
            settlement_digest: "abc123".to_string(),
        }
    }

    #[test]
    fn settlement_ledger_new_is_empty() {
        let ledger = ChannelSettlementLedger::new("ch1");
        assert!(ledger.is_empty());
        assert_eq!(ledger.balance(), &ResourceUnit::TokenCyd(0));
    }

    #[test]
    fn record_settlement_creates_two_transactions() {
        let mut ledger = ChannelSettlementLedger::new("ch1");
        let proof = sample_proof();
        let (debit_id, credit_id) = ledger.record_settlement(&proof, 1000).unwrap();

        assert!(!debit_id.is_empty());
        assert!(!credit_id.is_empty());
        assert_eq!(ledger.len(), 2);
        assert_eq!(ledger.balance(), &ResourceUnit::TokenCyd(1000));
    }

    #[test]
    fn record_settlement_transactions_have_correct_direction() {
        let mut ledger = ChannelSettlementLedger::new("ch1");
        let proof = sample_proof();
        ledger.record_settlement(&proof, 1000).unwrap();

        let debit = &ledger.transactions()[0];
        assert_eq!(debit.direction, TransactionDirection::Debit);
        assert_eq!(debit.resource, ResourceUnit::TokenCyd(800));

        let credit = &ledger.transactions()[1];
        assert_eq!(credit.direction, TransactionDirection::Credit);
        assert_eq!(credit.resource, ResourceUnit::TokenCyd(200));
    }

    #[test]
    fn record_settlement_multiple_times_accumulates() {
        let mut ledger = ChannelSettlementLedger::new("ch1");
        let proof = sample_proof();

        ledger.record_settlement(&proof, 1000).unwrap();
        ledger.record_settlement(&proof, 2000).unwrap();

        assert_eq!(ledger.len(), 4);
        assert_eq!(ledger.balance(), &ResourceUnit::TokenCyd(2000));
    }
}
