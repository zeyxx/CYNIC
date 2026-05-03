//! Judge consensus math — pure functions, no state.
//! Aggregation (trimmed mean + median reasoning), residual detection, verdict hash chain.

use crate::domain::dog::{
    AxiomReasoning, AxiomScores, DogScore, PHI_INV2, QScore, Verdict, compute_qscore,
};

/// Trimmed mean: drop highest + lowest value when >= 4 scores, average the rest.
/// With 2-3 scores: plain arithmetic mean. Robust against outlier LLM responses.
/// Dogs that abstained on the target axiom are excluded — abstention ≠ low score.
pub(super) fn trimmed_mean(
    scores: &[DogScore],
    axiom_name: &str,
    extract: impl Fn(&DogScore) -> f64,
) -> f64 {
    let mut values: Vec<f64> = scores
        .iter()
        .filter(|s| !s.abstentions.iter().any(|a| a == axiom_name))
        .map(&extract)
        .collect();

    // Fallback: if all dogs abstained, include everyone (better than empty)
    if values.is_empty() {
        values = scores.iter().map(&extract).collect();
    }

    // WHY: malformed LLM responses can produce NaN/Inf scores. NaN compares as
    // Equal via unwrap_or, bypassing trimmed-mean outlier filtering silently.
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if values.len() >= 4 {
        let trimmed = &values[1..values.len() - 1];
        trimmed.iter().sum::<f64>() / trimmed.len() as f64
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// BLAKE3 integrity hash of a verdict — forms a hash chain linking verdicts.
/// Lives here (not in domain/) because blake3 is an external crate.
/// Timestamp is canonicalized to `Z` suffix before hashing — SurrealDB normalizes
/// `+00:00` → `Z` on round-trip, so the hash must be format-independent.
pub(super) fn verdict_hash(
    id: &str,
    q_total: f64,
    scores: [f64; 6],
    stimulus: &str,
    timestamp: &str,
    prev_hash: Option<&str>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(id.as_bytes());
    hasher.update(&q_total.to_le_bytes());
    for s in &scores {
        hasher.update(&s.to_le_bytes());
    }
    hasher.update(stimulus.as_bytes());
    // Canonicalize: Rust to_rfc3339() emits "+00:00", SurrealDB returns "Z".
    // Both are semantically identical but byte-different → hash mismatch.
    let canonical_ts = timestamp.replace("+00:00", "Z");
    hasher.update(canonical_ts.as_bytes());
    if let Some(ph) = prev_hash {
        hasher.update(ph.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// Verify the BLAKE3 integrity hash of a verdict.
/// Re-derives the hash from verdict fields and compares against the stored hash.
/// Returns `false` if the hash is missing (pre-chain verdict) or mismatches (tampered).
pub fn verify_verdict_integrity(verdict: &Verdict) -> bool {
    let Some(ref stored_hash) = verdict.integrity_hash else {
        return false;
    };
    let recomputed = verdict_hash(
        &verdict.id,
        verdict.q_score.total,
        [
            verdict.q_score.fidelity,
            verdict.q_score.phi,
            verdict.q_score.verify,
            verdict.q_score.culture,
            verdict.q_score.burn,
            verdict.q_score.sovereignty,
        ],
        &verdict.stimulus_summary,
        &verdict.timestamp,
        verdict.prev_hash.as_deref(),
    );
    stored_hash == &recomputed
}

/// Aggregate per-Dog scores into consensus AxiomScores.
///
/// - Each axiom is reduced by `trimmed_mean` (robust against outliers).
/// - The `reasoning` field is taken from the median Dog (sorted by Q-score) — deterministic
///   under parallel execution since we sort before picking.
pub(super) fn aggregate_scores(dog_scores: &[DogScore]) -> AxiomScores {
    let avg_fidelity = trimmed_mean(dog_scores, "fidelity", |s| s.fidelity);
    let avg_phi = trimmed_mean(dog_scores, "phi", |s| s.phi);
    let avg_verify = trimmed_mean(dog_scores, "verify", |s| s.verify);
    let avg_culture = trimmed_mean(dog_scores, "culture", |s| s.culture);
    let avg_burn = trimmed_mean(dog_scores, "burn", |s| s.burn);
    let avg_sovereignty = trimmed_mean(dog_scores, "sovereignty", |s| s.sovereignty);

    // Use median Dog's reasoning (deterministic under parallel execution)
    let mut sorted_by_q: Vec<&DogScore> = dog_scores.iter().collect();
    sorted_by_q.sort_by(|a, b| {
        let qa = compute_qscore(&AxiomScores {
            fidelity: a.fidelity,
            phi: a.phi,
            verify: a.verify,
            culture: a.culture,
            burn: a.burn,
            sovereignty: a.sovereignty,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        })
        .total;
        let qb = compute_qscore(&AxiomScores {
            fidelity: b.fidelity,
            phi: b.phi,
            verify: b.verify,
            culture: b.culture,
            burn: b.burn,
            sovereignty: b.sovereignty,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        })
        .total;
        qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let median_reasoning = sorted_by_q
        .get(sorted_by_q.len() / 2)
        .map(|s| s.reasoning.clone())
        .unwrap_or_default();

    AxiomScores {
        fidelity: avg_fidelity,
        phi: avg_phi,
        verify: avg_verify,
        culture: avg_culture,
        burn: avg_burn,
        sovereignty: avg_sovereignty,
        reasoning: median_reasoning,
        ..Default::default()
    }
}

/// Soma gate: confidence-weighted aggregation (PATH 2 research).
///
/// Each Dog is weighted by its axiom confidence: `1.0 / (axiom_spread + ε)`.
/// Dogs with tighter axiom distributions (lower spread) contribute more to the consensus.
/// This is the hypothesis: removing "noisy" Dogs is actually a calibration problem — all Dogs
/// are valid oscillators at different frequencies, and we should weight by coherence.
// WHY: Reserved for Phase 2 (May 5-6) Soma gate measurement; will be called when soma_gate flag is set.
#[allow(dead_code)]
pub(super) fn aggregate_scores_soma_gate(dog_scores: &[DogScore]) -> (AxiomScores, Vec<f64>) {
    const EPSILON: f64 = 0.01; // Avoid division by zero

    // Compute per-Dog axiom spreads (max - min across the 6 axioms)
    let dog_spreads: Vec<(usize, f64)> = dog_scores
        .iter()
        .enumerate()
        .map(|(i, dog)| {
            let values = [
                dog.fidelity,
                dog.phi,
                dog.verify,
                dog.culture,
                dog.burn,
                dog.sovereignty,
            ];
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let spread = (max - min).max(0.0);
            (i, spread)
        })
        .collect();

    // Compute confidence weights: 1.0 / (spread + epsilon)
    let mut weights: Vec<f64> = dog_spreads
        .iter()
        .map(|(_, spread)| 1.0 / (spread + EPSILON))
        .collect();

    // Normalize weights to sum to 1.0
    let total_weight: f64 = weights.iter().sum();
    if total_weight > 0.0 {
        weights.iter_mut().for_each(|w| *w /= total_weight);
    } else {
        // Fallback: equal weights
        let weight = 1.0 / dog_scores.len() as f64;
        weights = vec![weight; dog_scores.len()];
    }

    // Weighted mean per axiom
    let mut weighted_fidelity = 0.0;
    let mut weighted_phi = 0.0;
    let mut weighted_verify = 0.0;
    let mut weighted_culture = 0.0;
    let mut weighted_burn = 0.0;
    let mut weighted_sovereignty = 0.0;

    for (i, dog) in dog_scores.iter().enumerate() {
        let w = weights[i];
        if !dog.abstentions.iter().any(|a| a == "fidelity") {
            weighted_fidelity += dog.fidelity * w;
        }
        if !dog.abstentions.iter().any(|a| a == "phi") {
            weighted_phi += dog.phi * w;
        }
        if !dog.abstentions.iter().any(|a| a == "verify") {
            weighted_verify += dog.verify * w;
        }
        if !dog.abstentions.iter().any(|a| a == "culture") {
            weighted_culture += dog.culture * w;
        }
        if !dog.abstentions.iter().any(|a| a == "burn") {
            weighted_burn += dog.burn * w;
        }
        if !dog.abstentions.iter().any(|a| a == "sovereignty") {
            weighted_sovereignty += dog.sovereignty * w;
        }
    }

    // Use median Dog's reasoning (same as baseline)
    let mut sorted_by_q: Vec<&DogScore> = dog_scores.iter().collect();
    sorted_by_q.sort_by(|a, b| {
        let qa = compute_qscore(&AxiomScores {
            fidelity: a.fidelity,
            phi: a.phi,
            verify: a.verify,
            culture: a.culture,
            burn: a.burn,
            sovereignty: a.sovereignty,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        })
        .total;
        let qb = compute_qscore(&AxiomScores {
            fidelity: b.fidelity,
            phi: b.phi,
            verify: b.verify,
            culture: b.culture,
            burn: b.burn,
            sovereignty: b.sovereignty,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        })
        .total;
        qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let median_reasoning = sorted_by_q
        .get(sorted_by_q.len() / 2)
        .map(|s| s.reasoning.clone())
        .unwrap_or_default();

    let axiom_scores = AxiomScores {
        fidelity: weighted_fidelity,
        phi: weighted_phi,
        verify: weighted_verify,
        culture: weighted_culture,
        burn: weighted_burn,
        sovereignty: weighted_sovereignty,
        reasoning: median_reasoning,
        ..Default::default()
    };

    (axiom_scores, weights)
}

/// Residual detection: find max per-axiom spread across Dogs.
///
/// Catches cases where Dogs agree on Q-score total but disagree on individual axioms.
/// Abstentions are excluded per axiom — a Dog that abstained does not contribute to spread.
///
/// Returns `(max_disagreement, anomaly_axiom)` where `anomaly_axiom` is `Some` only
/// when `max_disagreement > φ⁻² = 0.382` (the anomaly threshold).
pub(super) fn detect_residuals(dog_scores: &[DogScore]) -> (f64, Option<String>) {
    if dog_scores.len() <= 1 {
        return (0.0, None);
    }

    let axiom_names = [
        "fidelity",
        "phi",
        "verify",
        "culture",
        "burn",
        "sovereignty",
    ];
    let spreads: Vec<(f64, &str)> = axiom_names
        .iter()
        .map(|&name| {
            // Exclude Dogs that abstained on this axiom — abstention ≠ disagreement.
            let values: Vec<f64> = dog_scores
                .iter()
                .filter(|s| !s.abstentions.iter().any(|a| a == name))
                .map(|s| match name {
                    "fidelity" => s.fidelity,
                    "phi" => s.phi,
                    "verify" => s.verify,
                    "culture" => s.culture,
                    "burn" => s.burn,
                    "sovereignty" => s.sovereignty,
                    _ => 0.0,
                })
                .filter(|v| v.is_finite())
                .collect();
            if values.len() < 2 {
                return (0.0, name); // Can't compute spread with < 2 active scores
            }
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            (max - min, name)
        })
        .collect();
    let (max_spread, max_axiom) = spreads
        .iter()
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(s, n)| (*s, *n))
        .unwrap_or((0.0, ""));

    if max_spread > PHI_INV2 {
        (max_spread, Some(max_axiom.to_string()))
    } else {
        (max_spread, None)
    }
}

/// Compute a chained verdict hash using the caller's previous-hash pointer.
///
/// Returns `(new_hash, prev_hash)`. The caller is responsible for holding whatever
/// lock protects the chain pointer and for writing `new_hash` back into it.
pub(super) fn chain_hash(
    id: &str,
    q_score: &QScore,
    stimulus_summary: &str,
    timestamp: &str,
    prev_hash: Option<&str>,
) -> String {
    verdict_hash(
        id,
        q_score.total,
        [
            q_score.fidelity,
            q_score.phi,
            q_score.verify,
            q_score.culture,
            q_score.burn,
            q_score.sovereignty,
        ],
        stimulus_summary,
        timestamp,
        prev_hash,
    )
}
