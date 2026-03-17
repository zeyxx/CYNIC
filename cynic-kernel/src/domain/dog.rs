//! Dog — the model-agnostic evaluator interface.
//! Any intelligence source implements this trait.
//! The Dog receives a Stimulus, returns AxiomScores.
//! The kernel phi-bounds and aggregates — the Dog never self-caps.

use serde::{Deserialize, Serialize};

// ── PHI CONSTANTS ──────────────────────────────────────────
pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_INV: f64 = 0.618_033_988_749_895; // max confidence
pub const PHI_INV2: f64 = 0.381_966_011_250_105; // anomaly threshold

// ── STIMULUS ───────────────────────────────────────────────
/// What the organism perceives. Domain-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stimulus {
    /// What to evaluate (chess move, code, statement, anything)
    pub content: String,
    /// Optional context (board state, file contents, conversation)
    pub context: Option<String>,
    /// Domain hint for weight adjustment (e.g. "chess", "code", "geopolitics")
    pub domain: Option<String>,
}

// ── AXIOM SCORES ───────────────────────────────────────────
/// Raw scores from a Dog. NOT phi-bounded — the kernel does that.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AxiomScores {
    /// Truth loyalty: is this faithful to reality? (kenosis)
    pub fidelity: f64,
    /// Structural harmony: is this well-proportioned? (golden ratio)
    pub phi: f64,
    /// Evidence + falsifiability: is this verifiable/falsifiable? (Popper)
    pub verify: f64,
    /// Continuity + patterns: does this honor existing lineage?
    pub culture: f64,
    /// Simplicity + action: is this minimal and efficient? (destroy excess)
    pub burn: f64,
    /// Individual agency: does this preserve sovereignty? (the soul of CYNIC)
    pub sovereignty: f64,
    /// Optional reasoning per axiom
    pub reasoning: AxiomReasoning,
    /// Token usage from inference (0 for deterministic/local)
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxiomReasoning {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
    pub culture: String,
    pub burn: String,
    pub sovereignty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DogScore {
    pub dog_id: String,
    pub latency_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
    pub reasoning: AxiomReasoning,
}

// ── PHI-BOUNDED Q-SCORE ────────────────────────────────────
/// The kernel's final score. Every value capped at phi^-1 (0.618).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QScore {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

// ── VERDICT ────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerdictKind {
    Howl,   // >= 0.82 raw (phi-bounded: high conviction)
    Wag,    // >= 0.618 raw (positive)
    Growl,  // >= 0.382 raw (cautious)
    Bark,   // < 0.382 (rejection / insufficient confidence)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub id: String,
    pub kind: VerdictKind,
    pub q_score: QScore,
    pub reasoning: AxiomReasoning,
    pub dog_id: String,
    pub stimulus_summary: String,
    pub timestamp: String,
    #[serde(default)]
    pub dog_scores: Vec<DogScore>,
    #[serde(default)]
    pub anomaly_detected: bool,
    #[serde(default)]
    pub max_disagreement: f64,
    #[serde(default)]
    pub anomaly_axiom: Option<String>,
    /// BLAKE3 hash of this verdict's content (L1 integrity)
    #[serde(default)]
    pub integrity_hash: Option<String>,
    /// BLAKE3 hash of the previous verdict (hash chain)
    #[serde(default)]
    pub prev_hash: Option<String>,
}

// ── PHI-BOUNDING ───────────────────────────────────────────
/// Clamp a raw score to [FLOOR, phi^-1]. Floor at 0.05 because a true zero
/// is always a parsing failure, never a real epistemic judgment. A zero in the
/// arithmetic mean drags consensus unfairly; 0.05 preserves the "near-zero = terrible"
/// signal without mathematical pathology.
const SCORE_FLOOR: f64 = 0.05;
pub fn phi_bound(raw: f64) -> f64 {
    raw.clamp(SCORE_FLOOR, PHI_INV)
}

/// Compute phi-bounded Q-Score from raw axiom scores.
/// Uses geometric mean of 6 axioms — one weak axiom drags everything down.
/// Q = ⁶√(F × Φ × V × C × B × S), then phi-bounded.
pub fn compute_qscore(raw: &AxiomScores) -> QScore {
    let f = phi_bound(raw.fidelity);
    let p = phi_bound(raw.phi);
    let v = phi_bound(raw.verify);
    let c = phi_bound(raw.culture);
    let b = phi_bound(raw.burn);
    let s = phi_bound(raw.sovereignty);

    // Geometric mean of 6 axioms, then phi-bound the result
    let geo = (f * p * v * c * b * s).powf(1.0 / 6.0);
    let total = phi_bound(geo);

    QScore { total, fidelity: f, phi: p, verify: v, culture: c, burn: b, sovereignty: s }
}

