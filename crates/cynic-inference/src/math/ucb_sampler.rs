//! UCB1 adaptive benchmark scheduling for bounded empirical rewards.
//!
//! This is deliberately not labelled Bayesian: it is the finite-arm UCB1 rule
//! with a proved logarithmic regret bound when rewards are independent and in
//! `[0, 1]`. Parameter-space interpolation requires a separately validated
//! surrogate model and is therefore not guessed here.

/// Aggregate observations for one benchmark configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ucb1Arm {
    reward_sum: f64,
    sample_count: u64,
}

impl Ucb1Arm {
    pub const fn unobserved() -> Self {
        Self {
            reward_sum: 0.0,
            sample_count: 0,
        }
    }

    pub fn from_rewards(rewards: impl IntoIterator<Item = f64>) -> Result<Self, RewardError> {
        let mut arm = Self::unobserved();
        for reward in rewards {
            arm.record(reward)?;
        }
        Ok(arm)
    }

    /// Adds an observed normalized reward. The caller owns normalization and
    /// must retain its definition with the benchmark evidence.
    pub fn record(&mut self, reward: f64) -> Result<(), RewardError> {
        if !reward.is_finite() || !(0.0..=1.0).contains(&reward) {
            return Err(RewardError::OutOfRange(reward));
        }
        self.reward_sum += reward;
        self.sample_count += 1;
        Ok(())
    }

    pub const fn sample_count(&self) -> u64 {
        self.sample_count
    }

    pub fn mean_reward(&self) -> Option<f64> {
        (self.sample_count > 0).then(|| self.reward_sum / self.sample_count as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RewardError {
    OutOfRange(f64),
}

/// UCB1 score: `mean + sqrt(2 * ln(total samples) / arm samples)`.
/// An unobserved arm has no finite score because it must be sampled before the
/// formula applies; [`select_ucb1_arm`] handles that case deterministically.
pub fn ucb1_score(arm: Ucb1Arm, total_samples: u64) -> Option<f64> {
    let mean = arm.mean_reward()?;
    if total_samples <= 1 {
        return Some(mean);
    }
    Some(mean + (2.0 * (total_samples as f64).ln() / arm.sample_count as f64).sqrt())
}

/// Selects the next configuration index. Every unobserved configuration is
/// sampled once in input order; subsequent selection maximizes UCB1.
pub fn select_ucb1_arm(arms: &[Ucb1Arm]) -> Option<usize> {
    let total_samples = arms.iter().map(Ucb1Arm::sample_count).sum();
    arms.iter()
        .position(|arm| arm.sample_count == 0)
        .or_else(|| {
            arms.iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    ucb1_score(**left, total_samples)
                        .expect("all arms observed")
                        .partial_cmp(
                            &ucb1_score(**right, total_samples).expect("all arms observed"),
                        )
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(index, _)| index)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_every_arm_before_repeating_one() {
        let arms = [
            Ucb1Arm::from_rewards([0.8]).expect("bounded"),
            Ucb1Arm::unobserved(),
            Ucb1Arm::unobserved(),
        ];
        assert_eq!(select_ucb1_arm(&arms), Some(1));
    }

    #[test]
    fn confidence_bonus_selects_under_sampled_arm() {
        let arms = [
            Ucb1Arm::from_rewards(std::iter::repeat_n(0.9, 100)).expect("bounded"),
            Ucb1Arm::from_rewards([0.8]).expect("bounded"),
        ];
        assert_eq!(select_ucb1_arm(&arms), Some(1));
    }

    #[test]
    fn rejects_unbounded_or_non_finite_rewards() {
        assert_eq!(
            Ucb1Arm::from_rewards([1.1]),
            Err(RewardError::OutOfRange(1.1))
        );
        assert!(matches!(
            Ucb1Arm::from_rewards([f64::NAN]),
            Err(RewardError::OutOfRange(value)) if value.is_nan()
        ));
    }
}
