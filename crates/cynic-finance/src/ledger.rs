//! Ledger engine — append-only, monotone, idempotent, double-entry.
//!
//! This module provides the core ledger logic. It is pure — no I/O.

use crate::types::{
    ResourceLedger, ResourceTransaction, ResourceType,
};

/// Errors that can occur in ledger operations.
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerError {
    /// Transaction ID already exists (idempotency violation).
    DuplicateTransaction(String),
    /// Resource type mismatch between transaction and ledger.
    ResourceTypeMismatch {
        expected: ResourceType,
        actual: ResourceType,
    },
    /// Insufficient balance for a debit transaction.
    InsufficientBalance {
        available: f64,
        requested: f64,
    },
    /// Proof verification failed.
    InvalidProof(String),
    /// Ledger is empty.
    EmptyLedger,
}

/// The ledger engine — manages resource ledgers with strict invariants.
#[derive(Debug, Clone)]
pub struct LedgerEngine {
    ledgers: Vec<ResourceLedger>,
}

impl LedgerEngine {
    /// Creates a new empty ledger engine.
    pub fn new() -> Self {
        Self {
            ledgers: Vec::new(),
        }
    }

    /// Returns the number of ledgers managed by this engine.
    pub fn ledger_count(&self) -> usize {
        self.ledgers.len()
    }

    /// Returns all ledgers.
    pub fn ledgers(&self) -> &[ResourceLedger] {
        &self.ledgers
    }

    /// Finds a ledger by ID.
    pub fn find_ledger(&self, ledger_id: &str) -> Option<&ResourceLedger> {
        self.ledgers.iter().find(|l| l.ledger_id == ledger_id)
    }

    /// Finds a mutable ledger by ID.
    fn find_ledger_mut(&mut self, ledger_id: &str) -> Option<&mut ResourceLedger> {
        self.ledgers.iter_mut().find(|l| l.ledger_id == ledger_id)
    }

    /// Creates a new ledger for a given resource type.
    pub fn create_ledger(
        &mut self,
        ledger_id: impl Into<String>,
        resource_type: ResourceType,
    ) -> &ResourceLedger {
        let ledger = ResourceLedger::new(ledger_id, resource_type);
        self.ledgers.push(ledger);
        self.ledgers.last().unwrap()
    }

    /// Appends a transaction to a ledger.
    ///
    /// Invariants enforced:
    /// - Append-only: entries are never modified or deleted
    /// - Idempotent: same transaction_id applied twice is a no-op
    /// - Monotone: balance only decreases for debits, only increases for credits
    /// - Double-entry: every debit has a corresponding credit
    pub fn append(
        &mut self,
        ledger_id: &str,
        transaction: ResourceTransaction,
    ) -> Result<(), LedgerError> {
        let ledger = self
            .find_ledger_mut(ledger_id)
            .ok_or_else(|| LedgerError::EmptyLedger)?;

        // Check resource type matches
        if transaction.resource.resource_type() != ledger.resource_type {
            return Err(LedgerError::ResourceTypeMismatch {
                expected: ledger.resource_type,
                actual: transaction.resource.resource_type(),
            });
        }

        // Delegate to ResourceLedger's AppendOnlyJournal invariants.
        let value = transaction.resource.as_f64();
        let current_balance = ledger.balance.as_f64();
        use cynic_protocol::journal_port::AppendOnlyJournal;
        match ledger.append(transaction) {
            Ok(_) => Ok(()),
            Err(msg) => {
                if msg.starts_with("invalid proof") {
                    Err(LedgerError::InvalidProof(String::new()))
                } else if msg.starts_with("duplicate transaction_id") {
                    let id = msg.split(": ").nth(1).unwrap_or("").to_string();
                    Err(LedgerError::DuplicateTransaction(id))
                } else if msg.starts_with("insufficient balance") {
                    Err(LedgerError::InsufficientBalance {
                        available: current_balance,
                        requested: value,
                    })
                } else {
                    Err(LedgerError::InvalidProof(msg))
                }
            }
        }
    }

    /// Returns the balance of a ledger.
    pub fn balance(&self, ledger_id: &str) -> Option<&crate::types::ResourceUnit> {
        self.find_ledger(ledger_id).map(|l| l.balance())
    }

    /// Returns all entries of a ledger.
    pub fn entries(&self, ledger_id: &str) -> Option<&[ResourceTransaction]> {
        self.find_ledger(ledger_id).map(|l| l.entries())
    }
}

impl Default for LedgerEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Decrements a resource unit by a given amount.
pub fn decrement_unit(unit: &crate::types::ResourceUnit, amount: f64) -> crate::types::ResourceUnit {
    use crate::types::ResourceUnit::*;
    match unit {
        ComputeCpuMs(v) => ComputeCpuMs((*v as f64 - amount).max(0.0) as u64),
        ComputeGpuMs(v) => ComputeGpuMs((*v as f64 - amount).max(0.0) as u64),
        ComputeVramBytes(v) => ComputeVramBytes((*v as f64 - amount).max(0.0) as u64),
        StorageBytes(v) => StorageBytes((*v as f64 - amount).max(0.0) as u64),
        StorageIops(v) => StorageIops((*v as f64 - amount).max(0.0) as u64),
        NetworkBytes(v) => NetworkBytes((*v as f64 - amount).max(0.0) as u64),
        NetworkPackets(v) => NetworkPackets((*v as f64 - amount).max(0.0) as u64),
        EnergyWatts(v) => EnergyWatts((*v - amount).max(0.0)),
        EnergyKwh(v) => EnergyKwh((*v - amount).max(0.0)),
        TokenCyd(v) => TokenCyd((*v as f64 - amount).max(0.0) as u64),
        ContextTokens(v) => ContextTokens((*v as f64 - amount).max(0.0) as u64),
        TimeSeconds(v) => TimeSeconds((*v as f64 - amount).max(0.0) as u64),
        Actions(v) => Actions((*v as f64 - amount).max(0.0) as u64),
    }
}

