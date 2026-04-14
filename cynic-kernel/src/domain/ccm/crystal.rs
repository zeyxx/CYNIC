//! Crystal types — the persistent wisdom unit of CCM.
//!
//! Pure types: no logic, no I/O, no external dependencies beyond serde.

use serde::{Deserialize, Serialize};

/// Fibonacci F(8) = 21 — minimum observations before crystallization.
/// Single source of truth — used by both SurrealDB SQL and InMemory adapter.
pub const MIN_CRYSTALLIZATION_CYCLES: u32 = 21;
/// Fibonacci F(13) = 233 — canonical status (deeply crystallized).
pub const CANONICAL_CYCLES: u32 = 233;
/// Max provenance entries per crystal. Prevents unbounded Vec growth on long-lived crystals.
pub const MAX_CONTRIBUTING_VERDICTS: usize = 500;

// ── CRYSTAL ─────────────────────────────────────────────────
/// A crystallized truth — persistent wisdom extracted from ephemeral verdicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crystal {
    /// Unique identifier
    pub id: String,
    /// The crystallized insight (domain-agnostic)
    pub content: String,
    /// Domain this was crystallized from (e.g. "chess", "code", "general")
    pub domain: String,
    /// Running mean of normalized Q-Score totals (= quality dimension)
    pub confidence: f64,
    /// Number of concordant observations
    pub observations: u32,
    /// Current state in the crystallization lifecycle
    pub state: CrystalState,
    /// Timestamp of creation
    pub created_at: String,
    /// Timestamp of last update
    pub updated_at: String,
    /// Verdict IDs that contributed observations to this crystal (provenance trail).
    /// Enables auditing: "why did this crystal crystallize?" → trace back to verdicts.
    #[serde(default)]
    pub contributing_verdicts: Vec<String>,

    // ── 4D Crystal Model ──────────────────────────────────────
    // Quality = confidence (above). Certainty + polarity + time below.
    /// Statistical certainty: concordance(Welford) × volume(obs/21).
    /// Crystallization gate uses this, not confidence. Allows negative truths to crystallize.
    #[serde(default)]
    pub certainty: f64,
    /// Welford running M2 — internal state for incremental variance.
    /// stddev = sqrt(variance_m2 / (observations - 1)) when observations > 1.
    #[serde(default)]
    pub variance_m2: f64,
    /// Running mean of voter_count across observations (observability, not gating).
    #[serde(default)]
    pub mean_quorum: f64,
    /// HOWL verdict count — for polarity tracking.
    #[serde(default)]
    pub howl_count: u32,
    /// WAG verdict count.
    #[serde(default)]
    pub wag_count: u32,
    /// GROWL verdict count.
    #[serde(default)]
    pub growl_count: u32,
    /// BARK verdict count.
    #[serde(default)]
    pub bark_count: u32,
}

impl Crystal {
    /// Dominant verdict polarity — argmax of the 4 verdict counters.
    /// Returns "HOWL"/"WAG"/"GROWL"/"BARK", or "UNKNOWN" if no observations.
    pub fn dominant_polarity(&self) -> &'static str {
        let counts = [
            (self.howl_count, "HOWL"),
            (self.wag_count, "WAG"),
            (self.growl_count, "GROWL"),
            (self.bark_count, "BARK"),
        ];
        counts
            .iter()
            .max_by_key(|(count, _)| *count)
            .filter(|(count, _)| *count > 0)
            .map(|(_, name)| *name)
            .unwrap_or("UNKNOWN")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrystalState {
    /// Accumulating observations, not yet crystallized (< 21 cycles)
    Forming,
    /// Reached certainty threshold (>= 21 cycles, certainty >= phi-inverse)
    Crystallized,
    /// Reached canonical status (>= 233 cycles, certainty >= phi-inverse)
    Canonical,
    /// Certainty dropped below phi-inverse-squared — decaying
    Decaying,
    /// Fully dissolved — no longer valid wisdom
    Dissolved,
}

impl std::fmt::Display for CrystalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forming => write!(f, "forming"),
            Self::Crystallized => write!(f, "crystallized"),
            Self::Canonical => write!(f, "canonical"),
            Self::Decaying => write!(f, "decaying"),
            Self::Dissolved => write!(f, "dissolved"),
        }
    }
}

impl std::str::FromStr for CrystalState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forming" => Ok(Self::Forming),
            "crystallized" => Ok(Self::Crystallized),
            "canonical" => Ok(Self::Canonical),
            "decaying" => Ok(Self::Decaying),
            "dissolved" => Ok(Self::Dissolved),
            other => Err(format!("unknown crystal state: {other}")),
        }
    }
}

