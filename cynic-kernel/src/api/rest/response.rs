//! REST API response mapping — verdict-to-JSON and temporal aggregation.

use super::types::*;
use crate::domain::dog::{Verdict, PHI_INV};

pub fn verdict_to_response(v: &Verdict) -> JudgeResponse {
    JudgeResponse {
        verdict_id: v.id.clone(),
        verdict: format!("{:?}", v.kind),
        q_score: QScoreResponse {
            total: v.q_score.total,
            fidelity: v.q_score.fidelity,
            phi: v.q_score.phi,
            verify: v.q_score.verify,
            culture: v.q_score.culture,
            burn: v.q_score.burn,
            sovereignty: v.q_score.sovereignty,
        },
        reasoning: ReasoningResponse {
            fidelity: v.reasoning.fidelity.clone(),
            phi: v.reasoning.phi.clone(),
            verify: v.reasoning.verify.clone(),
            culture: v.reasoning.culture.clone(),
            burn: v.reasoning.burn.clone(),
            sovereignty: v.reasoning.sovereignty.clone(),
        },
        dogs_used: v.dog_id.clone(),
        phi_max: PHI_INV,
        dog_scores: v.dog_scores.iter().map(|ds| DogScoreResponse {
            dog_id: ds.dog_id.clone(),
            latency_ms: ds.latency_ms,
            prompt_tokens: ds.prompt_tokens,
            completion_tokens: ds.completion_tokens,
            fidelity: ds.fidelity,
            phi: ds.phi,
            verify: ds.verify,
            culture: ds.culture,
            burn: ds.burn,
            sovereignty: ds.sovereignty,
            reasoning: ReasoningResponse {
                fidelity: ds.reasoning.fidelity.clone(),
                phi: ds.reasoning.phi.clone(),
                verify: ds.reasoning.verify.clone(),
                culture: ds.reasoning.culture.clone(),
                burn: ds.reasoning.burn.clone(),
                sovereignty: ds.reasoning.sovereignty.clone(),
            },
        }).collect(),
        anomaly_detected: v.anomaly_detected,
        max_disagreement: v.max_disagreement,
        anomaly_axiom: v.anomaly_axiom.clone(),
        temporal: compute_temporal_from_dogs(&v.dog_scores),
        integrity_hash: v.integrity_hash.clone(),
        prev_hash: v.prev_hash.clone(),
    }
}

/// Map Dog evaluations onto temporal perspectives and aggregate.
/// Each Dog represents a different "temporal lens" on the stimulus.
pub fn compute_temporal_from_dogs(dog_scores: &[crate::domain::dog::DogScore]) -> Option<TemporalResponse> {
    use crate::domain::temporal::{TemporalPerspective, TemporalScore, aggregate_temporal};
    use crate::domain::dog::compute_qscore;

    if dog_scores.len() < 2 {
        return None; // Need multiple perspectives
    }

    let perspectives = TemporalPerspective::ALL;

    let temporal_scores: Vec<(TemporalScore, String)> = dog_scores.iter().enumerate().filter_map(|(i, ds)| {
        let perspective = perspectives.get(i % perspectives.len())?;

        let axiom_scores = crate::domain::dog::AxiomScores {
            fidelity: ds.fidelity, phi: ds.phi, verify: ds.verify,
            culture: ds.culture, burn: ds.burn, sovereignty: ds.sovereignty,
            reasoning: crate::domain::dog::AxiomReasoning::default(),
            ..Default::default()
        };
        let q = compute_qscore(&axiom_scores);
        Some((TemporalScore { perspective: *perspective, axiom_scores, q_total: q.total }, ds.dog_id.clone()))
    }).collect();

    if temporal_scores.is_empty() {
        return None;
    }

    let scores_only: Vec<TemporalScore> = temporal_scores.iter().map(|(s, _)| s.clone()).collect();
    let tv = aggregate_temporal(&scores_only);

    Some(TemporalResponse {
        temporal_total: tv.temporal_total,
        outlier_perspective: tv.outlier_perspective.map(|p| p.label().to_string()),
        max_divergence: tv.max_divergence,
        perspectives: temporal_scores.iter().map(|(ts, dog_id)| {
            TemporalPerspectiveScore {
                perspective: ts.perspective.label().to_string(),
                q_total: ts.q_total,
                dog_id: dog_id.clone(),
            }
        }).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::*;

    #[test]
    fn verdict_to_response_maps_all_fields() {
        let verdict = Verdict {
            id: "test-id".into(),
            kind: VerdictKind::Howl,
            q_score: QScore {
                total: 0.55, fidelity: 0.6, phi: 0.5,
                verify: 0.55, culture: 0.5, burn: 0.45, sovereignty: 0.5,
            },
            reasoning: AxiomReasoning {
                fidelity: "good".into(), phi: "ok".into(), verify: "decent".into(),
                culture: "fine".into(), burn: "lean".into(), sovereignty: "free".into(),
            },
            dog_id: "test-dog".into(),
            stimulus_summary: "test stimulus".into(),
            timestamp: "2026-03-15T00:00:00Z".into(),
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            integrity_hash: Some("abc123".into()),
            prev_hash: None,
        };

        let resp = verdict_to_response(&verdict);
        assert_eq!(resp.verdict_id, "test-id");
        assert_eq!(resp.verdict, "Howl");
        assert_eq!(resp.q_score.total, 0.55);
        assert_eq!(resp.reasoning.fidelity, "good");
        assert_eq!(resp.dogs_used, "test-dog");
        assert_eq!(resp.phi_max, PHI_INV);
        assert!(!resp.anomaly_detected);
    }

    #[test]
    fn temporal_returns_none_with_single_dog() {
        let scores = vec![DogScore {
            dog_id: "deterministic-dog".into(),
            latency_ms: 0, prompt_tokens: 0, completion_tokens: 0,
            fidelity: 0.5, phi: 0.5, verify: 0.5,
            culture: 0.5, burn: 0.5, sovereignty: 0.5,
            reasoning: AxiomReasoning::default(),
        }];
        assert!(compute_temporal_from_dogs(&scores).is_none());
    }

    #[test]
    fn temporal_returns_some_with_multiple_dogs() {
        let scores = vec![
            DogScore {
                dog_id: "deterministic-dog".into(),
                latency_ms: 0, prompt_tokens: 0, completion_tokens: 0,
                fidelity: 0.5, phi: 0.5, verify: 0.5,
                culture: 0.5, burn: 0.5, sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
            },
            DogScore {
                dog_id: "gemini".into(),
                latency_ms: 100, prompt_tokens: 50, completion_tokens: 30,
                fidelity: 0.6, phi: 0.55, verify: 0.5,
                culture: 0.45, burn: 0.4, sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
            },
        ];
        let temporal = compute_temporal_from_dogs(&scores);
        assert!(temporal.is_some());
        let t = temporal.unwrap();
        assert!(t.temporal_total > 0.0);
        assert!(t.temporal_total <= PHI_INV + 1e-10);
    }
}
