//! Temporal evaluation — 7 perspectives, single Dog call, JSON response.
//!
//! Evaluate a stimulus through 7 temporal lenses (Past, Present, Future, Cycle, Trend, Emergence, Transcendence).
//! Currently uses hardcoded heuristic responses to test aggregation logic.
//! TODO: Wire Judge.evaluate_temporal() when Dog multi-perspective inference is available.

use crate::domain::dog::AxiomScores;
use crate::domain::temporal::{
    TemporalPerspective, TemporalScore, TemporalVerdict, aggregate_temporal,
};
use serde::{Deserialize, Serialize};

/// Dog response: 7 perspectives, each with axiom scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TemporalDogResponse {
    pub perspectives: Vec<TemporalDogScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TemporalDogScore {
    pub perspective: String, // "PAST", "PRESENT", etc.
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

/// Evaluate stimulus through 7 temporal perspectives.
/// HARDCODED HEURISTIC: Returns fixed scores for testing temporal aggregation.
/// TODO: Replace with actual Dog call once multi-perspective prompting is available.
pub(super) fn evaluate_temporal(_content: &str) -> Result<TemporalVerdict, String> {
    // Heuristic response: 6 perspectives agree at 0.7, EMERGENCE diverges at 0.4
    let response = TemporalDogResponse {
        perspectives: vec![
            TemporalDogScore {
                perspective: "PAST".to_string(),
                fidelity: 0.8,
                phi: 0.7,
                verify: 0.8,
                culture: 0.8,
                burn: 0.6,
                sovereignty: 0.9,
            },
            TemporalDogScore {
                perspective: "PRESENT".to_string(),
                fidelity: 0.7,
                phi: 0.8,
                verify: 0.8,
                culture: 0.7,
                burn: 0.7,
                sovereignty: 0.8,
            },
            TemporalDogScore {
                perspective: "FUTURE".to_string(),
                fidelity: 0.6,
                phi: 0.5,
                verify: 0.6,
                culture: 0.6,
                burn: 0.8,
                sovereignty: 0.7,
            },
            TemporalDogScore {
                perspective: "CYCLE".to_string(),
                fidelity: 0.7,
                phi: 0.6,
                verify: 0.7,
                culture: 0.7,
                burn: 0.6,
                sovereignty: 0.8,
            },
            TemporalDogScore {
                perspective: "TREND".to_string(),
                fidelity: 0.7,
                phi: 0.6,
                verify: 0.7,
                culture: 0.7,
                burn: 0.6,
                sovereignty: 0.8,
            },
            TemporalDogScore {
                perspective: "EMERGENCE".to_string(),
                fidelity: 0.3,
                phi: 0.2,
                verify: 0.3,
                culture: 0.3,
                burn: 0.9,
                sovereignty: 0.2,
            },
            TemporalDogScore {
                perspective: "TRANSCENDENCE".to_string(),
                fidelity: 0.8,
                phi: 0.75,
                verify: 0.8,
                culture: 0.8,
                burn: 0.5,
                sovereignty: 0.9,
            },
        ],
    };

    let dog_response =
        serde_json::to_string(&response).map_err(|e| format!("serialization failed: {e}"))?;

    // Parse JSON response
    let temporal_response: TemporalDogResponse = serde_json::from_str(&dog_response)
        .map_err(|e| format!("failed to parse temporal response JSON: {e}"))?;

    // Convert Dog response to TemporalScores
    let mut temporal_scores = Vec::new();
    for dog_score in temporal_response.perspectives {
        let perspective = match dog_score.perspective.as_str() {
            "PAST" => TemporalPerspective::Past,
            "PRESENT" => TemporalPerspective::Present,
            "FUTURE" => TemporalPerspective::Future,
            "CYCLE" => TemporalPerspective::Cycle,
            "TREND" => TemporalPerspective::Trend,
            "EMERGENCE" => TemporalPerspective::Emergence,
            "TRANSCENDENCE" => TemporalPerspective::Transcendence,
            other => return Err(format!("unknown perspective: {other}")),
        };

        let axiom_scores = AxiomScores {
            fidelity: dog_score.fidelity.clamp(0.0, 1.0),
            phi: dog_score.phi.clamp(0.0, 1.0),
            verify: dog_score.verify.clamp(0.0, 1.0),
            culture: dog_score.culture.clamp(0.0, 1.0),
            burn: dog_score.burn.clamp(0.0, 1.0),
            sovereignty: dog_score.sovereignty.clamp(0.0, 1.0),
            ..Default::default()
        };

        let q_total = crate::domain::dog::compute_qscore(&axiom_scores).total;
        temporal_scores.push(TemporalScore {
            perspective,
            axiom_scores,
            q_total,
        });
    }

    if temporal_scores.len() != 7 {
        return Err(format!(
            "expected 7 perspectives, got {}",
            temporal_scores.len()
        ));
    }

    Ok(aggregate_temporal(&temporal_scores))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_temporal_response() {
        let json = r#"{
  "perspectives": [
    {"perspective": "PAST", "fidelity": 0.8, "phi": 0.6, "verify": 0.7, "culture": 0.8, "burn": 0.5, "sovereignty": 0.9},
    {"perspective": "PRESENT", "fidelity": 0.7, "phi": 0.7, "verify": 0.8, "culture": 0.7, "burn": 0.6, "sovereignty": 0.8},
    {"perspective": "FUTURE", "fidelity": 0.6, "phi": 0.5, "verify": 0.6, "culture": 0.6, "burn": 0.7, "sovereignty": 0.7},
    {"perspective": "CYCLE", "fidelity": 0.75, "phi": 0.65, "verify": 0.75, "culture": 0.75, "burn": 0.55, "sovereignty": 0.85},
    {"perspective": "TREND", "fidelity": 0.7, "phi": 0.6, "verify": 0.7, "culture": 0.7, "burn": 0.6, "sovereignty": 0.8},
    {"perspective": "EMERGENCE", "fidelity": 0.5, "phi": 0.4, "verify": 0.5, "culture": 0.5, "burn": 0.8, "sovereignty": 0.6},
    {"perspective": "TRANSCENDENCE", "fidelity": 0.85, "phi": 0.75, "verify": 0.85, "culture": 0.8, "burn": 0.4, "sovereignty": 0.95}
  ]
}"#;

        let resp: TemporalDogResponse = serde_json::from_str(json).expect("parse");
        assert_eq!(resp.perspectives.len(), 7);
        assert_eq!(resp.perspectives[0].perspective, "PAST");
        assert!((resp.perspectives[0].fidelity - 0.8).abs() < 0.001);
    }
}
