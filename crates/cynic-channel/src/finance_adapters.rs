//! Finance adapters for cynic-channel.
//!
//! Implements cynic-finance integration for AgentChannel settlements.
//! Converts channel settlements into ResourceTransaction entries.

use crate::types::ChannelSettlementProof;
use cynic_finance::ResourceUnit;
use cynic_finance::ports::MeterError;
use cynic_finance::types::{
    ResourceLedger, ResourceTransaction, ResourceType, TransactionDirection,
};

/// Adds a token amount to a balance, keeping the unit.
///
/// `ResourceUnit` has no arithmetic in `cynic-finance`, and
/// `increment_unit` there is private to that crate. A ledger balance is a
/// TOKEN balance, so the addition is the token one; a balance of another
/// unit is returned unchanged rather than silently reinterpreted.
fn add_token_cyd(balance: &ResourceUnit, amount: u64) -> ResourceUnit {
    match balance {
        ResourceUnit::TokenCyd(current) => ResourceUnit::TokenCyd(current.saturating_add(amount)),
        other => other.clone(),
    }
}

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

        // The transaction id must be UNIQUE per settlement.
        //
        // It used to be `{channel_id}/debit/{final_sequence}`, which
        // repeats for two settlements of the same channel and the same
        // final sequence: the second one produced the id of the first.
        // `ResourceLedger` is documented as "idempotent", and an id that
        // repeats is not an identity — the second settlement would be
        // indistinguishable from the first.
        //
        // The timestamp is what distinguishes them: it is the moment the
        // settlement happened, and it differs between two settlements of
        // the same sequence.
        let settlement_id = format!("{}/settlement/{}", proof.channel_id, timestamp);

        // Debit from initiator (remaining balance returned)
        let debit_tx = ResourceTransaction::new(
            format!("{settlement_id}/debit"),
            timestamp,
            "initiator",
            ResourceUnit::TokenCyd(proof.initiator_balance),
            TransactionDirection::Debit,
            "channel_settlement",
            Default::default(),
        );

        // Credit to worker (earned balance)
        let credit_tx = ResourceTransaction::new(
            format!("{settlement_id}/credit"),
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

        // The balance ACCUMULATES.
        //
        // It used to be REPLACED (`self.ledger.balance = total`), so a
        // second settlement overwrote the first and the ledger reported
        // only the last settlement's total. The test
        // `record_settlement_multiple_times_accumulates` caught it:
        // two settlements of 1000 each reported 1000 instead of 2000.
        //
        // `ResourceLedger` documents its balance as "Derived balance —
        // computed from entries, never stored independently", and the
        // ledger as "append-only, monotone". Monotone means the balance
        // only grows, so a settlement adds to it.
        //
        // The amount added is the settled TOTAL — the debit and the
        // credit together are the amount that crossed the channel, and
        // the two entries are the two sides of that one movement. Adding
        // the credit alone would report what the worker earned and lose
        // the initiator's half of the settlement.
        self.ledger.balance = add_token_cyd(&self.ledger.balance, total);

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
