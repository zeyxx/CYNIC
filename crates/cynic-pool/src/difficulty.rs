//! Difficulty adjustment and PPLNS reward calculator for CYNIC pool.

use serde::{Deserialize, Serialize};

/// Difficulty target (lower = harder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Difficulty(pub u64);

impl Difficulty {
    pub const MIN: Difficulty = Difficulty(1);
    pub const MAX: Difficulty = Difficulty(u64::MAX);

    pub fn new(value: u64) -> Self {
        Self(value.max(Self::MIN.0))
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

/// PPLNS (Pay Per Last N Shares) calculator.
#[derive(Debug, Clone)]
pub struct PPLNSCalculator {
    pub n: usize,
    pub block_reward: f64,
    pub shares: Vec<(String, u64)>, // (agent_id, share_value)
}

impl PPLNSCalculator {
    pub fn new(n: usize, block_reward: f64) -> Self {
        Self {
            n,
            block_reward,
            shares: Vec::new(),
        }
    }

    pub fn add_share(&mut self, agent_id: &str, value: u64) {
        self.shares.push((agent_id.to_string(), value));
    }

    /// Distribute reward proportionally to the last N shares.
    pub fn distribute(&self) -> Vec<(String, f64)> {
        let start = self.shares.len().saturating_sub(self.n);
        let recent = &self.shares[start..];
        let total_value: u64 = recent.iter().map(|(_, v)| v).sum();

        if total_value == 0 {
            return Vec::new();
        }

        let mut rewards: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for (agent_id, value) in recent {
            let fraction = *value as f64 / total_value as f64;
            *rewards.entry(agent_id.clone()).or_default() += fraction * self.block_reward;
        }

        rewards.into_iter().collect()
    }
}

/// Halving schedule: reward halves every N epochs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalvingSchedule {
    pub initial_reward: f64,
    pub halving_interval_epochs: u64,
}

impl Default for HalvingSchedule {
    fn default() -> Self {
        Self {
            initial_reward: 50.0,
            halving_interval_epochs: 210_000,
        }
    }
}

impl HalvingSchedule {
    pub fn reward_at_epoch(&self, epoch: u64) -> f64 {
        let halvings = epoch / self.halving_interval_epochs;
        self.initial_reward / 2.0_f64.powi(halvings as i32).max(1.0)
    }
}

/// Adjust difficulty for a new epoch based on actual vs target duration.
pub fn adjust_difficulty(
    current_difficulty: Difficulty,
    target_epoch_ms: u64,
    actual_epoch_ms: u64,
) -> Difficulty {
    if actual_epoch_ms == 0 || target_epoch_ms == 0 {
        return current_difficulty;
    }
    let ratio = target_epoch_ms as f64 / actual_epoch_ms as f64;
    // Clamp adjustment to [0.25, 4.0] (same as Bitcoin).
    let clamped = ratio.clamp(0.25, 4.0);
    let new_value = (current_difficulty.value() as f64 * clamped) as u64;
    Difficulty::new(new_value)
}

/// Compute reward for a block given epoch number and schedule.
pub fn reward_for_block(epoch: u64, schedule: &HalvingSchedule) -> f64 {
    schedule.reward_at_epoch(epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_clamped_to_min() {
        let d = Difficulty::new(0);
        assert_eq!(d.value(), 1);
    }

    #[test]
    fn adjust_difficulty_slow_down() {
        // If actual > target, decrease difficulty (make easier).
        let current = Difficulty::new(100);
        let adjusted = adjust_difficulty(current, 1000, 2000);
        assert!(adjusted.value() < current.value());
    }

    #[test]
    fn adjust_difficulty_speed_up() {
        // If actual < target, increase difficulty (make harder).
        let current = Difficulty::new(100);
        let adjusted = adjust_difficulty(current, 2000, 1000);
        assert!(adjusted.value() > current.value());
    }

    #[test]
    fn adjust_difficulty_clamped() {
        let current = Difficulty::new(100);
        let adjusted = adjust_difficulty(current, 100, 1_000_000);
        // Ratio would be 0.0001, but clamped to 0.25.
        assert_eq!(adjusted.value(), 25);
    }

    #[test]
    fn pplns_proportional_distribution() {
        let mut calc = PPLNSCalculator::new(100, 50.0);
        calc.add_share("agent_a", 30);
        calc.add_share("agent_b", 70);
        let rewards = calc.distribute();
        assert_eq!(rewards.len(), 2);
        let a = rewards.iter().find(|(id, _)| id == "agent_a").unwrap();
        let b = rewards.iter().find(|(id, _)| id == "agent_b").unwrap();
        assert!((a.1 - 15.0).abs() < 0.001);
        assert!((b.1 - 35.0).abs() < 0.001);
    }

    #[test]
    fn halving_schedule() {
        let schedule = HalvingSchedule::default();
        assert!((schedule.reward_at_epoch(0) - 50.0).abs() < 0.001);
        assert!((schedule.reward_at_epoch(210_000) - 25.0).abs() < 0.001);
        assert!((schedule.reward_at_epoch(420_000) - 12.5).abs() < 0.001);
    }
}
