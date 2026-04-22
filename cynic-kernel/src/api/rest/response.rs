//! REST API response mapping — verdict-to-JSON and shared error helpers.

use axum::{http::StatusCode, response::Json};

use super::types::*;
use crate::domain::dog::{DogScore, PHI_INV, Verdict};

/// 500 + "storage unavailable" — used by any handler that fails on StoragePort.
pub fn storage_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "storage unavailable".into(),
        }),
    )
}

/// 500 + "coordination unavailable" — used by any handler that fails on CoordPort.
pub fn coordination_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "coordination unavailable".into(),
        }),
    )
}

pub fn verdict_to_response(v: &Verdict) -> JudgeResponse {
    JudgeResponse {
        verdict_id: v.id.clone(),
        domain: v.domain.clone(),
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
        timestamp: v.timestamp.clone(),
        dog_scores: v.dog_scores.iter().map(dog_score_to_response).collect(),
        voter_count: v.voter_count,
        anomaly_detected: v.anomaly_detected,
        max_disagreement: v.max_disagreement,
        anomaly_axiom: v.anomaly_axiom.clone(),
        integrity_hash: v.integrity_hash.clone(),
        prev_hash: v.prev_hash.clone(),
        cache_hit: None,
        token_data: None,
        stimulus_content: None,
    }
}

/// Wrap a verdict response with cache hit metadata.
pub fn verdict_response_cached(v: &Verdict, similarity: f64) -> JudgeResponse {
    let mut resp = verdict_to_response(v);
    resp.cache_hit = Some(similarity);
    resp
}

pub fn dog_score_to_response(ds: &DogScore) -> DogScoreResponse {
    DogScoreResponse {
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
        raw_fidelity: ds.raw_fidelity,
        raw_phi: ds.raw_phi,
        raw_verify: ds.raw_verify,
        raw_culture: ds.raw_culture,
        raw_burn: ds.raw_burn,
        raw_sovereignty: ds.raw_sovereignty,
        reasoning: ReasoningResponse {
            fidelity: ds.reasoning.fidelity.clone(),
            phi: ds.reasoning.phi.clone(),
            verify: ds.reasoning.verify.clone(),
            culture: ds.reasoning.culture.clone(),
            burn: ds.reasoning.burn.clone(),
            sovereignty: ds.reasoning.sovereignty.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::*;

    #[test]
    fn verdict_to_response_maps_all_fields() {
        let verdict = Verdict {
            id: "test-id".into(),
            domain: "chess".into(),
            kind: VerdictKind::Howl,
            q_score: QScore {
                total: 0.55,
                fidelity: 0.6,
                phi: 0.5,
                verify: 0.55,
                culture: 0.5,
                burn: 0.45,
                sovereignty: 0.5,
            },
            reasoning: AxiomReasoning {
                fidelity: "good".into(),
                phi: "ok".into(),
                verify: "decent".into(),
                culture: "fine".into(),
                burn: "lean".into(),
                sovereignty: "free".into(),
            },
            dog_id: "test-dog".into(),
            stimulus_summary: "test stimulus".into(),
            timestamp: "2026-03-15T00:00:00Z".into(),
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            voter_count: 0,
            failed_dogs: Vec::new(),
            failed_dog_errors: Default::default(),
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
        assert_eq!(resp.timestamp, "2026-03-15T00:00:00Z");
        assert!(!resp.anomaly_detected);
    }
}
