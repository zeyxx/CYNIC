use super::types::{
    CrystalKnowledge, CrystalState, CRYSTAL_SCHEMA_V1, PHI_INV_BPS, PHI_INV_CUBE_BPS,
    PHI_INV_SQ_BPS,
};

pub const FIBONACCI_CRYSTALLIZE_THRESHOLD: u32 = 21; // F8
pub const FIBONACCI_CANONICAL_THRESHOLD: u32 = 233;   // F13

impl CrystalKnowledge {
    pub fn new(crystal_id: &str, domain: &str, pattern_digest: &str, now: i64) -> Self {
        Self {
            schema: CRYSTAL_SCHEMA_V1,
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

    pub fn record_observation(&mut self, q_score_bps: u32, timestamp: i64) {
        self.observation_count += 1;
        self.last_observed_at = timestamp;

        // Moyenne mobile exponentielle de la confiance
        if self.confidence_bps == 0 {
            self.confidence_bps = q_score_bps;
        } else {
            self.confidence_bps = ((self.confidence_bps as u64 * 8 + q_score_bps as u64 * 2) / 10) as u32;
        }

        self.update_state();
    }

    fn update_state(&mut self) {
        if self.confidence_bps < PHI_INV_CUBE_BPS && self.observation_count >= FIBONACCI_CRYSTALLIZE_THRESHOLD {
            self.state = CrystalState::Dissolved;
        } else if self.confidence_bps < PHI_INV_SQ_BPS {
            self.state = CrystalState::Decaying;
        } else if self.observation_count >= FIBONACCI_CANONICAL_THRESHOLD && self.confidence_bps >= PHI_INV_BPS {
            self.state = CrystalState::Canonical;
        } else if self.observation_count >= FIBONACCI_CRYSTALLIZE_THRESHOLD && self.confidence_bps >= PHI_INV_BPS {
            self.state = CrystalState::Crystallized;
        } else {
            self.state = CrystalState::Observation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crystal_fibonacci_lifecycle_transitions() {
        let mut crystal = CrystalKnowledge::new("cst-001", "chess", "hash-sicilian-dragon", 1_720_000_000);
        assert_eq!(crystal.state, CrystalState::Observation);

        // 20 observations parfaites (6180 bps) -> Toujours en Observation
        for i in 1..=20 {
            crystal.record_observation(PHI_INV_BPS, 1_720_000_000 + i * 100);
        }
        assert_eq!(crystal.observation_count, 20);
        assert_eq!(crystal.state, CrystalState::Observation);

        // 21e observation (Fibonacci F8) -> Transition vers Crystallized !
        crystal.record_observation(PHI_INV_BPS, 1_720_002_100);
        assert_eq!(crystal.observation_count, 21);
        assert_eq!(crystal.state, CrystalState::Crystallized);
    }
}
