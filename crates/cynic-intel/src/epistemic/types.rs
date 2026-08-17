//! Bounded, non-authoritative epistemic-consensus data types.
//!
//! A verdict is a contextual estimate from supplied validator inputs. It is not
//! runtime truth, settlement authority, or proof of an external fact.

use serde::{Deserialize, Serialize};

pub const EPISTEMIC_VERDICT_SCHEMA_V1: &str = "cynic.epistemic_verdict.v1";
pub const CRYSTAL_SCHEMA_V1: &str = "cynic.crystal.v1";
pub const MAX_SCORE_BPS: u32 = 10_000;
pub const PHI_INV_BPS: u32 = 6_180;
pub const PHI_INV_SQ_BPS: u32 = 3_820;
pub const PHI_INV_CUBE_BPS: u32 = 2_361;
pub const HOWL_THRESHOLD_BPS: u32 = 5_279;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxiomType {
    Fidelity,
    Phi,
    Verify,
    Culture,
    Burn,
    Sovereignty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictRating {
    Howl,
    Wag,
    Growl,
    Bark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxiomScoresBps {
    pub fidelity: u32,
    pub phi: u32,
    pub verify: u32,
    pub culture: u32,
    pub burn: u32,
    pub sovereignty: u32,
}

impl AxiomScoresBps {
    pub fn all_within_bounds(&self) -> bool {
        [
            self.fidelity,
            self.phi,
            self.verify,
            self.culture,
            self.burn,
            self.sovereignty,
        ]
        .into_iter()
        .all(|score| score <= MAX_SCORE_BPS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorDogScore {
    pub dog_name: String,
    pub scores: AxiomScoresBps,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicVerdict {
    pub schema: String,
    pub verdict_id: String,
    pub stimulus_id: String,
    pub domain: String,
    pub rating: VerdictRating,
    pub q_score_bps: u32,
    pub consensus_axioms: AxiomScoresBps,
    pub participating_dogs_count: usize,
    pub anomaly_detected: bool,
    pub timestamp: i64,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrystalState {
    Observation,
    Crystallized,
    Canonical,
    Decaying,
    Dissolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrystalKnowledge {
    pub schema: String,
    pub crystal_id: String,
    pub domain: String,
    pub pattern_digest: String,
    pub observation_count: u32,
    pub confidence_bps: u32,
    pub state: CrystalState,
    pub first_observed_at: i64,
    pub last_observed_at: i64,
}
