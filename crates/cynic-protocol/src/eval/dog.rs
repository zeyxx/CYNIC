//! Dog — coordination vocabulary for evaluation evidence.
//!
//! Pure types only: stimuli, axiom scores, verdicts, and quorum/threshold
//! constants consumed by crystallization and storage. Scoring law lives in
//! `cynic-intel`; this module keeps no evaluator implementation.

pub use crate::phi::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// Optional reasoning per axiom (structured JSON from model)
    pub reasoning: AxiomReasoning,
    /// Raw reasoning trace/Chain-of-Thought (CoT) before JSON extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_trace: Option<String>,
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
    /// Raw reasoning trace/Chain-of-Thought (CoT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_trace: Option<String>,
    /// Axioms where this Dog abstained (returned NEUTRAL, not an active evaluation).
    /// Excluded from disagreement calculation — abstention ≠ disagreement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abstentions: Vec<String>,
}

// ── PHI-BOUNDED Q-SCORE ────────────────────────────────────
/// The kernel's final uncertainty score (Compass Signal).
/// Instead of a consensus mean, it represents the geometric mean of the variance/spread
/// across all Dogs for each axiom. Every value capped at phi^-1 (0.618).
/// Low QScore = High Consensus (Bark). High QScore = High Uncertainty (Howl/Epoche).
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
    Howl,   // > φ⁻² + φ⁻⁴ = 0.528 (golden subdivision of WAG→MAX)
    Wag,    // > φ⁻²       = 0.382
    Growl,  // > φ⁻³       = 0.236
    Bark,   // ≤ φ⁻³
    Epoche, // Pyrrhonist suspension — Dogs in equipollence (data-driven P95 threshold)
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
    pub timestamp: i64,
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
    #[serde(default)]
    pub failed_dog_errors: BTreeMap<String, String>,
    /// Structured error kind per failed Dog (dog_id → error code string).
    /// Codes: api_error | parse_error | rate_limited | timeout | slot_unavailable
    ///        network_unreachable | circuit_open | quality_gate_excluded
    #[serde(default)]
    pub failed_dog_error_kinds: BTreeMap<String, String>,
    /// Dogs excluded before evaluation by circuit breaker or quality gate (not attempted).
    #[serde(default)]
    pub excluded_dogs: Vec<String>,
    /// Target identifier — the original content before enrichment.
    /// For token-analysis: the mint address. For wallet-judgment: the wallet address.
    /// For other domains: None (stimulus_summary is sufficient).
    #[serde(default)]
    pub target: Option<String>,
    /// BLAKE3 hash of this verdict's content (L1 integrity)
    #[serde(default)]
    pub integrity_hash: Option<String>,
    /// BLAKE3 hash of the previous verdict (hash chain)
    #[serde(default)]
    pub prev_hash: Option<String>,
}
