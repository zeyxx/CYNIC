//! Dog — the model-agnostic evaluator interface.
//! Any intelligence source implements this trait.
//! The Dog receives a Stimulus, returns AxiomScores.
//! The kernel phi-bounds and aggregates — the Dog never self-caps.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── PHI CONSTANTS ──────────────────────────────────────────
pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_INV: f64 = 0.618_033_988_749_895; // max confidence
pub const PHI_INV2: f64 = 0.381_966_011_250_105; // anomaly threshold
pub const PHI_INV3: f64 = 0.236_067_977_499_790; // GROWL threshold
pub const PHI_INV4: f64 = 0.145_898_033_750_315; // convergence increment

/// Canonical axiom names — single source of truth for API surfaces.
pub const AXIOM_NAMES: &[&str] = &[
    "FIDELITY",
    "PHI",
    "VERIFY/FALSIFY",
    "CULTURE",
    "BURN",
    "SOVEREIGNTY",
];

/// Minimum Dogs required for a verdict to accumulate crystal observations (T8).
/// Single-Dog verdicts are served for availability but MUST NOT crystallize.
/// Value 2 = deterministic-dog + at least 1 LLM Dog. Falsified: no crystal
/// starvation at this threshold (H6 — crystal state doesn't decay without obs).
pub const MIN_QUORUM: usize = 2;

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
    /// RC7-2: Distributed tracing identifier propagated from the REST boundary
    #[serde(default)]
    pub request_id: Option<String>,
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
    /// Estimated thinking tokens (0 for non-thinking models).
    #[serde(default)]
    pub thinking_tokens: u32,
    /// Axioms where this Dog abstained (returned NEUTRAL, not active evaluation).
    #[serde(default)]
    pub abstentions: Vec<String>,
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
    /// Phi-bounded axiom scores (clamped to [0.05, φ⁻¹]). Used for consensus.
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
    /// Raw axiom scores BEFORE phi_bound. For diagnostics — what the model actually produced.
    #[serde(default)]
    pub raw_fidelity: f64,
    #[serde(default)]
    pub raw_phi: f64,
    #[serde(default)]
    pub raw_verify: f64,
    #[serde(default)]
    pub raw_culture: f64,
    #[serde(default)]
    pub raw_burn: f64,
    #[serde(default)]
    pub raw_sovereignty: f64,
    pub reasoning: AxiomReasoning,
    /// Axioms where this Dog abstained (returned NEUTRAL, not an active evaluation).
    /// Excluded from disagreement calculation — abstention ≠ disagreement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abstentions: Vec<String>,
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
    Howl,  // > φ⁻² + φ⁻⁴ = 0.528 (golden subdivision of WAG→MAX)
    Wag,   // > φ⁻²       = 0.382
    Growl, // > φ⁻³       = 0.236
    Bark,  // ≤ φ⁻³
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub id: String,
    pub domain: String,
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
    /// Number of Dogs that contributed scores to this verdict.
    /// Explicit field — not derived from dog_scores.len() — because dog_scores
    /// may be empty after DB deserialization (corrupt JSON blob).
    /// Used by quorum gate (T8): only verdicts with voter_count >= min_quorum crystallize.
    #[serde(default)]
    pub voter_count: usize,
    /// Dog IDs that failed during evaluation — ephemeral, not persisted to DB.
    /// Consumed by handlers immediately after evaluation to call usage.record_failure().
    #[serde(default)]
    pub failed_dogs: Vec<String>,
    /// Error details per failed Dog (dog_id → error description). Ephemeral, not persisted.
    /// Used by event bus to emit DogFailed events with the actual failure reason.
    #[serde(default)]
    pub failed_dog_errors: HashMap<String, String>,
    /// BLAKE3 hash of this verdict's content (L1 integrity)
    #[serde(default)]
    pub integrity_hash: Option<String>,
    /// BLAKE3 hash of the previous verdict (hash chain)
    #[serde(default)]
    pub prev_hash: Option<String>,
}

// ── SCORE VALIDATION (pre-phi_bound) ─────────────────────
/// Minimum variance threshold for score validation.
/// Below this, scores are considered degenerate (model collapsed to uniform output).
/// Value: 10⁻⁴ — extremely conservative, catches only pathological cases.
const MIN_SCORE_VARIANCE: f64 = 0.0001;

/// Maximum number of raw 0.0 scores before declaring a zero flood.
/// 4 out of 6 axioms at exactly 0.0 = the model failed to score, not a real judgment.
const MAX_ZERO_SCORES: usize = 3;

