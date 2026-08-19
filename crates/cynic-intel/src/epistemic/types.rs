use serde::{Deserialize, Serialize};

pub const EPISTEMIC_VERDICT_SCHEMA_V1: &str = "cynic.epistemic_verdict.v1";
pub const CRYSTAL_SCHEMA_V1: &str = "cynic.crystal.v1";

/// Constante d'Or : φ⁻¹ = 61.8034% (en basis points, 10 000 bps = 100%)
pub const PHI_INV_BPS: u32 = 6180; // 0.618034
/// Seuil d'anomalie : φ⁻² = 38.1966%
pub const PHI_INV_SQ_BPS: u32 = 3820; // 0.381966
/// Seuil Growl : φ⁻³ = 23.6068%
pub const PHI_INV_CUBE_BPS: u32 = 2361; // 0.236068
/// Seuil Howl : φ⁻² + φ⁻⁴ = 52.7864%
pub const HOWL_THRESHOLD_BPS: u32 = 5279; // 0.527864

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
    Howl,  // > 5279 bps (Exceptionnel)
    Wag,   // > 3820 bps (Bon)
    Growl, // > 2361 bps (Douteux)
    Bark,  // <= 2361 bps (Rejeté)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorDogScore {
    pub dog_name: String,
    pub scores: AxiomScoresBps,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicVerdict {
    pub schema: &'static str,
    pub verdict_id: String,
    pub stimulus_id: String,
    pub domain: String,
    pub rating: VerdictRating,
    pub q_score_bps: u32, // Borne strictement à PHI_INV_BPS (6180)
    pub consensus_axioms: AxiomScoresBps,
    pub participating_dogs_count: usize,
    pub anomaly_detected: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrystalState {
    Observation, // < 21 observations
    Crystallized, // >= 21 observations + confidence >= φ⁻¹
    Canonical,    // >= 233 observations (Fibonacci F13)
    Decaying,     // confidence < φ⁻²
    Dissolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrystalKnowledge {
    pub schema: &'static str,
    pub crystal_id: String,
    pub domain: String,
    pub pattern_digest: String,
    pub observation_count: u32,
    pub confidence_bps: u32,
    pub state: CrystalState,
    pub first_observed_at: i64,
    pub last_observed_at: i64,
}
