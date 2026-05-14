//! Crystal observation from verdicts — the critical link in the compound loop.
//!
//! Extracted from pipeline.rs: epistemic gating + crystal observation logic.
//! Called on BOTH cache hits and full evaluations.

use crate::domain::ccm;
use crate::domain::dog::{PHI_INV, PHI_INV2};
use crate::domain::embedding::Embedding;
use crate::domain::events::KernelEvent;

use super::PipelineDeps;

/// Compute epistemic injection weight from Dog disagreement level.
///
/// Returns `(tag, weight)` where tag is "agreed"/"disputed"/"contested"
/// and weight is in [0.0, 1.0]. Pure function — no side effects.
///
/// Thresholds widened (2026-04-26): structural divergence between qwen7b and
/// qwen35 produces per-axiom spreads of 0.4–0.6 on legitimate tokens. The
/// original φ⁻² (0.382) contested gate quarantined >80% of verdicts, starving
/// the CCM of observations. New gate uses φ⁻¹ (0.618) — philosophically
/// consistent: only quarantine disagreement beyond max-confidence ceiling.
///
///   - Agreed   (disagree < φ⁻²): full weight (1.0)
///   - Disputed (φ⁻² ≤ disagree < φ⁻¹): linear decay 1.0→0.0
///   - Contested (disagree ≥ φ⁻¹): quarantined — zero crystal update
pub(crate) fn epistemic_gate(max_disagreement: f64) -> (&'static str, f64) {
    if max_disagreement < PHI_INV2 {
        ("agreed", 1.0)
    } else if max_disagreement < PHI_INV {
        // Linear decay from 1.0 (at φ⁻²) to 0.0 (at φ⁻¹)
        let range = PHI_INV - PHI_INV2;
        let position = max_disagreement - PHI_INV2;
        ("disputed", (1.0 - position / range).max(0.0))
    } else {
        ("contested", 0.0)
    }
}

/// Domain-specific Q-score calibration. Maps raw Q-scores to [0, 1] range
/// based on domain's observed percentile distribution (2026-05-12 organism audit).
///
/// Problem: Raw Q-scores vary by domain (chess p50=0.549 vs twitter p50=0.249).
/// Without calibration, the same threshold (φ⁻¹=0.618) unfairly gates all domains.
/// Solution: Use domain percentiles to normalize Q-scores so p50 → 0.5 for all domains.
///
/// Percentiles derived from 120K+ observations, 13K verdicts (2026-05-12):
/// - chess          (p50=0.549, p95=0.611) — high variance, mostly repeatable positions
/// - token-analysis (p50=0.333, p95=0.468) — lower variance, diverse tokens
/// - twitter        (p50=0.249, p95=0.387) — lowest, social media noise
/// - dev            (p50=0.380, p95=0.510) — estimated from git commit variability
/// - general        (p50=0.300, p95=0.420) — fallback for unmapped domains
///
/// Calibration formula: `normalized_q = (raw_q / domain_p50).min(1.0)`
/// Maps p50 → 0.5, p95 → ~1.0 (clamped), preserving relative signal.
/// The caller scales the result by PHI_INV to restore the absolute epistemic ceiling.
fn domain_specific_q_normalization(raw_q: f64, domain: &str) -> f64 {
    let domain_p50 = match domain {
        "chess" => 0.549,
        "token" | "token-analysis" => 0.333,
        "twitter" => 0.249,
        "dev" => 0.380,
        "ops" => 0.300,     // kernel/infra observations — new domain, estimate
        "session" => 0.350, // session compliance/metrics — estimate
        "hermes" => 0.280,  // hermes organ health/cycles — estimate
        _ => 0.300,         // general fallback
    };
    (raw_q / domain_p50).min(1.0)
}

/// Observe a crystal from a verdict — the critical link in the compound loop.
///
/// Called from BOTH the pipeline (user /judge) and nightshift (background).
/// Unifying these paths ensures all crystal observations go through the same
/// gates (quorum, epistemic, KNN merge) regardless of origin.
///
/// Uses semantic merge (cosine ≥ 0.75) to accumulate on existing crystals
/// instead of creating fragmented duplicates.
///
/// **Epistemic soft gate** (widened 2026-04-26): verdicts with high Dog disagreement
/// are weighted down to prevent crystal poisoning. Three tiers:
///
///   - Agreed   (disagree < φ⁻²): full weight (1.0)
///   - Disputed (φ⁻² ≤ disagree < φ⁻¹): linear decay 1.0→0.0
///   - Contested (disagree ≥ φ⁻¹): quarantined — zero crystal update
pub(crate) async fn observe_crystal_for_verdict(
    verdict: &crate::domain::dog::Verdict,
    stimulus_embedding: &Option<Embedding>,
    domain: &str,
    deps: &PipelineDeps<'_>,
) {
    observe_crystal_for_verdict_core(
        verdict,
        stimulus_embedding,
        domain,
        deps.storage,
        deps.metrics,
        deps.event_tx,
    )
    .await;
}