/// Increments a resource unit by a given amount.
///
/// Public for the same reason as `decrement_unit`: the arithmetic that
/// moves a balance belongs to one place.
pub fn increment_unit(unit: &crate::types::ResourceUnit, amount: f64) -> crate::types::ResourceUnit {
    use crate::types::ResourceUnit::*;
    match unit {
        ComputeCpuMs(v) => ComputeCpuMs(*v + amount as u64),
        ComputeGpuMs(v) => ComputeGpuMs(*v + amount as u64),
        ComputeVramBytes(v) => ComputeVramBytes(*v + amount as u64),
        StorageBytes(v) => StorageBytes(*v + amount as u64),
        StorageIops(v) => StorageIops(*v + amount as u64),
        NetworkBytes(v) => NetworkBytes(*v + amount as u64),
        NetworkPackets(v) => NetworkPackets(*v + amount as u64),
        EnergyWatts(v) => EnergyWatts(*v + amount),
        EnergyKwh(v) => EnergyKwh(*v + amount),
        TokenCyd(v) => TokenCyd(*v + amount as u64),
        ContextTokens(v) => ContextTokens(*v + amount as u64),
        TimeSeconds(v) => TimeSeconds(*v + amount as u64),
        Actions(v) => Actions(*v + amount as u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ResourceMetadata, ResourceUnit, TransactionDirection};

    fn sample_transaction(id: &str, direction: TransactionDirection) -> ResourceTransaction {
        ResourceTransaction::new(
            id,
            1000,
            "agent-1",
            ResourceUnit::TokenCyd(100),
            direction,
            "treasury",
            ResourceMetadata::default(),
        )
    }

    #[test]
    fn ledger_engine_creates_empty_ledger() {
        let mut engine = LedgerEngine::new();
        engine.create_ledger("led-1", ResourceType::Token);
        assert_eq!(engine.ledger_count(), 1);
        assert!(engine.balance("led-1").is_some());
    }

    #[test]
    fn append_credit_increases_balance() {
        let mut engine = LedgerEngine::new();
        engine.create_ledger("led-1", ResourceType::Token);
        let tx = sample_transaction("tx-1", TransactionDirection::Credit);
        engine.append("led-1", tx).unwrap();
        assert_eq!(engine.balance("led-1"), Some(&ResourceUnit::TokenCyd(100)));
    }

    #[test]
    fn append_debit_decreases_balance() {
        let mut engine = LedgerEngine::new();
        engine.create_ledger("led-1", ResourceType::Token);
        let tx1 = sample_transaction("tx-1", TransactionDirection::Credit);
        engine.append("led-1", tx1).unwrap();
        let tx2 = sample_transaction("tx-2", TransactionDirection::Debit);
        engine.append("led-1", tx2).unwrap();
        assert_eq!(engine.balance("led-1"), Some(&ResourceUnit::TokenCyd(0)));
    }

    #[test]
    fn duplicate_transaction_is_rejected() {
        let mut engine = LedgerEngine::new();
        engine.create_ledger("led-1", ResourceType::Token);
        let tx = sample_transaction("tx-1", TransactionDirection::Credit);
        engine.append("led-1", tx.clone()).unwrap();
        let result = engine.append("led-1", tx);
        assert!(matches!(result, Err(LedgerError::DuplicateTransaction(_))));
    }

    #[test]
    fn insufficient_balance_is_rejected() {
        let mut engine = LedgerEngine::new();
        engine.create_ledger("led-1", ResourceType::Token);
        let tx = sample_transaction("tx-1", TransactionDirection::Debit);
        let result = engine.append("led-1", tx);
        assert!(matches!(result, Err(LedgerError::InsufficientBalance { .. })));
    }

    #[test]
    fn resource_type_mismatch_is_rejected() {
        let mut engine = LedgerEngine::new();
        engine.create_ledger("led-1", ResourceType::Token);
        let tx = ResourceTransaction::new(
            "tx-1",
            1000,
            "agent-1",
            ResourceUnit::ComputeCpuMs(100),
            TransactionDirection::Credit,
            "treasury",
            ResourceMetadata::default(),
        );
        let result = engine.append("led-1", tx);
        assert!(matches!(result, Err(LedgerError::ResourceTypeMismatch { .. })));
    }

    #[test]
    fn proof_verification_succeeds() {
        let tx = sample_transaction("tx-1", TransactionDirection::Credit);
        assert!(tx.verify_proof());
    }

    #[test]
    fn proof_verification_fails_on_tampering() {
        let mut tx = sample_transaction("tx-1", TransactionDirection::Credit);
        tx.resource = ResourceUnit::TokenCyd(999); // Tamper with the transaction
        assert!(!tx.verify_proof());
    }
}
