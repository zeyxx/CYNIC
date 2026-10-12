//! Scoring engine — PoW, PoJ, authority updates.

/// Configuration for Proof-of-Work scoring.
#[derive(Debug, Clone)]
pub struct PowConfig {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
}

impl Default for PowConfig {
    fn default() -> Self {
        Self {
            alpha: 0.7,
            beta: 0.3,
            gamma: 0.1,
        }
    }
}

/// Configuration for Proof-of-Jury scoring.
#[derive(Debug, Clone)]
pub struct PojConfig {
    pub min_jurors: usize,
    pub max_jurors: usize,
    pub escalation_threshold: f64,
}

impl Default for PojConfig {
    fn default() -> Self {
        Self {
            min_jurors: 3,
            max_jurors: 8,
            escalation_threshold: 0.3,
        }
    }
}

/// Scoring engine.
#[derive(Debug)]
pub struct ScoringEngine {
    pub pow_config: PowConfig,
    pub poj_config: PojConfig,
}

impl ScoringEngine {
    pub fn new() -> Self {
        Self {
            pow_config: PowConfig::default(),
            poj_config: PojConfig::default(),
        }
    }

    /// Compute PoW score for an actor.
    pub fn compute_pow(
        &self,
        tasks_completed: u32,
        success_rate: f64,
        complexity: f64,
        energy_joules: f64,
        latency_seconds: f64,
    ) -> f64 {
        if tasks_completed == 0 || energy_joules <= 0.0 || latency_seconds <= 0.0 {
            return 0.0;
        }

        let numerator = tasks_completed as f64 * success_rate * complexity;
        let denominator = energy_joules.powf(self.pow_config.alpha)
            * (1.0 + latency_seconds).ln().powf(self.pow_config.beta);

        if denominator <= 0.0 {
            return 0.0;
        }

        numerator / denominator
    }

    /// Consensus confidence modifier from Shannon entropy of verdict buckets.
    ///
    /// Wraps `cynic_protocol::information::consensus_confidence_modifier` so
    /// the market surface consumes the canonical primitive rather than
    /// recomputing it locally.
    pub fn poj_consensus_confidence(
        &self,
        verdict_counts: &[usize],
        buckets: usize,
    ) -> Result<f64, cynic_protocol::information::EntropyError> {
        cynic_protocol::information::consensus_confidence_modifier(verdict_counts, buckets)
    }
}

impl Default for ScoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow_zero_for_no_tasks() {
        let engine = ScoringEngine::new();
        assert_eq!(engine.compute_pow(0, 0.0, 0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn pow_increases_with_success() {
        let engine = ScoringEngine::new();
        let low = engine.compute_pow(10, 0.5, 0.5, 100.0, 10.0);
        let high = engine.compute_pow(10, 0.9, 0.5, 100.0, 10.0);
        assert!(high > low);
    }

    #[test]
    fn pow_decreases_with_energy() {
        let engine = ScoringEngine::new();
        let low_energy = engine.compute_pow(10, 0.8, 0.5, 50.0, 10.0);
        let high_energy = engine.compute_pow(10, 0.8, 0.5, 200.0, 10.0);
        assert!(low_energy > high_energy);
    }

    #[test]
    fn poj_consensus_confidence_delegates_to_canonical_primitive() {
        let engine = ScoringEngine::new();
        let counts = [3usize, 0, 0];
        assert_eq!(
            engine.poj_consensus_confidence(&counts, 3).unwrap(),
            cynic_protocol::information::consensus_confidence_modifier(&counts, 3).unwrap()
        );
    }
}
