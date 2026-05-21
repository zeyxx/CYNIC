//! Pipeline evaluation stages — Dog selection, judgment, and post-verdict gates.

use crate::domain::dog::{
    DogScore, Stimulus, Verdict, VerdictKind, compute_qscore, phi_bound, verdict_kind,
};
use crate::domain::events::KernelEvent;
use crate::judge::{Judge, JudgeError};

use super::{PipelineDeps, PipelineResult, SOVEREIGN_DOMAINS};

/// Build a deterministic wallet verdict without invoking LLM Dogs.
/// Returns `Some(PipelineResult)` if domain is wallet-judgment and profile is available.
pub(super) async fn wallet_judgment_fast_path(
    stimulus: &Stimulus,
    profile: Option<&crate::domain::wallet_judgment::WalletProfile>,
    domain_hint: &str,
    stimulus_embedding: &Option<crate::domain::embedding::Embedding>,
    deps: &PipelineDeps<'_>,
) -> Result<Option<PipelineResult>, JudgeError> {
    if domain_hint != "wallet-judgment" {
        return Ok(None);
    }

    let Some(profile) = profile else {
        return Err(JudgeError::InvalidInput(
            "wallet-judgment requires valid WalletProfile JSON in context field".to_string(),
        ));
    };

    let (_, axiom_scores) = crate::domain::wallet_judgment::deterministic_dog(profile);
    let q_score = compute_qscore(&axiom_scores);
    let kind = verdict_kind(q_score.total);
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let stimulus_summary: String = stimulus.content.chars().take(300).collect();
    let dog_score = DogScore {
        dog_id: "wallet-deterministic-dog".to_string(),
        latency_ms: 0,
        prompt_tokens: 0,
        completion_tokens: 0,
        fidelity: phi_bound(axiom_scores.fidelity),
        phi: phi_bound(axiom_scores.phi),
        verify: phi_bound(axiom_scores.verify),
        culture: phi_bound(axiom_scores.culture),
        burn: phi_bound(axiom_scores.burn),
        sovereignty: phi_bound(axiom_scores.sovereignty),
        raw_fidelity: axiom_scores.fidelity,
        raw_phi: axiom_scores.phi,
        raw_verify: axiom_scores.verify,
        raw_culture: axiom_scores.culture,
        raw_burn: axiom_scores.burn,
        raw_sovereignty: axiom_scores.sovereignty,
        reasoning: axiom_scores.reasoning.clone(),
        reasoning_trace: None,
        abstentions: axiom_scores.abstentions.clone(),
    };
    let verdict = Verdict {
        id,
        domain: "wallet-judgment".to_string(),
        kind,
        q_score,
        reasoning: axiom_scores.reasoning,
        dog_id: "wallet-deterministic-dog".to_string(),
        stimulus_summary,
        timestamp,
        voter_count: 1,
        dog_scores: vec![dog_score],
        anomaly_detected: false,
        max_disagreement: 0.0,
        anomaly_axiom: None,
        failed_dogs: vec![],
        failed_dog_errors: std::collections::BTreeMap::new(),
        integrity_hash: None,
        prev_hash: None,
    };

    tracing::info!(
        phase = "verdict",
        verdict_id = %verdict.id,
        q_score = %format!("{:.3}", verdict.q_score.total),
        kind = ?verdict.kind,
        "wallet verdict issued (deterministic)"
    );

    emit_verdict_event(&verdict, "wallet-judgment", deps);

    super::side_effects::run(stimulus, &verdict, stimulus_embedding, deps, false).await;

    Ok(Some(PipelineResult::Evaluated {
        verdict: Box::new(verdict),
        token_data: None,
        enriched_content: None,
    }))
}

/// Resolved dog filter: either an owned vec (from router/sovereign) or borrowed (from caller).
pub(super) type DogFilter<'a> = (Option<Vec<String>>, Option<&'a [String]>);

