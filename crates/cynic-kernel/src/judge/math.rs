//! Judge consensus math — pure functions, no state.
//! Aggregation (trimmed mean + median reasoning), residual detection, verdict hash chain.

use crate::domain::dog::{
    AxiomReasoning, AxiomScores, DogScore, PHI_INV2, QScore, Verdict, compute_qscore,
};

/// Compute the spread (max - min) of scores for a given axiom across Dogs.
/// This replaces `trimmed_mean` — the Kernel no longer computes a consensus mean.
/// The signal is pure uncertainty (variance/friction). The AI is a compass.
/// Dogs that abstained on the target axiom are excluded.
pub(super) fn axiom_spread(
    scores: &[DogScore],
    axiom_name: &str,
    extract: impl Fn(&DogScore) -> f64,
) -> f64 {
    let values: Vec<f64> = scores
        .iter()
        .filter(|s| !s.abstentions.iter().any(|a| a == axiom_name))
        .map(&extract)
        .filter(|v| v.is_finite())
        .collect();

    // If 0 or 1 Dog (no disagreement possible), or if all abstained
    if values.len() < 2 {
        return 0.05; // SCORE_FLOOR to prevent zero-flooding the geometric mean
    }

    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    
    let spread = max - min;
    if spread < 0.05 {
        0.05
    } else {
        spread
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
/// - The Kernel now acts as a COMPASS: it maps uncertainty.
/// - Each axiom is reduced by `axiom_spread` (measuring friction/variance, not a mean).
/// - The `reasoning` field is taken from the median Dog (sorted by Q-score) — deterministic
///   under parallel execution since we sort before picking.
pub(super) fn aggregate_scores(dog_scores: &[DogScore]) -> AxiomScores {
    let spread_fidelity = axiom_spread(dog_scores, "fidelity", |s| s.fidelity);
    let spread_phi = axiom_spread(dog_scores, "phi", |s| s.phi);
    let spread_verify = axiom_spread(dog_scores, "verify", |s| s.verify);
    let spread_culture = axiom_spread(dog_scores, "culture", |s| s.culture);
    let spread_burn = axiom_spread(dog_scores, "burn", |s| s.burn);
    let spread_sovereignty = axiom_spread(dog_scores, "sovereignty", |s| s.sovereignty);

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
        fidelity: spread_fidelity,
        phi: spread_phi,
        verify: spread_verify,
        culture: spread_culture,
        burn: spread_burn,
        sovereignty: spread_sovereignty,
        reasoning: median_reasoning,
        ..Default::default()
    }
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