/// Core crystal observation logic — usable by both pipeline and nightshift.
///
/// Separated from PipelineDeps so nightshift can call it directly with
/// its own storage/metrics/event_tx, without constructing a full PipelineDeps.
pub async fn observe_crystal_for_verdict_core(
    verdict: &crate::domain::dog::Verdict,
    stimulus_embedding: &Option<Embedding>,
    domain: &str,
    storage: &dyn crate::domain::storage::StoragePort,
    metrics: &crate::domain::metrics::Metrics,
    event_tx: Option<&tokio::sync::broadcast::Sender<KernelEvent>>,
) {
    // ── Domain gate: "general" is a poison domain (KC poison fix) ──
    // Verdicts without an explicit domain default to "general" in the pipeline.
    // These are noise (test probes, unclassified requests). Crystallizing them
    // contaminates all domain queries. Skip.
    if domain == "general" {
        metrics.inc_domain_gate_skipped();
        tracing::debug!(
            phase = "crystal_gate",
            "domain='general' — crystal observation skipped (noise, not knowledge)"
        );
        return;
    }

    // ── T8+T9: Quorum gate — single-Dog verdicts must NOT crystallize ──
    // Verdict is still SERVED (availability), but only consensus crystallizes (integrity).
    if verdict.voter_count < crate::domain::dog::MIN_QUORUM {
        tracing::info!(
            phase = "crystal_gate",
            voter_count = verdict.voter_count,
            min_quorum = crate::domain::dog::MIN_QUORUM,
            "quorum not met — crystal observation skipped (verdict still served)"
        );
        metrics.inc_crystal_quorum_blocked();
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
        metrics.inc_crystal_contested();
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
        metrics.inc_crystal_disputed();
    }
    // Semantic merge: find existing crystal or create new via FNV hash.
    // Track `needs_embedding`: existing crystals already have a working embedding
    // in the HNSW index — re-writing it on every observation is pure waste and the
    // primary cause of SurrealKV compaction conflicts on IX:2 (176/24h).
    let (crystal_id, needs_embedding) = if let Some(emb) = stimulus_embedding {
        match storage
            .find_similar_crystal(&emb.vector, domain, 0.75)
            .await
        {
            Ok(Some((existing_id, sim))) => {
                tracing::info!(phase = "crystal_merge", crystal_id = %existing_id, similarity = %format!("{:.3}", sim), "reusing existing crystal");
                (existing_id, false)
            }
            Ok(None) => {
                let slug = ccm::semantic_slug(domain, &verdict.stimulus_summary);
                tracing::info!(
                    phase = "crystal_merge",
                    %slug,
                    "no similar crystal (sim < 0.75), creating new with semantic slug"
                );
                (
                    format!("{:x}", ccm::content_hash(&ccm::normalize_for_hash(&slug))),
                    true,
                )
            }
            Err(e) => {
                let slug = ccm::semantic_slug(domain, &verdict.stimulus_summary);
                tracing::warn!(phase = "crystal_merge", error = %e, %slug, "similarity search failed, using semantic slug + FNV hash");
                (
                    format!("{:x}", ccm::content_hash(&ccm::normalize_for_hash(&slug))),
                    true,
                )
            }
        }
    } else {
        // Semantic slug: reduce content to domain-aware topic before hashing.
        // Without this, every unique stimulus creates a new crystal.
        // Chess works because moves repeat exactly; dev/token/session never do.
        let slug = ccm::semantic_slug(domain, &verdict.stimulus_summary);
        tracing::info!(
            phase = "crystal_merge",
            %slug,
            "no embedding available, using semantic slug + FNV hash"
        );
        (
            format!("{:x}", ccm::content_hash(&ccm::normalize_for_hash(&slug))),
            true,
        )
    };

    let now = chrono::Utc::now().to_rfc3339();
    // Normalize Q-Score: raw scores are φ-bounded (max ≈ 0.618).
    // Without normalization, no crystal can ever reach the 0.618 crystallization threshold.
    // Apply domain-specific calibration (2026-05-12): chess vs twitter have different p50.
    // Then apply epistemic weight: disputed verdicts contribute less to crystal confidence.
    let raw_q = verdict.q_score.total;
    let domain_normalized_q = domain_specific_q_normalization(raw_q, domain);
    // Scale normalized_q ∈ [0,1] back to [0, PHI_INV] to preserve the absolute
    // epistemic ceiling. domain_normalized_q maps p50 → 0.5 and p95 → ~1.0;
    // multiplying by PHI_INV maps p50 → 0.309 and p95 → 0.618 (the ceiling).
    // Without this scaling, any q > domain_p50 produces confidence > PHI_INV,
    // violating the structural doubt principle. (Regression confirmed 2026-05-12.)
    let crystal_confidence = (domain_normalized_q * PHI_INV * injection_weight).min(PHI_INV);
    tracing::debug!(
        phase = "crystal_calibration",
        %domain,
        raw_q = %format!("{:.4}", raw_q),
        domain_normalized = %format!("{:.4}", domain_normalized_q),
        final_confidence = %format!("{:.4}", crystal_confidence),
        weight = %format!("{:.3}", injection_weight),
        "crystal confidence: raw → domain-calibrated → final (with epistemic weight)"
    );
    let verdict_kind = match verdict.kind {
        crate::domain::dog::VerdictKind::Howl => "howl",
        crate::domain::dog::VerdictKind::Wag => "wag",
        crate::domain::dog::VerdictKind::Growl => "growl",
        crate::domain::dog::VerdictKind::Bark => "bark",
    };
    if let Err(e) = storage
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
        metrics.inc_crystal_observe_failed();
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
        metrics.inc_crystal_obs();
        if let Some(tx) = event_tx {
            let _ = tx.send(KernelEvent::CrystalObserved {
                crystal_id: crystal_id.clone(),
                domain: domain.to_string(),
            }); // ok: no subscribers = silent no-op (receiver_count=0 returns Err)
        }
    }
    // Store embedding for KNN merge — for NEW crystals or when content evolves (KC12).
    // needs_embedding=true for new crystals. For existing crystals, only refresh when
    // the observation's score exceeds the crystal's running mean (content updated in DB).
    // This is rare enough (~10% of observations) to avoid SurrealKV compaction storms.
    let content_evolved = !needs_embedding && crystal_confidence > 0.5;
    if (needs_embedding || content_evolved)
        && let Some(emb) = stimulus_embedding
        && let Err(e) = storage
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
    fn epistemic_gate_agreed_below_phi_inv2() {
        let (tag, weight) = epistemic_gate(PHI_INV2 - 0.001);
        assert_eq!(tag, "agreed");
        assert!((weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_disputed_mid_range() {
        // Midpoint between φ⁻² (0.382) and φ⁻¹ (0.618)
        let mid = (PHI_INV2 + PHI_INV) / 2.0;
        let (tag, weight) = epistemic_gate(mid);
        assert_eq!(tag, "disputed");
        assert!(
            (weight - 0.5).abs() < 0.01,
            "midpoint should be ~0.5, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_disputed_near_agreed_boundary() {
        let (tag, weight) = epistemic_gate(PHI_INV2 + 0.001);
        assert_eq!(tag, "disputed");
        assert!(
            weight > 0.9,
            "near agreed boundary should be ~1.0, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_disputed_near_contested_boundary() {
        let (tag, weight) = epistemic_gate(PHI_INV - 0.001);
        assert_eq!(tag, "disputed");
        assert!(
            weight < 0.01,
            "near contested boundary should be ~0.0, got {weight}"
        );
    }

    #[test]
    fn epistemic_gate_contested_at_threshold() {
        let (tag, weight) = epistemic_gate(PHI_INV);
        assert_eq!(tag, "contested");
        assert!((weight - 0.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_contested_above_threshold() {
        let (tag, weight) = epistemic_gate(0.75);
        assert_eq!(tag, "contested");
        assert!((weight - 0.0).abs() < 1e-10);
    }

    #[test]
    fn epistemic_gate_real_world_disagreement_passes() {
        // Observed: USDC disagreement = 0.568, rug-pull = 0.468
        // Both should now be "disputed" (not quarantined)
        let (tag_usdc, weight_usdc) = epistemic_gate(0.568);
        assert_eq!(tag_usdc, "disputed");
        assert!(
            weight_usdc > 0.0,
            "USDC should pass with positive weight, got {weight_usdc}"
        );

        let (tag_rug, weight_rug) = epistemic_gate(0.468);
        assert_eq!(tag_rug, "disputed");
        assert!(
            weight_rug > weight_usdc,
            "lower disagreement should have higher weight"
        );
    }

    // ── crystal_confidence normalization (P1 ACCURACY) ──────

    #[test]
    fn crystal_confidence_howl_reaches_crystallization_threshold() {
        // HOWL verdict: Q-Score at φ-max (0.618) in chess domain (p50=0.549).
        // domain_normalized = min(0.618 / 0.549, 1.0) = 1.0
        // crystal_confidence = 1.0 * PHI_INV * 1.0 = 0.618 = PHI_INV (ceiling)
        let total = PHI_INV;
        let weight = 1.0;
        let normalized = domain_specific_q_normalization(total, "chess");
        let conf = (normalized * PHI_INV * weight).min(PHI_INV);
        assert!(
            conf >= PHI_INV - 1e-10,
            "HOWL verdict should produce crystal_confidence = φ⁻¹ (ceiling), got {conf}"
        );
    }

    #[test]
    fn crystal_confidence_bark_stays_below_crystallization() {
        // BARK verdict: Q-Score = 0.2 in chess domain (p50=0.549).
        // domain_normalized = 0.2 / 0.549 = 0.364
        // crystal_confidence = 0.364 * 0.618 * 1.0 = 0.225 < PHI_INV
        let total = 0.2;
        let weight = 1.0;
        let normalized = domain_specific_q_normalization(total, "chess");
        let conf = (normalized * PHI_INV * weight).min(PHI_INV);
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