/// Validate raw axiom scores BEFORE phi_bound. Catches pathological LLM outputs
/// that would otherwise be silently masked by phi_bound (0.0 → 0.05).
///
/// Conservative thresholds — only rejects clearly degenerate outputs:
/// - Zero flood: ≥4 axioms at exactly 0.0
/// - Degenerate variance: all scores within ε (model collapsed)
///
/// Returns `Ok(())` for usable scores, even imperfect ones.
pub fn validate_scores(scores: &AxiomScores) -> Result<(), DogError> {
    let raw = [
        scores.fidelity,
        scores.phi,
        scores.verify,
        scores.culture,
        scores.burn,
        scores.sovereignty,
    ];

    // Zero flood: too many axioms at exactly 0.0 = parse/generation failure
    let zero_count = raw.iter().filter(|&&v| v == 0.0).count();
    if zero_count > MAX_ZERO_SCORES {
        return Err(DogError::ZeroFlood(zero_count));
    }

    // Degenerate variance: all scores nearly identical = model collapsed.
    // Exception: uniform scores near the floor (mean ≤ 0.10) are a valid signal —
    // "everything is terrible" is a legitimate judgment on genuinely bad content.
    // Without this exception, models that correctly identify rug pulls (all axioms
    // at minimum) are rejected as collapsed, and their valid reasoning is lost.
    let mean = raw.iter().sum::<f64>() / 6.0;
    let variance = raw.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 6.0;
    if variance < MIN_SCORE_VARIANCE && mean > 0.10 {
        return Err(DogError::DegenerateScores {
            variance,
            min_variance: MIN_SCORE_VARIANCE,
        });
    }

    Ok(())
}

// ── PHI-BOUNDING ───────────────────────────────────────────
/// Clamp a raw score to [FLOOR, phi^-1]. Floor at 0.05 because a true zero
/// is always a parsing failure, never a real epistemic judgment. A zero in the
/// arithmetic mean drags consensus unfairly; 0.05 preserves the "near-zero = terrible"
/// signal without mathematical pathology.
const SCORE_FLOOR: f64 = 0.05;
pub fn phi_bound(raw: f64) -> f64 {
    if !raw.is_finite() {
        return SCORE_FLOOR;
    }
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

    QScore {
        total,
        fidelity: f,
        phi: p,
        verify: v,
        culture: c,
        burn: b,
        sovereignty: s,
    }
}

/// Determine verdict from Q-Score total (phi-bounded scale: 0..φ⁻¹)
///
/// Thresholds are golden subdivisions:
///   GROWL: φ⁻³ (golden cut of [0, φ⁻²])
///   WAG:   φ⁻² (golden cut of [0, φ⁻¹])
///   HOWL:  φ⁻² + φ⁻⁴ (golden cut of [φ⁻², φ⁻¹])
pub fn verdict_kind(total: f64) -> VerdictKind {
    if total > PHI_INV2 + PHI_INV4 {
        VerdictKind::Howl
    } else if total > PHI_INV2 {
        VerdictKind::Wag
    } else if total > PHI_INV3 {
        VerdictKind::Growl
    } else {
        VerdictKind::Bark
    }
}

// ── DOG TRAIT ──────────────────────────────────────────────
/// The contract every evaluator must fulfill.
/// Model-agnostic: Gemini, Llama, GPT, deterministic code — all identical to the caller.
#[async_trait::async_trait]
pub trait Dog: Send + Sync {
    /// Unique identifier for this Dog
    fn id(&self) -> &str;

    /// Max context tokens this Dog supports. 0 = unlimited.
    fn max_context(&self) -> u32 {
        0
    }

    /// Max evaluation timeout in seconds. Default: 30. Sovereign CPU models override to 60+.
    fn timeout_secs(&self) -> u64 {
        30
    }

    /// Backend health — cascades from the underlying inference provider.
    /// Default: Healthy (correct for DeterministicDog and any in-process evaluator).
    /// InferenceDog delegates to its ChatPort's BackendPort::health().
    async fn health(&self) -> crate::domain::inference::BackendStatus {
        crate::domain::inference::BackendStatus::Healthy
    }

    /// Evaluate a stimulus, return raw axiom scores (NOT phi-bounded)
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError>;
}

/// Estimate token count from text. Heuristic: ~4 chars per token for English.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as f64 / 4.0).ceil() as u32
}

