//! Reward distribution and tracking for CYNIC pool contributors.

use crate::difficulty::HalvingSchedule;
use serde::{Deserialize, Serialize};

/// Tracks reward state for the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardState {
    pub total_distributed: f64,
    pub total_shares_processed: u64,
    pub current_epoch: u64,
    pub schedule: HalvingSchedule,
    pub contributor_balances: Vec<ContributorBalance>,
}

/// Balance for a single contributor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorBalance {
    pub agent_id: String,
    pub pending_rewards: f64,
    pub claimed_rewards: f64,
    pub last_claimed_epoch: u64,
}

impl RewardState {
    pub fn new(schedule: HalvingSchedule) -> Self {
        Self {
            total_distributed: 0.0,
            total_shares_processed: 0,
            current_epoch: 0,
            schedule,
            contributor_balances: Vec::new(),
        }
    }

    /// Get or create a contributor balance entry.
    pub fn balance_for(&mut self, agent_id: &str) -> &mut ContributorBalance {
        if let Some(pos) = self
            .contributor_balances
            .iter()
            .position(|b| b.agent_id == agent_id)
        {
            &mut self.contributor_balances[pos]
        } else {
            self.contributor_balances.push(ContributorBalance {
                agent_id: agent_id.to_string(),
                pending_rewards: 0.0,
                claimed_rewards: 0.0,
                last_claimed_epoch: 0,
            });
            self.contributor_balances.last_mut().unwrap()
        }
    }

    /// Add rewards to a contributor's pending balance.
    pub fn credit(&mut self, agent_id: &str, amount: f64) {
        let balance = self.balance_for(agent_id);
        balance.pending_rewards += amount;
        self.total_distributed += amount;
    }

    /// Claim rewards up to a given epoch.
    pub fn claim(
        &mut self,
        agent_id: &str,
        up_to_epoch: u64,
    ) -> Result<f64, crate::error::PoolError> {
        let balance = self.balance_for(agent_id);
        if up_to_epoch <= balance.last_claimed_epoch {
            return Err(crate::error::PoolError::RewardClaimInvalid(format!(
                "epoch {} already claimed",
                up_to_epoch
            )));
        }
        let claimable = balance.pending_rewards;
        balance.claimed_rewards += claimable;
        balance.pending_rewards = 0.0;
        balance.last_claimed_epoch = up_to_epoch;
        Ok(claimable)
    }
}

/// Summary of rewards for a contributor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSummary {
    pub agent_id: String,
    pub pending: f64,
    pub claimed: f64,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_state_tracks_balances() {
        let schedule = HalvingSchedule::default();
        let mut state = RewardState::new(schedule);
        state.credit("agent_a", 10.0);
        state.credit("agent_b", 20.0);
        state.credit("agent_a", 5.0);

        assert_eq!(state.balance_for("agent_a").pending_rewards, 15.0);
        assert_eq!(state.balance_for("agent_b").pending_rewards, 20.0);
        assert_eq!(state.total_distributed, 35.0);
    }

    #[test]
    fn claim_transfers_to_claimed() {
        let schedule = HalvingSchedule::default();
        let mut state = RewardState::new(schedule);
        state.credit("agent_a", 10.0);

        let claimed = state.claim("agent_a", 1).unwrap();
        assert_eq!(claimed, 10.0);
        assert_eq!(state.balance_for("agent_a").pending_rewards, 0.0);
        assert_eq!(state.balance_for("agent_a").claimed_rewards, 10.0);
    }

    #[test]
    fn claim_fails_if_already_claimed() {
        let schedule = HalvingSchedule::default();
        let mut state = RewardState::new(schedule);
        state.credit("agent_a", 10.0);
        state.claim("agent_a", 1).unwrap();

        let result = state.claim("agent_a", 1);
        assert!(result.is_err());
    }
}
