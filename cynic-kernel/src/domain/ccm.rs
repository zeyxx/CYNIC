//! CCM — Cognitive Crystallization Mechanism.
//! Ephemeral verdicts → persistent wisdom. Domain-pure logic.
//!
//! A pattern that scores >= φ⁻¹ (0.618) repeatedly across 21+ cycles
//! crystallizes into persistent wisdom. Below φ⁻² (0.382) it decays.
//! This is how CYNIC learns without training — through phi-bounded consensus.

use serde::{Deserialize, Serialize};
use crate::domain::dog::{PHI_INV, PHI_INV2};

/// Fibonacci F(8) = 21 — minimum observations before crystallization.
pub const MIN_CRYSTALLIZATION_CYCLES: u32 = 21;
/// Fibonacci F(13) = 233 — canonical status (deeply crystallized).
pub const CANONICAL_CYCLES: u32 = 233;

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
    /// Running mean of Q-Score totals that contributed
    pub confidence: f64,
    /// Number of concordant observations
    pub observations: u32,
    /// Current state in the crystallization lifecycle
    pub state: CrystalState,
    /// Timestamp of creation
    pub created_at: String,
    /// Timestamp of last update
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrystalState {
    /// Accumulating observations, not yet crystallized (< 21 cycles)
    Forming,
    /// Reached crystallization threshold (>= 21 cycles, confidence >= φ⁻¹)
    Crystallized,
    /// Reached canonical status (>= 233 cycles, confidence >= φ⁻¹)
    Canonical,
    /// Confidence dropped below φ⁻² — decaying
    Decaying,
    /// Fully dissolved — no longer valid wisdom
    Dissolved,
}

// ── CRYSTALLIZATION ENGINE (pure domain logic) ──────────────

/// Observe a new Q-Score for a pattern. Returns the updated crystal state.
/// This is the core CCM algorithm — stateless, pure function.
pub fn observe(crystal: &Crystal, new_score: f64) -> CrystalState {
    let next_obs = crystal.observations + 1;
    let next_confidence = running_mean(crystal.confidence, crystal.observations, new_score);

    classify(next_confidence, next_obs)
}

/// Floating-point tolerance for phi-threshold comparisons.
const EPSILON: f64 = 1e-10;

/// Classify a crystal based on its confidence and observation count.
fn classify(confidence: f64, observations: u32) -> CrystalState {
    if confidence < PHI_INV2 - EPSILON {
        if observations > MIN_CRYSTALLIZATION_CYCLES {
            CrystalState::Decaying
        } else {
            CrystalState::Dissolved
        }
    } else if observations >= CANONICAL_CYCLES && confidence >= PHI_INV - EPSILON {
        CrystalState::Canonical
    } else if observations >= MIN_CRYSTALLIZATION_CYCLES && confidence >= PHI_INV - EPSILON {
        CrystalState::Crystallized
    } else {
        CrystalState::Forming
    }
}

/// Update a crystal with a new observation. Returns the new crystal.
/// Pure function — caller is responsible for persistence.
pub fn update_crystal(crystal: &Crystal, new_score: f64, timestamp: &str) -> Crystal {
    let observations = crystal.observations + 1;
    let confidence = running_mean(crystal.confidence, crystal.observations, new_score);
    let state = classify(confidence, observations);

    Crystal {
        id: crystal.id.clone(),
        content: crystal.content.clone(),
        domain: crystal.domain.clone(),
        confidence,
        observations,
        state,
        created_at: crystal.created_at.clone(),
        updated_at: timestamp.to_string(),
    }
}

/// Create a new crystal from a first observation.
pub fn new_crystal(id: String, content: String, domain: String, initial_score: f64, timestamp: &str) -> Crystal {
    Crystal {
        id,
        content,
        domain,
        confidence: initial_score,
        observations: 1,
        state: CrystalState::Forming,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
    }
}

/// Incremental running mean: avoids storing all historical values.
fn running_mean(current_mean: f64, count: u32, new_value: f64) -> f64 {
    if count == 0 {
        return new_value;
    }
    let n = count as f64;
    (current_mean * n + new_value) / (n + 1.0)
}

// ── CCM FEEDBACK — inject crystallized wisdom into stimulus context ──

/// Format mature crystals as context for Dog prompts.
/// Only includes Crystallized and Canonical crystals from the same domain.
/// Token-budget-aware: caps at max_chars to avoid overflowing small models.
pub fn format_crystal_context(crystals: &[Crystal], domain: &str, max_chars: usize) -> Option<String> {
    let mature: Vec<&Crystal> = crystals.iter()
        .filter(|c| c.domain == domain || domain == "general")
        .filter(|c| matches!(c.state, CrystalState::Crystallized | CrystalState::Canonical))
        .collect();

    if mature.is_empty() {
        return None;
    }

    // Sort by confidence descending — highest-value crystals first (agentkeeper pattern)
    let mut sorted = mature;
    sorted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

    let mut lines = Vec::new();
    let mut total_chars = 0;
    let header = format!("[CYNIC Memory — {} crystallized patterns for domain '{}']", sorted.len(), domain);
    total_chars += header.len();
    lines.push(header);

    for c in sorted {
        let state_label = if c.state == CrystalState::Canonical { "CANONICAL" } else { "CRYSTALLIZED" };
        let line = format!("- [{}] (confidence: {:.2}, {} observations): {}", state_label, c.confidence, c.observations, c.content);
        if total_chars + line.len() > max_chars {
            break; // Token budget exhausted
        }
        total_chars += line.len();
        lines.push(line);
    }

    if lines.len() <= 1 {
        return None; // Only header, no crystals fit
    }

    Some(lines.join("\n"))
}