#[derive(Debug, thiserror::Error)]
pub enum DogError {
    #[error("Dog API error: {0}")]
    ApiError(String),
    #[error("Dog parse error: {0}")]
    ParseError(String),
    #[error("zero flood: {0}/6 axioms at 0.0")]
    ZeroFlood(usize),
    #[error("degenerate scores: variance {variance:.6} < {min_variance}")]
    DegenerateScores { variance: f64, min_variance: f64 },
    #[error("Dog rate limited: {0}")]
    RateLimited(String),
    #[error("Dog evaluation timed out")]
    Timeout,
    #[error(
        "context overflow: prompt {prompt_tokens} + completion {completion_budget} = {total} > context {context_size}"
    )]
    ContextOverflow {
        prompt_tokens: u32,
        completion_budget: u32,
        total: u32,
        context_size: u32,
    },
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
    fn phi_bound_floors_nan_and_infinity() {
        assert!((phi_bound(f64::NAN) - SCORE_FLOOR).abs() < 1e-10);
        assert!((phi_bound(f64::INFINITY) - SCORE_FLOOR).abs() < 1e-10);
        assert!((phi_bound(f64::NEG_INFINITY) - SCORE_FLOOR).abs() < 1e-10);
    }

    #[test]
    fn qscore_geometric_mean_correct() {
        let raw = AxiomScores {
            fidelity: 0.6,
            phi: 0.5,
            verify: 0.4,
            culture: 0.5,
            burn: 0.5,
            sovereignty: 0.6,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        };
        let q = compute_qscore(&raw);
        assert!(q.total <= PHI_INV + 1e-10);
        assert!(q.fidelity <= PHI_INV + 1e-10);
        let expected = (0.6_f64 * 0.5 * 0.4 * 0.5 * 0.5 * 0.6).powf(1.0 / 6.0);
        assert!((q.total - expected).abs() < 0.01);
    }

    #[test]
    fn one_weak_axiom_drags_score_down() {
        let strong = AxiomScores {
            fidelity: 0.6,
            phi: 0.6,
            verify: 0.6,
            culture: 0.6,
            burn: 0.6,
            sovereignty: 0.6,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        };
        let weak = AxiomScores {
            fidelity: 0.6,
            phi: 0.6,
            verify: 0.6,
            culture: 0.6,
            burn: 0.6,
            sovereignty: 0.1,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
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

    // ── validate_scores tests ────────────────────────────────

    fn scores_with(f: f64, p: f64, v: f64, c: f64, b: f64, s: f64) -> AxiomScores {
        AxiomScores {
            fidelity: f,
            phi: p,
            verify: v,
            culture: c,
            burn: b,
            sovereignty: s,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        }
    }

    #[test]
    fn validate_normal_scores_pass() {
        let s = scores_with(0.5, 0.4, 0.3, 0.45, 0.6, 0.35);
        assert!(validate_scores(&s).is_ok());
    }

    #[test]
    fn validate_low_but_varied_scores_pass() {
        // Low scores are fine as long as they're not all zero
        let s = scores_with(0.1, 0.05, 0.15, 0.2, 0.08, 0.12);
        assert!(validate_scores(&s).is_ok());
    }

    #[test]
    fn validate_zero_flood_rejected() {
        // 4 out of 6 at 0.0 — model failed to score
        let s = scores_with(0.0, 0.0, 0.0, 0.0, 0.5, 0.3);
        let err = validate_scores(&s).unwrap_err();
        assert!(err.to_string().contains("zero flood"), "got: {err}");
    }

    #[test]
    fn validate_three_zeros_pass() {
        // 3 zeros is at the threshold — still accepted (conservative)
        let s = scores_with(0.0, 0.0, 0.0, 0.5, 0.4, 0.3);
        assert!(validate_scores(&s).is_ok());
    }

    #[test]
    fn validate_all_zero_rejected() {
        let s = scores_with(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let err = validate_scores(&s).unwrap_err();
        assert!(err.to_string().contains("zero flood"), "got: {err}");
    }

    #[test]
    fn validate_degenerate_all_same_rejected() {
        // Model collapsed — all scores identical
        let s = scores_with(0.5, 0.5, 0.5, 0.5, 0.5, 0.5);
        let err = validate_scores(&s).unwrap_err();
        assert!(err.to_string().contains("degenerate"), "got: {err}");
    }

    #[test]
    fn validate_nearly_same_but_varied_enough_pass() {
        // Close but not identical — should pass
        let s = scores_with(0.50, 0.49, 0.51, 0.50, 0.48, 0.52);
        assert!(validate_scores(&s).is_ok());
    }

    #[test]
    fn validate_mistral_pattern_rejected() {
        // Real Mistral failure: 4 zeros + sovereignty inverted
        let s = scores_with(0.0, 0.0, 0.0, 0.0, 0.5, 1.0);
        let err = validate_scores(&s).unwrap_err();
        assert!(err.to_string().contains("zero flood"), "got: {err}");
    }
}
