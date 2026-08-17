use super::types::{
    CRYSTAL_SCHEMA_V1, CrystalKnowledge, CrystalState, MAX_SCORE_BPS, PHI_INV_BPS,
    PHI_INV_CUBE_BPS, PHI_INV_SQ_BPS,
};

pub const FIBONACCI_CRYSTALLIZE_THRESHOLD: u32 = 21;
pub const FIBONACCI_CANONICAL_THRESHOLD: u32 = 233;

impl CrystalKnowledge {
    pub fn new(crystal_id: &str, domain: &str, pattern_digest: &str, now: i64) -> Self {
        Self {
            schema: CRYSTAL_SCHEMA_V1.to_string(),
            crystal_id: crystal_id.to_string(),
            domain: domain.to_string(),
            pattern_digest: pattern_digest.to_string(),
            observation_count: 0,
            confidence_bps: 0,
            state: CrystalState::Observation,
            first_observed_at: now,
            last_observed_at: now,
        }
    }

    pub fn record_observation(&mut self, q_score_bps: u32, timestamp: i64) -> Result<(), String> {
        if q_score_bps > MAX_SCORE_BPS {
            return Err("q-score exceeds the BPS range".to_string());
        }
        if timestamp < self.last_observed_at {
            return Err("observation timestamp moved backwards".to_string());
        }

        self.observation_count = self
            .observation_count
            .checked_add(1)
            .ok_or_else(|| "observation count overflowed".to_string())?;
        self.last_observed_at = timestamp;
        self.confidence_bps = if self.confidence_bps == 0 {
            q_score_bps
        } else {
            ((u64::from(self.confidence_bps) * 8 + u64::from(q_score_bps) * 2) / 10) as u32
        };
        self.update_state();
        Ok(())
    }

    fn update_state(&mut self) {
        self.state = if self.confidence_bps < PHI_INV_CUBE_BPS
            && self.observation_count >= FIBONACCI_CRYSTALLIZE_THRESHOLD
        {
            CrystalState::Dissolved
        } else if self.confidence_bps < PHI_INV_SQ_BPS {
            CrystalState::Decaying
        } else if self.observation_count >= FIBONACCI_CANONICAL_THRESHOLD
            && self.confidence_bps >= PHI_INV_BPS
        {
            CrystalState::Canonical
        } else if self.observation_count >= FIBONACCI_CRYSTALLIZE_THRESHOLD
            && self.confidence_bps >= PHI_INV_BPS
        {
            CrystalState::Crystallized
        } else {
            CrystalState::Observation
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_requires_monotonic_bounded_observations() {
        let mut crystal = CrystalKnowledge::new("crystal-1", "simulation", "digest", 10);
        for timestamp in 10..31 {
            crystal
                .record_observation(PHI_INV_BPS, timestamp)
                .expect("valid observation");
        }
        assert_eq!(crystal.state, CrystalState::Crystallized);
        assert!(crystal.record_observation(MAX_SCORE_BPS + 1, 32).is_err());
        assert!(crystal.record_observation(PHI_INV_BPS, 9).is_err());
    }
}
