//! Crystal observation from verdicts — the critical link in the compound loop.
//!
//! Extracted from pipeline.rs: epistemic gating + crystal observation logic.
//! Called on BOTH cache hits and full evaluations.

use crate::domain::ccm;
use crate::domain::dog::{PHI_INV, PHI_INV2, PHI_INV3};
use crate::domain::embedding::Embedding;
use crate::domain::events::KernelEvent;

use super::PipelineDeps;

/// Compute epistemic injection weight from Dog disagreement level.
///
/// Returns `(tag, weight)` where tag is "agreed"/"disputed"/"contested"
/// and weight is in [0.0, 1.0]. Pure function — no side effects.
pub(crate) fn epistemic_gate(max_disagreement: f64) -> (&'static str, f64) {
    if max_disagreement < PHI_INV3 {
        ("agreed", 1.0)
    } else if max_disagreement < PHI_INV2 {
        // Linear decay from 1.0 (at φ⁻³) to 0.0 (at φ⁻²)
        let range = PHI_INV2 - PHI_INV3;
        let position = max_disagreement - PHI_INV3;
        ("disputed", (1.0 - position / range).max(0.0))
    } else {
        ("contested", 0.0)
    }
}

/// Observe a crystal from a verdict — the critical link in the compound loop.
///
/// Called on BOTH cache hits and full evaluations. Without this on cache hits,
/// repeated stimuli (which are the MOST common in real usage) never accumulate
/// crystal observations → crystals never mature → never inject → no compound.
///
/// Uses semantic merge (cosine ≥ 0.75) to accumulate on existing crystals
/// instead of creating fragmented duplicates.
///
/// **Epistemic soft gate** (2026-03-24): verdicts with high Dog disagreement
/// are weighted down to prevent crystal poisoning. Three tiers:
///
///   - Agreed   (disagree < φ⁻³): full weight (1.0)
///   - Disputed (φ⁻³ ≤ disagree < φ⁻²): linear decay 1.0→0.0
///   - Contested (disagree ≥ φ⁻²): quarantined — zero crystal update
///
/// Research: noisy-label learning (Northcutt JAIR 2022),
/// recommender feedback bias (FAccT 2024), QBC active learning.
pub(crate) async fn observe_crystal_for_verdict(
    verdict: &crate::domain::dog::Verdict,
    stimulus_embedding: &Option<Embedding>,
    domain: &str,
    deps: &PipelineDeps<'_>,
) {
    // ── T8+T9: Quorum gate — single-Dog verdicts must NOT crystallize ──
    // Verdict is still SERVED (availability), but only consensus crystallizes (integrity).
    if verdict.voter_count < crate::domain::dog::MIN_QUORUM {
        tracing::info!(
            phase = "crystal_gate",
            voter_count = verdict.voter_count,
            min_quorum = crate::domain::dog::MIN_QUORUM,
            "quorum not met — crystal observation skipped (verdict still served)"
        );
        return;
    }

    // ── Epistemic soft gate: weight crystal observation by Dog agreement ──
    let (epistemic_tag, injection_weight) = epistemic_gate(verdict.max_disagreement);

    if injection_weight <= 0.0 {
        tracing::info!(
            phase = "crystal_gate",
            tag = epistemic_tag,
            disagreement = %format!("{:.3}", verdict.max_disagreement),
            "contested verdict — crystal observation quarantined"
        );
        return;
    }

    if injection_weight < 1.0 {
        tracing::info!(
            phase = "crystal_gate",
            tag = epistemic_tag,
            weight = %format!("{:.3}", injection_weight),
            disagreement = %format!("{:.3}", verdict.max_disagreement),
            "disputed verdict — crystal observation weight reduced"
        );
    }
    // Semantic merge: find existing crystal or create new via FNV hash.
    // Track `needs_embedding`: existing crystals already have a working embedding
    // in the HNSW index — re-writing it on every observation is pure waste and the
    // primary cause of SurrealKV compaction conflicts on IX:2 (176/24h).
    let (crystal_id, needs_embedding) = if let Some(emb) = stimulus_embedding {
        match deps
            .storage
            .find_similar_crystal(&emb.vector, domain, 0.75)
            .await
        {
            Ok(Some((existing_id, sim))) => {
                tracing::info!(phase = "crystal_merge", crystal_id = %existing_id, similarity = %format!("{:.3}", sim), "reusing existing crystal");
                (existing_id, false)
            }
            Ok(None) => {
                tracing::info!(
                    phase = "crystal_merge",
                    "no similar crystal (sim < 0.75), creating new"
                );
                (
                    format!(
                        "{:x}",
                        ccm::content_hash(&ccm::normalize_for_hash(&format!(
                            "{}:{}",
                            domain, verdict.stimulus_summary
                        )))
                    ),
                    true,
                )
            }
            Err(e) => {
                tracing::warn!(phase = "crystal_merge", error = %e, "similarity search failed, using FNV hash");
                (
                    format!(
                        "{:x}",
                        ccm::content_hash(&ccm::normalize_for_hash(&format!(
                            "{}:{}",
                            domain, verdict.stimulus_summary
                        )))
                    ),
                    true,
                )
            }
        }
    } else {
        tracing::info!(
            phase = "crystal_merge",
            "no embedding available, using FNV hash"
        );
        (
            format!(
                "{:x}",
                ccm::content_hash(&ccm::normalize_for_hash(&format!(
                    "{}:{}",
                    domain, verdict.stimulus_summary
                )))
            ),
            true,
        )
    };

    let now = chrono::Utc::now().to_rfc3339();
    // Normalize Q-Score: raw scores are φ-bounded (max ≈ 0.618).
    // Without normalization, no crystal can ever reach the 0.618 crystallization threshold.
    // Apply epistemic weight: disputed verdicts contribute less to crystal confidence.
    let crystal_confidence = ((verdict.q_score.total / PHI_INV) * injection_weight).min(1.0);
    let verdict_kind = match verdict.kind {
        crate::domain::dog::VerdictKind::Howl => "howl",
        crate::domain::dog::VerdictKind::Wag => "wag",
        crate::domain::dog::VerdictKind::Growl => "growl",
        crate::domain::dog::VerdictKind::Bark => "bark",
    };
    if let Err(e) = deps
        .storage
        .observe_crystal(
            &crystal_id,
            &verdict.stimulus_summary,
            domain,
            crystal_confidence,
            &now,
            verdict.voter_count,
            &verdict.id,
            verdict_kind,
        )
        .await
    {
        tracing::warn!(phase = "crystal_observe", crystal_id = %crystal_id, error = %e, "failed to observe crystal");
    } else {
        tracing::info!(
            organ = "crystal",
            action = "observed",
            crystal_id = %crystal_id,
            domain = %domain,
            confidence = %format!("{:.3}", crystal_confidence),
            polarity = %verdict_kind,
            voters = verdict.voter_count,
            epistemic = %epistemic_tag,
            "crystal observation recorded"
        );
        deps.metrics.inc_crystal_obs();
        if let Some(tx) = deps.event_tx {
            let _ = tx.send(KernelEvent::CrystalObserved {
                crystal_id: crystal_id.clone(),
                domain: domain.to_string(),
            }); // ok: no subscribers = silent no-op (receiver_count=0 returns Err)
        }
    }
    // Store embedding for KNN merge — only for NEW crystals.
    // Existing crystals (found via semantic search) already have a working embedding
    // in the HNSW index. Overwriting it on every observation caused 176 SurrealKV
    // compaction conflicts/24h on IX:2, eventually crashing the DB.
    if needs_embedding
        && let Some(emb) = stimulus_embedding
        && let Err(e) = deps
            .storage
            .store_crystal_embedding(&crystal_id, &emb.vector)
            .await
    {
        tracing::warn!(phase = "crystal_embed", crystal_id = %crystal_id, error = %e, "failed to store crystal embedding");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Epistemic gate tests ──────────────────────────────────

    #[test]
    fn epistemic_gate_agreed_for_low_disagreement() {
        let (tag, weight) = epistemic_gate(0.1);
        assert_eq!(tag, "agreed");
        assert!((weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_agreed_at_zero() {
        let (tag, weight) = epistemic_gate(0.0);
        assert_eq!(tag, "agreed");
        assert!((weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_disputed_mid_range() {
        // Midpoint between φ⁻³ (0.236) and φ⁻² (0.382)
        let mid = (PHI_INV3 + PHI_INV2) / 2.0;
        let (tag, weight) = epistemic_gate(mid);
        assert_eq!(tag, "disputed");
        assert!(
            (weight - 0.5).abs() < 0.01,
            "midpoint should be ~0.5, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_disputed_near_agreed_boundary() {
        let (tag, weight) = epistemic_gate(PHI_INV3 + 0.001);
        assert_eq!(tag, "disputed");
        assert!(
            weight > 0.9,
            "near agreed boundary should be ~1.0, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_disputed_near_contested_boundary() {
        let (tag, weight) = epistemic_gate(PHI_INV2 - 0.001);
        assert_eq!(tag, "disputed");
        assert!(
            weight < 0.01,
            "near contested boundary should be ~0.0, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_contested_at_threshold() {
        let (tag, weight) = epistemic_gate(PHI_INV2);
        assert_eq!(tag, "contested");
        assert!((weight - 0.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_contested_above_threshold() {
        let (tag, weight) = epistemic_gate(0.55);
        assert_eq!(tag, "contested");
        assert!((weight - 0.0).abs() < 1e-10);
    }

    // ── crystal_confidence normalization (P1 ACCURACY) ──────

    #[test]
    fn crystal_confidence_howl_reaches_crystallization_threshold() {
        // HOWL verdict: Q-Score total at max φ-bounded = 0.618
        // crystal_confidence = (total / PHI_INV) * weight = (0.618 / 0.618) * 1.0 = 1.0
        let total = PHI_INV; // max possible Q-Score
        let weight = 1.0; // agreed (no dispute)
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(
            conf >= PHI_INV,
            "HOWL verdict should produce crystal_confidence >= φ⁻¹ (crystallization threshold), got {conf}"
        );
    }

    #[test]
    fn crystal_confidence_bark_stays_below_crystallization() {
        // BARK verdict: Q-Score total = 0.2 (well below threshold)
        let total = 0.2;
        let weight = 1.0;
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(
            conf < PHI_INV,
            "BARK verdict should NOT reach crystallization threshold, got {conf}"
        );
    }

    #[test]
    fn crystal_confidence_disputed_howl_reduced() {
        // HOWL Q-Score but disputed (weight < 1.0)
        let total = PHI_INV;
        let weight = 0.5; // disputed midpoint
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(
            conf < PHI_INV,
            "disputed HOWL should be reduced below crystallization, got {conf}"
        );
        assert!(
            conf > 0.0,
            "disputed HOWL should still have positive confidence"
        );
    }

    #[test]
    fn crystal_confidence_clamped_at_one() {
        // Even if formula would exceed 1.0 (shouldn't with phi-bounded inputs, but safety)
        let total = PHI_INV;
        let weight = 1.0;
        let conf = ((total / PHI_INV) * weight).min(1.0);
        assert!(conf <= 1.0, "crystal_confidence must be ≤ 1.0, got {conf}");
    }
}