/// Determine verdict from Q-Score total
pub fn verdict_kind(total: f64) -> VerdictKind {
    // These thresholds are on the phi-bounded scale (0..0.618)
    // Map: HOWL > 0.507 (82% of 0.618), WAG > 0.382, GROWL > 0.236, BARK below
    if total > PHI_INV * 0.82 { VerdictKind::Howl }
    else if total > PHI_INV2 { VerdictKind::Wag }
    else if total > PHI_INV2 * PHI_INV { VerdictKind::Growl }
    else { VerdictKind::Bark }
}

// ── DOG TRAIT ──────────────────────────────────────────────
/// The contract every evaluator must fulfill.
/// Model-agnostic: Gemini, Llama, GPT, deterministic code — all identical to the caller.
#[async_trait::async_trait]
pub trait Dog: Send + Sync {
    /// Unique identifier for this Dog
    fn id(&self) -> &str;

    /// Max context tokens this Dog supports. 0 = unlimited.
    fn max_context(&self) -> u32 { 0 }

    /// Evaluate a stimulus, return raw axiom scores (NOT phi-bounded)
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError>;
}

/// Estimate token count from text. Heuristic: ~4 chars per token for English.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as f64 / 4.0).ceil() as u32
}

#[derive(Debug)]
pub enum DogError {
    /// Model API returned an error
    ApiError(String),
    /// Response couldn't be parsed into axiom scores
    ParseError(String),
    /// Rate limited
    RateLimited(String),
    /// Request timed out
    Timeout,
}

impl std::fmt::Display for DogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiError(msg) => write!(f, "Dog API error: {}", msg),
            Self::ParseError(msg) => write!(f, "Dog parse error: {}", msg),
            Self::RateLimited(msg) => write!(f, "Dog rate limited: {}", msg),
            Self::Timeout => write!(f, "Dog evaluation timed out"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_bound_clamps_high_values() {
        assert!((phi_bound(0.95) - PHI_INV).abs() < 1e-10);
        assert!((phi_bound(1.0) - PHI_INV).abs() < 1e-10);
    }

    #[test]
    fn phi_bound_preserves_low_values() {
        assert!((phi_bound(0.3) - 0.3).abs() < 1e-10);
        // 0.0 is floored to SCORE_FLOOR (0.05) — a true zero is always a parsing failure
        assert!((phi_bound(0.0) - SCORE_FLOOR).abs() < 1e-10);
    }

    #[test]
    fn phi_bound_clamps_negative() {
        // Negative values are floored to SCORE_FLOOR
        assert!((phi_bound(-0.5) - SCORE_FLOOR).abs() < 1e-10);
    }

    #[test]
    fn qscore_geometric_mean_correct() {
        let raw = AxiomScores {
            fidelity: 0.6, phi: 0.5, verify: 0.4,
            culture: 0.5, burn: 0.5, sovereignty: 0.6,
            reasoning: AxiomReasoning::default(), ..Default::default()
        };
        let q = compute_qscore(&raw);
        assert!(q.total <= PHI_INV + 1e-10);
        assert!(q.fidelity <= PHI_INV + 1e-10);
        let expected = (0.6_f64 * 0.5 * 0.4 * 0.5 * 0.5 * 0.6).powf(1.0/6.0);
        assert!((q.total - expected).abs() < 0.01);
    }

    #[test]
    fn one_weak_axiom_drags_score_down() {
        let strong = AxiomScores {
            fidelity: 0.6, phi: 0.6, verify: 0.6,
            culture: 0.6, burn: 0.6, sovereignty: 0.6,
            reasoning: AxiomReasoning::default(), ..Default::default()
        };
        let weak = AxiomScores {
            fidelity: 0.6, phi: 0.6, verify: 0.6,
            culture: 0.6, burn: 0.6, sovereignty: 0.1,
            reasoning: AxiomReasoning::default(), ..Default::default()
        };
        let q_strong = compute_qscore(&strong);
        let q_weak = compute_qscore(&weak);
        // One weak axiom must significantly reduce total
        assert!(q_weak.total < q_strong.total * 0.8);
    }

    #[test]
    fn verdict_thresholds() {
        assert_eq!(verdict_kind(PHI_INV), VerdictKind::Howl);
        assert_eq!(verdict_kind(0.45), VerdictKind::Wag);
        assert_eq!(verdict_kind(0.25), VerdictKind::Growl);
        assert_eq!(verdict_kind(0.1), VerdictKind::Bark);
    }
}
