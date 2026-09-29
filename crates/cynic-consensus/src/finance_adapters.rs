//! Finance adapters for cynic-consensus.
//!
//! Implements cynic-finance TokenMeter port using TreasuryAccount.
//! Provides TreasuryLedger: 4 ResourceLedger<TokenCyd>, one per credit type.

use crate::peer::accounts::TreasuryAccount;
use cynic_finance::ports::{MeterError, TokenMeter};
use cynic_finance::types::{
    ResourceLedger, ResourceTransaction, ResourceType, TransactionDirection,
};
use cynic_finance::ResourceUnit;

/// Token meter — measures CYD balance from a TreasuryAccount.
pub struct ConsensusTokenMeter {
    treasury: TreasuryAccount,
}

impl ConsensusTokenMeter {
    pub fn new(treasury: TreasuryAccount) -> Self {
        Self { treasury }
    }
}

impl TokenMeter for ConsensusTokenMeter {
    fn measure_cyd(&self, agent_id: &str) -> Result<u64, MeterError> {
        if self.treasury.treasury_id == agent_id {
            Ok(self.treasury.cyd_calibration_credits)
        } else {
            Err(MeterError::Unknown(format!(
                "treasury '{}' does not match agent '{}'",
                self.treasury.treasury_id, agent_id
            )))
        }
    }
}

/// Treasury ledger — 4 ResourceLedger<TokenCyd>, one per credit type.
///
/// Each ledger tracks a distinct CYD balance with full transaction history:
/// - `calibration` — cyd_calibration_credits
/// - `escrow` — escrow_credits
/// - `disputed` — disputed_credits
/// - `spent` — total_spent
#[derive(Debug, Clone)]
pub struct TreasuryLedger {
    pub calibration: ResourceLedger,
    pub escrow: ResourceLedger,
    pub disputed: ResourceLedger,
    pub spent: ResourceLedger,
}

impl TreasuryLedger {
    /// Creates a new empty treasury ledger.
    pub fn new(treasury_id: impl Into<String>) -> Self {
        let id = treasury_id.into();
        Self {
            calibration: ResourceLedger::new(format!("{}/calibration", id), ResourceType::Token),
            escrow: ResourceLedger::new(format!("{}/escrow", id), ResourceType::Token),
            disputed: ResourceLedger::new(format!("{}/disputed", id), ResourceType::Token),
            spent: ResourceLedger::new(format!("{}/spent", id), ResourceType::Token),
        }
    }

    /// Creates a treasury ledger from an existing TreasuryAccount.
    pub fn from_treasury(treasury: &TreasuryAccount) -> Self {
        let mut ledger = Self::new(&treasury.treasury_id);

        // Seed calibration balance
        if treasury.cyd_calibration_credits > 0 {
            let tx = ResourceTransaction::new(
                format!("{}/genesis/calibration", treasury.treasury_id),
                0,
                &treasury.treasury_id,
                ResourceUnit::TokenCyd(treasury.cyd_calibration_credits),
                TransactionDirection::Credit,
                "genesis",
                Default::default(),
            );
            ledger.calibration.entries.push(tx);
            ledger.calibration.balance = ResourceUnit::TokenCyd(treasury.cyd_calibration_credits);
        }

        // Seed escrow balance
        if treasury.escrow_credits > 0 {
            let tx = ResourceTransaction::new(
                format!("{}/genesis/escrow", treasury.treasury_id),
                0,
                &treasury.treasury_id,
                ResourceUnit::TokenCyd(treasury.escrow_credits),
                TransactionDirection::Credit,
                "genesis",
                Default::default(),
            );
            ledger.escrow.entries.push(tx);
            ledger.escrow.balance = ResourceUnit::TokenCyd(treasury.escrow_credits);
        }

        // Seed disputed balance
        if treasury.disputed_credits > 0 {
            let tx = ResourceTransaction::new(
                format!("{}/genesis/disputed", treasury.treasury_id),
                0,
                &treasury.treasury_id,
                ResourceUnit::TokenCyd(treasury.disputed_credits),
                TransactionDirection::Credit,
                "genesis",
                Default::default(),
            );
            ledger.disputed.entries.push(tx);
            ledger.disputed.balance = ResourceUnit::TokenCyd(treasury.disputed_credits);
        }

        // Seed spent balance
        if treasury.total_spent > 0 {
            let tx = ResourceTransaction::new(
                format!("{}/genesis/spent", treasury.treasury_id),
                0,
                &treasury.treasury_id,
                ResourceUnit::TokenCyd(treasury.total_spent),
                TransactionDirection::Debit,
                "genesis",
                Default::default(),
            );
            ledger.spent.entries.push(tx);
            ledger.spent.balance = ResourceUnit::TokenCyd(treasury.total_spent);
        }

        ledger
    }

    /// Returns the total CYD balance across all ledgers.
    pub fn total_cyd(&self) -> u64 {
        self.calibration.balance.as_f64() as u64
            + self.escrow.balance.as_f64() as u64
            + self.disputed.balance.as_f64() as u64
    }

    /// Returns the total spent CYD.
    pub fn total_spent(&self) -> u64 {
        self.spent.balance.as_f64() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_treasury() -> TreasuryAccount {
        TreasuryAccount {
            schema: "cynic.peer.account.treasury.v1".to_string(),
            treasury_id: "trs_main".to_string(),
            cyd_calibration_credits: 1000,
            escrow_credits: 200,
            disputed_credits: 50,
            total_spent: 300,
        }
    }

    #[test]
    fn treasury_ledger_from_treasury_seeds_balances() {
        let treasury = sample_treasury();
        let ledger = TreasuryLedger::from_treasury(&treasury);

        assert_eq!(ledger.calibration.balance, ResourceUnit::TokenCyd(1000));
        assert_eq!(ledger.escrow.balance, ResourceUnit::TokenCyd(200));
        assert_eq!(ledger.disputed.balance, ResourceUnit::TokenCyd(50));
        assert_eq!(ledger.spent.balance, ResourceUnit::TokenCyd(300));
    }

    #[test]
    fn treasury_ledger_total_cyd_sums_all_ledgers() {
        let treasury = sample_treasury();
        let ledger = TreasuryLedger::from_treasury(&treasury);
        assert_eq!(ledger.total_cyd(), 1250);
    }

    #[test]
    fn treasury_ledger_total_spent_matches_spent_ledger() {
        let treasury = sample_treasury();
        let ledger = TreasuryLedger::from_treasury(&treasury);
        assert_eq!(ledger.total_spent(), 300);
    }

    #[test]
    fn treasury_ledger_new_creates_empty_ledgers() {
        let ledger = TreasuryLedger::new("trs_test");
        assert_eq!(ledger.calibration.balance, ResourceUnit::TokenCyd(0));
        assert_eq!(ledger.escrow.balance, ResourceUnit::TokenCyd(0));
        assert_eq!(ledger.disputed.balance, ResourceUnit::TokenCyd(0));
        assert_eq!(ledger.spent.balance, ResourceUnit::TokenCyd(0));
        assert_eq!(ledger.total_cyd(), 0);
    }

    #[test]
    fn token_meter_returns_calibration_credits() {
        let treasury = sample_treasury();
        let meter = ConsensusTokenMeter::new(treasury);
        assert_eq!(meter.measure_cyd("trs_main").unwrap(), 1000);
    }

    #[test]
    fn token_meter_rejects_unknown_agent() {
        let treasury = sample_treasury();
        let meter = ConsensusTokenMeter::new(treasury);
        assert!(meter.measure_cyd("other").is_err());
    }
}
