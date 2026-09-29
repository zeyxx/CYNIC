//! Finance adapters for cynic-agent-bus.
//!
//! Implements cynic-finance ActionMeter port using Claim consumed_actions.

use crate::types::Claim;
use cynic_finance::ports::{ActionMeter, MeterError};

/// Action meter — measures actions consumed by an agent from its active claim.
pub struct BusActionMeter {
    claim: Claim,
}

impl BusActionMeter {
    pub fn new(claim: Claim) -> Self {
        Self { claim }
    }
}

impl ActionMeter for BusActionMeter {
    fn measure_actions(&self, agent_id: &str) -> Result<u64, MeterError> {
        if self.claim.agent.to_string() == agent_id {
            Ok(self.claim.consumed_actions as u64)
        } else {
            Err(MeterError::Unknown(format!(
                "claim agent '{}' does not match '{}'",
                self.claim.agent, agent_id
            )))
        }
    }
}