// ── MATURE CRYSTAL NEWTYPE (T4) ─────────────────────────────
/// A crystal that has reached Crystallized or Canonical state.
/// Private inner field — can only be constructed via `TryFrom<Crystal>`,
/// which validates the state at compile-time boundary. This prevents
/// Forming/Decaying/Dissolved crystals from reaching Dog prompts.
#[derive(Debug, Clone)]
pub struct MatureCrystal {
    inner: Crystal,
}

/// Error when attempting to create a MatureCrystal from a non-mature Crystal.
#[derive(Debug)]
pub struct NotMatureError {
    pub id: String,
    pub state: CrystalState,
}

impl std::fmt::Display for NotMatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "crystal '{}' is {:?}, not mature", self.id, self.state)
    }
}

impl std::error::Error for NotMatureError {}

impl TryFrom<Crystal> for MatureCrystal {
    type Error = NotMatureError;

    fn try_from(crystal: Crystal) -> Result<Self, Self::Error> {
        match crystal.state {
            CrystalState::Crystallized | CrystalState::Canonical => {
                Ok(MatureCrystal { inner: crystal })
            }
            _ => Err(NotMatureError {
                id: crystal.id.clone(),
                state: crystal.state,
            }),
        }
    }
}

impl MatureCrystal {
    /// Access the underlying Crystal (read-only).
    pub fn crystal(&self) -> &Crystal {
        &self.inner
    }

    pub fn id(&self) -> &str {
        &self.inner.id
    }

    pub fn content(&self) -> &str {
        &self.inner.content
    }

    pub fn domain(&self) -> &str {
        &self.inner.domain
    }

    pub fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    pub fn observations(&self) -> u32 {
        self.inner.observations
    }

    pub fn state(&self) -> &CrystalState {
        &self.inner.state
    }

    pub fn updated_at(&self) -> &str {
        &self.inner.updated_at
    }

    pub fn certainty(&self) -> f64 {
        self.inner.certainty
    }

    pub fn dominant_polarity(&self) -> &'static str {
        self.inner.dominant_polarity()
    }
}

/// Filter a list of Crystals into only mature ones.
/// Convenience function — equivalent to try_from + filter_map.
pub fn filter_mature(crystals: Vec<Crystal>) -> Vec<MatureCrystal> {
    crystals
        .into_iter()
        .filter_map(|c| MatureCrystal::try_from(c).ok())
        .collect()
}

// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::PHI_INV;

    fn make_crystal(confidence: f64, observations: u32, state: CrystalState) -> Crystal {
        Crystal {
            id: "test-1".into(),
            content: "test pattern".into(),
            domain: "test".into(),
            confidence,
            observations,
            state,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
        }
    }

    #[test]
    fn mature_crystal_rejects_forming() {
        let forming = make_crystal(0.5, 5, CrystalState::Forming);
        assert!(MatureCrystal::try_from(forming).is_err());
    }

    #[test]
    fn mature_crystal_accepts_crystallized() {
        let crystallized = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
        assert!(MatureCrystal::try_from(crystallized).is_ok());
    }

    #[test]
    fn mature_crystal_accepts_canonical() {
        let canonical = make_crystal(PHI_INV, 250, CrystalState::Canonical);
        assert!(MatureCrystal::try_from(canonical).is_ok());
    }

    #[test]
    fn mature_crystal_rejects_decaying() {
        let decaying = make_crystal(0.2, 30, CrystalState::Decaying);
        assert!(MatureCrystal::try_from(decaying).is_err());
    }

    #[test]
    fn crystal_state_display_is_lowercase() {
        assert_eq!(CrystalState::Forming.to_string(), "forming");
        assert_eq!(CrystalState::Crystallized.to_string(), "crystallized");
        assert_eq!(CrystalState::Canonical.to_string(), "canonical");
        assert_eq!(CrystalState::Decaying.to_string(), "decaying");
        assert_eq!(CrystalState::Dissolved.to_string(), "dissolved");
    }

    #[test]
    fn crystal_state_round_trip() {
        for state in [
            CrystalState::Forming,
            CrystalState::Crystallized,
            CrystalState::Canonical,
            CrystalState::Decaying,
            CrystalState::Dissolved,
        ] {
            let s = state.to_string();
            let parsed: CrystalState = s.parse().unwrap();
            assert_eq!(parsed, state);
        }
    }

    #[test]
    fn phi_bounds_are_consistent() {
        use crate::domain::dog::PHI_INV2;
        let phi_inv = PHI_INV;
        let phi_inv2 = PHI_INV2;
        assert!(
            (phi_inv - 0.618).abs() < 0.001,
            "PHI_INV should be ~0.618, got {phi_inv}"
        );
        assert!(
            (phi_inv2 - 0.382).abs() < 0.001,
            "PHI_INV2 should be ~0.382, got {phi_inv2}"
        );
        assert!(
            phi_inv > phi_inv2,
            "PHI_INV ({phi_inv}) must be > PHI_INV2 ({phi_inv2})"
        );
    }
}