/// Select Dogs based on domain constraints, caller filter, and data-driven routing.
/// Returns the final filter to pass to `evaluate_progressive`.
pub(super) fn select_dogs<'a>(
    domain_hint: &str,
    dogs_filter: Option<&'a [String]>,
    judge: &Judge,
    deps: &PipelineDeps<'_>,
) -> Result<DogFilter<'a>, JudgeError> {
    let domain_requires_sovereign = SOVEREIGN_DOMAINS.contains(&domain_hint);

    if domain_requires_sovereign {
        let sovereign = judge.sovereign_dog_ids();
        if sovereign.is_empty() {
            return Err(JudgeError::InvalidInput(
                "Sensitive domain requires sovereign backend but none available".into(),
            ));
        }
        tracing::info!(
            domain = domain_hint,
            "Layer 3 (domain constraint) fired — routing to sovereign Dogs only"
        );
        return Ok((Some(sovereign), None));
    }

    if dogs_filter.is_some() {
        return Ok((None, dogs_filter));
    }

    let mut from_router = deps.domain_router.map(|r| r.dogs_for_domain(domain_hint));

    // K15 consumer: refine by observed per-domain reliability.
    // RoutingCalculator tracks success_rate per (domain, dog_id). If data exists and
    // some Dogs are unreliable (<95% success, >=10 samples), exclude them.
    if let Some(ref mut router_dogs) = from_router
        && let Some(routing_calc) = deps.routing_calc
        && let Some(reliable) = routing_calc.reliable_dogs(domain_hint)
        && !reliable.is_empty()
    {
        let before = router_dogs.len();
        router_dogs.retain(|d| reliable.contains(d));
        let excluded = before - router_dogs.len();
        if excluded > 0 {
            tracing::info!(
                domain = domain_hint,
                excluded_count = excluded,
                reliable_count = router_dogs.len(),
                "RoutingCalculator excluded unreliable Dogs for this domain"
            );
        }
    }

    if let Some(router_dogs) = from_router {
        Ok((Some(router_dogs), None))
    } else {
        Ok((None, dogs_filter))
    }
}

/// K14 jury gate: downgrade verdict when Dogs < expected (missing = degraded).
pub(super) fn jury_gate(
    verdict: &mut Verdict,
    dogs_filter: Option<&[String]>,
    deps: &PipelineDeps<'_>,
) {
    let expected_dogs = dogs_filter
        .map(|f| f.len())
        .unwrap_or(deps.expected_dog_count);
    let actual_dogs = verdict.dog_scores.len();
    if actual_dogs < expected_dogs {
        let downgraded = match verdict.kind {
            VerdictKind::Howl => VerdictKind::Wag,
            VerdictKind::Wag => VerdictKind::Growl,
            VerdictKind::Growl => VerdictKind::Bark,
            VerdictKind::Bark => VerdictKind::Bark,
        };
        tracing::warn!(
            phase = "jury_gate",
            expected = expected_dogs,
            actual = actual_dogs,
            missing = expected_dogs - actual_dogs,
            original_kind = ?verdict.kind,
            downgraded_kind = ?downgraded,
            "jury incomplete — downgrading verdict per K14 (missing = degraded)"
        );
        verdict.kind = downgraded;
    }
}

/// Emit verdict + dog failure events to the event bus (best-effort).
pub(super) fn emit_verdict_event(verdict: &Verdict, domain: &str, deps: &PipelineDeps<'_>) {
    let Some(tx) = deps.event_tx else { return };

    let receivers = tx.receiver_count();
    match tx.send(KernelEvent::VerdictIssued {
        verdict_id: verdict.id.clone(),
        domain: domain.to_string(),
        verdict: format!("{:?}", verdict.kind),
        q_score: verdict.q_score.total,
    }) {
        Ok(n) => tracing::info!(phase = "event_bus", receivers = n, "verdict event sent"),
        Err(_) => tracing::debug!(
            phase = "event_bus",
            receivers = receivers,
            "no SSE subscribers"
        ),
    }

    for dog_id in &verdict.failed_dogs {
        let error_detail = verdict
            .failed_dog_errors
            .get(dog_id)
            .cloned()
            .unwrap_or_else(|| "evaluation_failed".into());
        if tx
            .send(KernelEvent::DogFailed {
                dog_id: dog_id.clone(),
                error: error_detail,
            })
            .is_err()
        {
            tracing::debug!(phase = "event_bus", dog_id = %dog_id, "no SSE subscribers for DogFailed");
        }
    }
}