// ── CONTENT HASHING ─────────────────────────────────────────
/// Deterministic content hash for crystal IDs. FNV-1a — not cryptographic,
/// just stable content addressing. Used by both REST and MCP paths.
pub fn content_hash(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    h
}

// ── CRYSTALLIZATION PORT ────────────────────────────────────
/// Domain contract for crystal persistence. Adapter implements this.
#[async_trait::async_trait]
pub trait CrystallizationPort: Send + Sync {
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), CrystalError>;
    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, CrystalError>;
    async fn find_by_content(&self, content: &str, domain: &str) -> Result<Option<Crystal>, CrystalError>;
    async fn list_crystallized(&self, limit: u32) -> Result<Vec<Crystal>, CrystalError>;
}

#[derive(Debug)]
pub enum CrystalError {
    StorageFailed(String),
    NotFound(String),
}

impl std::fmt::Display for CrystalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageFailed(m) => write!(f, "Crystal storage failed: {}", m),
            Self::NotFound(m) => write!(f, "Crystal not found: {}", m),
        }
    }
}

// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

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
        }
    }

    #[test]
    fn new_crystal_starts_forming() {
        let c = new_crystal("x".into(), "test".into(), "chess".into(), 0.5, "now");
        assert_eq!(c.state, CrystalState::Forming);
        assert_eq!(c.observations, 1);
    }

    #[test]
    fn crystallizes_after_21_cycles_above_phi() {
        let c = make_crystal(PHI_INV, MIN_CRYSTALLIZATION_CYCLES - 1, CrystalState::Forming);
        let state = observe(&c, PHI_INV);
        assert_eq!(state, CrystalState::Crystallized);
    }

    #[test]
    fn does_not_crystallize_below_threshold() {
        let c = make_crystal(0.5, MIN_CRYSTALLIZATION_CYCLES - 1, CrystalState::Forming);
        let state = observe(&c, 0.5);
        // 0.5 < PHI_INV (0.618), so stays Forming
        assert_eq!(state, CrystalState::Forming);
    }

    #[test]
    fn decays_when_confidence_drops() {
        let c = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
        // Feed many low scores to drag confidence below PHI_INV2
        let mut crystal = c;
        for _ in 0..100 {
            crystal = update_crystal(&crystal, 0.1, "now");
        }
        assert!(crystal.confidence < PHI_INV2);
        assert_eq!(crystal.state, CrystalState::Decaying);
    }

    #[test]
    fn canonical_after_233_cycles() {
        let c = make_crystal(PHI_INV, CANONICAL_CYCLES - 1, CrystalState::Crystallized);
        let state = observe(&c, PHI_INV);
        assert_eq!(state, CrystalState::Canonical);
    }

    #[test]
    fn running_mean_correct() {
        // Mean of [0.5, 0.7] = 0.6
        let mean = running_mean(0.5, 1, 0.7);
        assert!((mean - 0.6).abs() < 1e-10);
    }

    #[test]
    fn running_mean_first_value() {
        let mean = running_mean(0.0, 0, 0.42);
        assert!((mean - 0.42).abs() < 1e-10);
    }

    #[test]
    fn geometric_mean_drag_prevents_false_crystallization() {
        // A pattern that oscillates between high and low should NOT crystallize
        let mut c = new_crystal("osc".into(), "oscillating".into(), "test".into(), 0.9, "now");
        for i in 0..30 {
            let score = if i % 2 == 0 { 0.9 } else { 0.2 };
            c = update_crystal(&c, score, "now");
        }
        // Mean of alternating 0.9/0.2 ≈ 0.55, below PHI_INV
        assert!(c.confidence < PHI_INV);
        assert_ne!(c.state, CrystalState::Crystallized);
    }

    #[test]
    fn crystal_context_includes_mature_only() {
        let crystals = vec![
            make_crystal(PHI_INV, 25, CrystalState::Crystallized),
            make_crystal(0.3, 5, CrystalState::Forming),
            make_crystal(PHI_INV, 250, CrystalState::Canonical),
        ];
        let ctx = format_crystal_context(&crystals, "test", 2000).unwrap();
        assert!(ctx.contains("CRYSTALLIZED"));
        assert!(ctx.contains("CANONICAL"));
        assert!(!ctx.contains("Forming")); // Forming excluded
    }

    #[test]
    fn crystal_context_respects_budget() {
        let crystals: Vec<Crystal> = (0..100).map(|i| {
            let mut c = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
            c.content = format!("pattern number {} with some extra text to fill space", i);
            c
        }).collect();
        let ctx = format_crystal_context(&crystals, "test", 200).unwrap();
        assert!(ctx.len() <= 300); // Some slack for header
    }

    #[test]
    fn crystal_context_empty_when_no_mature() {
        let crystals = vec![
            make_crystal(0.3, 5, CrystalState::Forming),
        ];
        assert!(format_crystal_context(&crystals, "test", 2000).is_none());
    }

    #[test]
    fn phi_bounds_are_consistent() {
        // Verify our thresholds match the phi constants
        assert!(PHI_INV > PHI_INV2);
        assert!((PHI_INV - 0.618).abs() < 0.001);
        assert!((PHI_INV2 - 0.382).abs() < 0.001);
    }
}
