//! Finance adapters for cynic-consensus.
//!
//! Implements cynic-finance TokenMeter port using TreasuryAccount.

use crate::peer::accounts::TreasuryAccount;
use cynic_finance::ports::{MeterError, TokenMeter};

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
